//! Temporary, user-specific raw-HID access via the desktop's authentication agent.

use anyhow::{bail, ensure, Context, Result};
use hidapi::HidApi;
use std::{
    fs::{self, OpenOptions},
    io::ErrorKind,
    os::unix::{ffi::OsStrExt, fs::FileTypeExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::hardware::{PRODUCT_ID_WIRED, PRODUCT_ID_WIRELESS, VENDOR_ID_WIRED, VENDOR_ID_WIRELESS};

pub(crate) fn is_control_collection(vid: u16, pid: u16, page: u16, usage: u16) -> bool {
    (vid, pid, page, usage) == (VENDOR_ID_WIRELESS, PRODUCT_ID_WIRELESS, 0xff02, 2)
        || (vid, pid, page, usage) == (VENDOR_ID_WIRED, PRODUCT_ID_WIRED, 0xff00, 1)
}

/// Grant this user access to inaccessible F75 control nodes, never the whole GUI.
/// No device settings are sent; connection must still validate identity/profile.
pub(crate) fn request_device_access() -> Result<()> {
    let api = HidApi::new().context("Failed to discover F75 control interfaces")?;
    let mut found = false;
    let mut paths = Vec::new();
    for info in api.device_list().filter(|info| {
        is_control_collection(
            info.vendor_id(),
            info.product_id(),
            info.usage_page(),
            info.usage(),
        )
    }) {
        found = true;
        let path = PathBuf::from(std::ffi::OsStr::from_bytes(info.path().to_bytes()));
        ensure!(
            fs::symlink_metadata(&path)?.file_type().is_char_device(),
            "Control node is not a character device: {}",
            path.display()
        );
        match OpenOptions::new().read(true).write(true).open(&path) {
            Ok(_) => {} // Already accessible: no authentication needed.
            Err(error) if error.kind() == ErrorKind::PermissionDenied => paths.push(path),
            Err(error) => return Err(error).context("Could not open F75 control node"),
        }
    }
    ensure!(
        found,
        "No supported F75 control interface found. Plug in USB or the 2.4 GHz receiver."
    );
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return Ok(());
    }

    // Fixed system locations, not a user-controlled PATH search for a root command.
    let pkexec = ["/run/wrappers/bin/pkexec", "/usr/bin/pkexec", "/bin/pkexec"]
        .into_iter()
        .find(|path| Path::new(path).is_file())
        .context(
            "Admin connection needs polkit (pkexec) and a running desktop authentication agent",
        )?;
    let setfacl = [
        "/run/current-system/sw/bin/setfacl",
        "/usr/bin/setfacl",
        "/bin/setfacl",
    ]
    .into_iter()
    .find(|path| Path::new(path).is_file())
    .context("Admin connection needs setfacl from the acl package")?;
    let status = fs::read_to_string("/proc/self/status")?;
    let uid = status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|values| values.split_whitespace().next())
        .context("Could not determine the current user's UID")?
        .parse::<u32>()?;
    let mut command = access_command(pkexec, setfacl, uid, &paths)?;
    run_access_command(&mut command)?;
    for path in paths {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .with_context(|| {
                format!(
                    "Access still denied or device unplugged after authentication: {}",
                    path.display()
                )
            })?;
    }
    Ok(())
}

fn access_command(pkexec: &str, setfacl: &str, uid: u32, paths: &[PathBuf]) -> Result<Command> {
    ensure!(!paths.is_empty(), "No control nodes to grant access to");
    for path in paths {
        let name = path
            .to_str()
            .and_then(|path| path.strip_prefix("/dev/hidraw"));
        ensure!(
            name.is_some_and(
                |suffix| !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
            ),
            "Refusing a non-hidraw permission target"
        );
    }
    let mut command = Command::new(pkexec);
    command
        .args([
            "--disable-internal-agent",
            setfacl,
            "--modify",
            &format!("u:{uid}:rw"),
            "--",
        ])
        .args(paths)
        // nix develop can set SHELL to a store path absent from /etc/shells;
        // pkexec rejects that environment before it can show authentication.
        .env_remove("SHELL")
        .stdin(Stdio::null());
    Ok(command)
}

fn run_access_command(command: &mut Command) -> Result<()> {
    let output = command
        .output()
        .context("Could not start the system authentication prompt")?;
    match output.status.code() {
        Some(0) => Ok(()),
        Some(126) => bail!("Admin authentication cancelled; device not connected"),
        Some(127) => bail!(
            "Admin authentication failed. Ensure a polkit authentication agent is running: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        _ => bail!(
            "Device access grant failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, process::Command};

    #[test]
    fn admin_command_grants_only_the_named_user_and_control_node() {
        let command = access_command(
            "/usr/bin/pkexec",
            "/usr/bin/setfacl",
            1000,
            &[PathBuf::from("/dev/hidraw6")],
        )
        .unwrap();
        assert_eq!(command.get_program(), "/usr/bin/pkexec");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                "--disable-internal-agent",
                "/usr/bin/setfacl",
                "--modify",
                "u:1000:rw",
                "--",
                "/dev/hidraw6"
            ]
        );
    }

    #[test]
    fn admin_command_does_not_inherit_a_nix_development_shell() {
        let command = access_command(
            "/usr/bin/pkexec",
            "/usr/bin/setfacl",
            1000,
            &[PathBuf::from("/dev/hidraw6")],
        )
        .unwrap();
        assert!(command
            .get_envs()
            .any(|(key, value)| key == "SHELL" && value.is_none()));
    }

    #[test]
    fn admin_command_rejects_non_hidraw_paths_and_empty_targets() {
        for paths in [
            vec![],
            vec![PathBuf::from("/dev/hidraw*")],
            vec![PathBuf::from("/dev/hidraw")],
            vec![PathBuf::from("/dev/../etc/passwd")],
            vec![PathBuf::from("/dev/hidraw6/../hidraw7")],
            vec![PathBuf::from("/dev/input/event0")],
            vec![PathBuf::from("-R")],
        ] {
            assert!(access_command("/usr/bin/pkexec", "/usr/bin/setfacl", 1000, &paths).is_err());
        }
    }

    #[test]
    fn cancelled_authentication_is_not_reported_as_a_grant() {
        let mut command = Command::new("sh");
        command.args(["-c", "exit 126"]);
        let error = run_access_command(&mut command).unwrap_err();
        assert!(error.to_string().contains("cancelled"));
    }

    #[test]
    fn failed_permission_command_is_not_reported_as_a_grant() {
        let mut command = Command::new("sh");
        command.args(["-c", "exit 1"]);
        assert!(run_access_command(&mut command).is_err());
    }

    #[test]
    fn successful_permission_command_is_accepted() {
        let mut command = Command::new("sh");
        command.args(["-c", "exit 0"]);
        run_access_command(&mut command).unwrap();
    }

    #[test]
    fn control_collection_filter_excludes_keyboard_and_other_devices() {
        assert!(is_control_collection(0x3554, 0xfa09, 0xff02, 2));
        assert!(is_control_collection(0x258a, 0x010c, 0xff00, 1));
        assert!(!is_control_collection(0x3554, 0xfa09, 1, 6));
        assert!(!is_control_collection(0x258a, 0x010c, 0xff02, 2));
        assert!(!is_control_collection(0x249a, 0x5c2f, 0xff02, 2));
        assert!(!is_control_collection(0x3554, 0xfa09, 0, 0));
    }
}
