# aulaf75-linux

An in-development Rust/GPUI port of the AULA F75 Windows configuration app.

## Current status

This is **not yet a working replacement** for the original app.

- The GPUI window builds and opens.
- HID enumeration can find the receiver. Opening a handle does not establish that the keyboard is online, has the expected identity, or accepts configuration commands.
- The existing effect tiles have no event handlers and their names/IDs are not trustworthy. They are an early UI shell, not implemented lighting control.
- Hardware-setting writes are blocked in `AulaDevice::set_rgb_mode` until the recovered F75-specific protocol has been validated.
- Lighting, key remapping, macros, profiles, sleep and debounce controls are not implemented in the UI yet.

## Reverse-engineering evidence

The source artifact is `AULA F75 Setup v2.0 20240509.exe`. Its extracted `OemDrv.exe` is being inspected statically through REA 6.4.0 and Ghidra 12.1.2. The original program has not been executed during this investigation.

See `docs/re/README.md` for observations, unknowns, and verification boundaries. Local raw Evidence is retained under `docs/re/evidence/` (ignored by Git); `docs/re/evidence/index.json` links operation names to exact Evidence IDs and source hashes.

A successful write system call is not evidence that a setting was applied. Any hardware implementation must validate identity, responses and profile data, preserve fields it does not understand, and report timeout, rejection and permission errors honestly. Firmware flashing and factory reset are outside the current implementation scope.

## Development

```sh
nix develop
cargo run
```

Or run `nix develop --command cargo run` from this directory.

Tests: `nix develop --command cargo test`.

Linux raw-HID access requires an appropriate device-specific permission grant. Do not make every `/dev/hidraw*` world-writable. For a temporary diagnostic grant, identify the current exact control node first, then grant only the intended user access; temporary permissions may disappear when the receiver reconnects.

### Connect with an administrator prompt

Click **Connect (admin prompt)** in the app footer if normal inspection reports a permissions error. The desktop's polkit password dialog (`pkexec`) grants your user read/write access only to the supported F75 control collection(s), using `setfacl`. The GUI stays non-root, never receives your password, and does not install a permanent udev rule or make other HID devices world-writable. Already-accessible devices do not need a prompt.

After approval, the app reopens the device and validates its identity and lighting profile before reporting a connection. No lighting settings are applied by connecting. Cancelling authentication or a failed grant leaves the device disconnected and reports the error. The grant lasts until the device node is removed (usually unplug/replug or reboot); closing the app does not revoke it.

This option requires `pkexec` (polkit), a running desktop polkit authentication agent, and `setfacl` (the `acl` package). On NixOS these must be available in the system environment; on other Linux distributions they are looked up in `/usr/bin` or `/bin`. A missing helper/agent is reported rather than falling back to running the whole app with sudo.
