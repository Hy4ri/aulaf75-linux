//! Hardware backend for AULA F75 keyboard communication.
//!
//! Handles interface discovery, connection validation, read-modify-write-readback
//! of lighting profile, preserving unedited bytes and returning descriptive errors.
//!
//! Follows contract /home/m57/.hermes/cache/scratch/aula-port-contract.md:
//! - Never report connection success from open descriptor alone.
//! - Strict identity validation against Catalogue.
//! - Bounded timeouts on reads/writes.
//! - No guessed writes or raw reset/macro/remap encoding.

use anyhow::{bail, ensure, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::time::{Duration, Instant};

use crate::model::{Catalogue, Profile};
use crate::wire::{
    decode_packet, encode_write, read_request, validate_ack, wired_read_identity_request,
    wired_read_lighting_request, wired_write_lighting_request, ReadCommand, WIRED_FEATURE_LEN,
    WIRELESS_PACKET_LEN,
};

pub const VENDOR_ID_WIRELESS: u16 = 0x3554;
pub const PRODUCT_ID_WIRELESS: u16 = 0xFA09;

pub const VENDOR_ID_WIRED: u16 = 0x258A;
pub const PRODUCT_ID_WIRED: u16 = 0x010C;

const READ_TIMEOUT_MS: i32 = 1000;
const DURATION_READ_TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTransport {
    Wireless24G,
    Wired,
}

pub struct Hardware {
    device: HidDevice,
    transport: DeviceTransport,
    identity: [u8; 6],
    cached_profile: [u8; 128],
    description_str: String,
}

impl Hardware {
    /// Ask the desktop authentication agent for user-specific device access,
    /// then perform the same identity/profile checks as a normal connection.
    pub fn connect_with_admin_prompt(cat: &Catalogue) -> Result<Self> {
        crate::permissions::request_device_access()?;
        Self::connect(cat)
    }

    /// Attempt to discover and connect to a supported AULA F75 device.
    ///
    /// Verifies device accessibility, validates online status and 6-byte identity
    /// against the provided Catalogue, and reads the real 128-byte device profile.
    /// Never returns success from an open descriptor alone.
    pub fn connect(cat: &Catalogue) -> Result<Self> {
        let api = HidApi::new().context("Failed to initialize HID API backend")?;

        let mut errors = Vec::new();

        // 1. Try Wireless Dongle (PID 3554:fa09)
        // Control collection: usage_page 0xff02, usage 0x0002
        for dev_info in api.device_list() {
            if dev_info.vendor_id() == VENDOR_ID_WIRELESS
                && dev_info.product_id() == PRODUCT_ID_WIRELESS
            {
                let matches_collection = crate::permissions::is_control_collection(
                    dev_info.vendor_id(),
                    dev_info.product_id(),
                    dev_info.usage_page(),
                    dev_info.usage(),
                );

                if matches_collection {
                    match Self::try_connect_wireless(&api, dev_info, cat) {
                        Ok(hw) => return Ok(hw),
                        Err(e) => {
                            let path = dev_info.path().to_string_lossy().into_owned();
                            errors.push(format!("Wireless candidate at {path}: {e}"));
                        }
                    }
                }
            }
        }

        // 2. Try Wired USB (PID 258a:010c)
        // Control collection: usage_page 0xff00, usage 0x0001
        for dev_info in api.device_list() {
            if dev_info.vendor_id() == VENDOR_ID_WIRED && dev_info.product_id() == PRODUCT_ID_WIRED
            {
                let matches_collection = crate::permissions::is_control_collection(
                    dev_info.vendor_id(),
                    dev_info.product_id(),
                    dev_info.usage_page(),
                    dev_info.usage(),
                );

                if matches_collection {
                    match Self::try_connect_wired(&api, dev_info, cat) {
                        Ok(hw) => return Ok(hw),
                        Err(e) => {
                            let path = dev_info.path().to_string_lossy().into_owned();
                            errors.push(format!("Wired candidate at {path}: {e}"));
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            bail!("No supported AULA F75 device found on USB/wireless bus (VID:PID 3554:fa09 or 258a:010c)");
        } else {
            bail!(
                "Found candidate AULA devices but connection or validation failed:\n{}",
                errors.join("\n")
            );
        }
    }

    fn try_connect_wireless(
        api: &HidApi,
        dev_info: &hidapi::DeviceInfo,
        cat: &Catalogue,
    ) -> Result<Self> {
        let path = dev_info.path();
        let handle = dev_info.open_device(api).with_context(|| {
            format!(
                "Failed to open wireless HID handle at {:?}. Check udev permissions (/dev/hidraw*)",
                path
            )
        })?;

        // Drain any stale reports before querying
        drain_incoming(&handle)?;

        // Send Online query (command 7)
        let online_req = read_request(ReadCommand::Online);
        handle
            .write(&online_req)
            .context("Failed to send wireless Online query (cmd 7)")?;

        // Read and decode Online status response
        let online_chunk = read_single_chunk(&handle, ReadCommand::Online)?;
        // Online query status1 is distinct from event0x0a type marker2.
        ensure!(
            online_chunk.count == 1 && online_chunk.index == 0,
            "Wireless Online response chunk count/index mismatch: count={}, index={}",
            online_chunk.count,
            online_chunk.index
        );
        ensure!(
            online_chunk.payload.len() == 1,
            "Wireless Online response payload length must be exactly 1, got {}",
            online_chunk.payload.len()
        );
        ensure!(
            online_chunk.payload[0] == 1,
            "Wireless Online response indicates device is offline (status byte: {:#04x})",
            online_chunk.payload[0]
        );

        // Send Identity query (command 5)
        let id_req = read_request(ReadCommand::Identity);
        handle
            .write(&id_req)
            .context("Failed to send wireless Identity query (cmd 5)")?;

        let id_chunk = read_single_chunk(&handle, ReadCommand::Identity)?;
        let dev_identity = validate_wireless_identity(&id_chunk, cat.identity)?;

        // Read lighting profile (128 bytes)
        let profile_bytes = Self::read_wireless_lighting_profile(&handle)?;

        let desc = format!(
            "AULA F75 [Wireless 2.4GHz] (ID: {:02x?}) via {:?}",
            dev_identity, path
        );

        Ok(Self {
            device: handle,
            transport: DeviceTransport::Wireless24G,
            identity: dev_identity,
            cached_profile: profile_bytes,
            description_str: desc,
        })
    }

    fn try_connect_wired(
        api: &HidApi,
        dev_info: &hidapi::DeviceInfo,
        cat: &Catalogue,
    ) -> Result<Self> {
        let path = dev_info.path();
        let handle = dev_info.open_device(api).with_context(|| {
            format!(
                "Failed to open wired HID handle at {:?}. Check udev permissions (/dev/hidraw*)",
                path
            )
        })?;

        // Query identity feature report [6, 82, 1, 0, 1, 0, 6, 0]
        let id_req = wired_read_identity_request();
        let mut id_resp = [0u8; WIRED_FEATURE_LEN];
        id_resp[0] = 6;

        handle
            .send_feature_report(&id_req)
            .context("Failed to send wired identity feature request (cmd 0x82)")?;

        let read_len = handle
            .get_feature_report(&mut id_resp)
            .context("Failed to get wired identity feature report")?;
        ensure!(
            read_len >= 14,
            "Wired identity response too short: got {} bytes, expected at least 14",
            read_len
        );
        ensure!(
            id_resp[0] == 6,
            "Wired identity response report ID mismatch: expected 6, got {}",
            id_resp[0]
        );
        ensure!(
            id_resp[1] == crate::wire::WIRED_CMD_GET_ID,
            "Wired identity response opcode mismatch: expected {:#04x}, got {:#04x}",
            crate::wire::WIRED_CMD_GET_ID,
            id_resp[1]
        );
        ensure!(
            id_resp[6] == 6 && id_resp[7] == 0,
            "Wired identity response length header mismatch: expected 6, got {}",
            u16::from_le_bytes([id_resp[6], id_resp[7]])
        );

        let mut dev_id = [0u8; 6];
        dev_id.copy_from_slice(&id_resp[8..14]);
        ensure!(
            dev_id == cat.identity,
            "Wired device identity mismatch: device returned {:02x?}, catalogue expected {:02x?}",
            dev_id,
            cat.identity
        );

        let profile = Self::read_wired_lighting_profile(&handle)?;
        let desc = format!("AULA F75 [Wired USB] (ID: {:02x?}) via {:?}", dev_id, path);
        Ok(Self {
            device: handle,
            transport: DeviceTransport::Wired,
            identity: dev_id,
            cached_profile: profile,
            description_str: desc,
        })
    }

    fn read_wireless_lighting_profile(handle: &HidDevice) -> Result<[u8; 128]> {
        drain_incoming(handle)?;

        let req = read_request(ReadCommand::Lighting);
        handle
            .write(&req)
            .context("Failed to send wireless Lighting read request (cmd 0x44)")?;

        let mut collected = Vec::with_capacity(128);
        let start = Instant::now();

        // Lighting read requires exactly 10 chunks: indices 0..=9, count 10,
        // chunk lengths: 14 bytes for first 9 chunks (indices 0..8) and 2 bytes for last chunk (index 9).
        let mut buf = [0u8; WIRELESS_PACKET_LEN];
        let mut expected_index = 0u8;
        let mut ignored_async_count = 0usize;
        const MAX_IGNORED_ASYNC: usize = 64;

        while expected_index < 10 {
            if start.elapsed() > DURATION_READ_TIMEOUT {
                bail!(
                    "Timed out reading wireless lighting profile (collected {} / 10 chunks, {} bytes)",
                    expected_index,
                    collected.len()
                );
            }

            let n = handle.read_timeout(&mut buf, READ_TIMEOUT_MS)?;
            if n == 0 {
                continue;
            }
            if n < WIRELESS_PACKET_LEN {
                bail!(
                    "Received runt wireless packet (length {}, expected {})",
                    n,
                    WIRELESS_PACKET_LEN
                );
            }

            // If packet matches wireless report ID 0x13 and lighting command 0x44, strictly decode it
            if buf[0] == crate::wire::WIRELESS_REPORT_ID && buf[1] == ReadCommand::Lighting.opcode()
            {
                let chunk = decode_packet(&buf, ReadCommand::Lighting)
                    .context("Malformed lighting response chunk frame")?;

                ensure!(
                    chunk.count == 10,
                    "Lighting profile chunk count mismatch: expected 10, got {}",
                    chunk.count
                );
                ensure!(
                    chunk.index == expected_index,
                    "Lighting profile chunk sequence mismatch: expected index {}, got {}",
                    expected_index,
                    chunk.index
                );

                let expected_len = if expected_index < 9 { 14 } else { 2 };
                ensure!(
                    chunk.payload.len() == expected_len,
                    "Lighting profile chunk {} payload length mismatch: expected {}, got {}",
                    expected_index,
                    expected_len,
                    chunk.payload.len()
                );

                collected.extend_from_slice(&chunk.payload);
                expected_index += 1;
            } else {
                // Non-matching asynchronous notification frame
                ignored_async_count += 1;
                ensure!(
                    ignored_async_count <= MAX_IGNORED_ASYNC,
                    "Exceeded maximum ignored asynchronous packets ({}) while waiting for lighting chunk {}",
                    MAX_IGNORED_ASYNC,
                    expected_index
                );
            }
        }

        ensure!(
            collected.len() == 128,
            "Received profile incomplete: {} bytes, expected exactly 128",
            collected.len()
        );

        let mut profile = [0u8; 128];
        profile.copy_from_slice(&collected);

        ensure!(
            profile[126] == 0x5a && profile[127] == 0xa5,
            "Profile verification failed: trailer magic bytes are {:#04x} {:#04x}, expected 0x5a 0xa5",
            profile[126],
            profile[127]
        );

        Ok(profile)
    }

    fn read_wired_lighting_profile(handle: &HidDevice) -> Result<[u8; 128]> {
        let req = wired_read_lighting_request();
        handle
            .send_feature_report(&req)
            .context("Failed to send wired GetLED feature request (report 6, cmd 84)")?;

        let mut resp = [0u8; WIRED_FEATURE_LEN];
        resp[0] = 6;
        let read_len = handle
            .get_feature_report(&mut resp)
            .context("Failed to get wired GetLED feature response")?;

        ensure!(
            read_len >= 8 + 128,
            "Wired GetLED response too short: got {} bytes, expected at least 136",
            read_len
        );
        ensure!(
            resp[0] == 6,
            "Wired GetLED response report ID mismatch: expected 6, got {}",
            resp[0]
        );
        ensure!(
            resp[1] == crate::wire::WIRED_CMD_GET_LED,
            "Wired GetLED response opcode mismatch: expected {:#04x}, got {:#04x}",
            crate::wire::WIRED_CMD_GET_LED,
            resp[1]
        );
        ensure!(
            resp[6] == 128 && resp[7] == 0,
            "Wired GetLED response payload size header mismatch: expected 128, got {}",
            u16::from_le_bytes([resp[6], resp[7]])
        );

        let mut profile = [0u8; 128];
        profile.copy_from_slice(&resp[8..8 + 128]);

        ensure!(
            profile[126] == 0x5a && profile[127] == 0xa5,
            "Wired profile validation failed: trailer bytes are {:#04x} {:#04x}, expected 0x5a 0xa5",
            profile[126],
            profile[127]
        );

        Ok(profile)
    }

    /// Read live lighting profile from the device.
    pub fn read_lighting(&self) -> Result<[u8; 128]> {
        match self.transport {
            DeviceTransport::Wireless24G => Self::read_wireless_lighting_profile(&self.device),
            DeviceTransport::Wired => Self::read_wired_lighting_profile(&self.device),
        }
    }

    /// Apply lighting settings with read-modify-write-readback verification:
    /// 1. Reads current profile from device to ensure freshest base.
    /// 2. Patches only permitted lighting bytes via Profile::patch_lighting.
    /// 3. Writes patched profile to device.
    /// 4. Validates transfer/acknowledgement.
    /// 5. Reads back device profile and verifies all target changes match exactly.
    pub fn apply_lighting(&mut self, profile: &Profile, cat: &Catalogue) -> Result<[u8; 128]> {
        // Step 1: Read live original profile
        let original = self
            .read_lighting()
            .context("Failed to read live device profile for baseline")?;

        // Step 2: Patch lighting parameters
        let patched = profile
            .patch_lighting(&original, cat)
            .context("Failed to patch lighting parameters")?;

        // Step 3 & 4: Write to device
        match self.transport {
            DeviceTransport::Wireless24G => {
                let packets = encode_write(4, 0, &patched)?;
                drain_incoming(&self.device)?;
                for (index, packet) in packets.iter().enumerate() {
                    let sent = self
                        .device
                        .write(packet)
                        .with_context(|| format!("Profile packet {index} send failed"))?;
                    ensure!(
                        sent == WIRELESS_PACKET_LEN,
                        "Partial wireless profile packet write"
                    );
                    // Stop on uncertain acknowledgement; never blindly retry a setting write.
                    await_ack(&self.device, 4).with_context(|| format!("Profile packet {index} was not acknowledged; setting not claimed applied"))?;
                }
            }
            DeviceTransport::Wired => {
                let req = wired_write_lighting_request(&patched);
                self.device
                    .send_feature_report(&req)
                    .context("Failed to send wired SetLED feature report (cmd 04)")?;
            }
        }

        // Step 5: Readback verification
        let readback = self
            .read_lighting()
            .context("Failed to read back device profile after write")?;

        // Verify ALL 128 bytes match the patched buffer exactly to detect collateral/unexpected changes
        ensure!(
            readback == patched,
            "Readback verification failed: full 128-byte profile mismatch (unexpected changes detected)"
        );

        self.cached_profile = readback;
        Ok(readback)
    }

    /// Access currently cached 128-byte profile without re-reading hardware.
    pub fn profile_bytes(&self) -> [u8; 128] {
        self.cached_profile
    }

    /// Human-readable device description and connection status.
    pub fn description(&self) -> String {
        self.description_str.clone()
    }

    /// Active connection transport.
    pub fn transport(&self) -> DeviceTransport {
        self.transport
    }

    /// Verified 6-byte hardware identity.
    pub fn identity(&self) -> [u8; 6] {
        self.identity
    }
}

fn validate_wireless_identity(chunk: &crate::wire::Chunk, expected: [u8; 6]) -> Result<[u8; 6]> {
    ensure!(
        chunk.count == 1 && chunk.index == 0,
        "Wireless Identity response chunk count/index mismatch: count={}, index={}",
        chunk.count,
        chunk.index
    );
    // Original discovery helper 0x486f70 (0x4870c3..0x4870ed) copies
    // min(reported payload length, requested length). The live receiver returns
    // ten bytes; its first six are the model ID, the trailing fields stay opaque.
    ensure!(
        (6..=crate::wire::WIRELESS_PAYLOAD_MAX).contains(&chunk.payload.len()),
        "Wireless Identity response must contain at least 6 identity bytes and at most 14 payload bytes, got {}",
        chunk.payload.len()
    );
    let mut identity = [0u8; 6];
    identity.copy_from_slice(&chunk.payload[..6]);
    ensure!(
        identity == expected,
        "Device identity mismatch: device returned {:02x?}, catalogue expected {:02x?}",
        identity,
        expected
    );
    Ok(identity)
}

fn drain_incoming(device: &HidDevice) -> Result<()> {
    let mut buf = [0u8; 64];
    let start = Instant::now();
    let timeout = Duration::from_millis(100);
    const MAX_DRAIN_COUNT: usize = 128;
    let mut drained = 0;

    while start.elapsed() < timeout {
        match device.read_timeout(&mut buf, 5) {
            Ok(0) => return Ok(()),
            Ok(_) => {
                drained += 1;
                ensure!(
                    drained <= MAX_DRAIN_COUNT,
                    "Device incoming pipe is continuously chattering; cannot drain stale reports (drained > {} packets)",
                    MAX_DRAIN_COUNT
                );
            }
            Err(e) => {
                bail!("Failed draining incoming HID packets: {}", e);
            }
        }
    }

    bail!("Incoming report queue did not drain before its deadline")
}

fn await_ack(device: &HidDevice, command: u8) -> Result<()> {
    let deadline = Instant::now() + DURATION_READ_TIMEOUT;
    let mut packet = [0u8; WIRELESS_PACKET_LEN];
    while Instant::now() < deadline {
        let remaining = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .clamp(1, 1000) as i32;
        let n = device.read_timeout(&mut packet, remaining)?;
        if n == 0 {
            continue;
        }
        ensure!(n == WIRELESS_PACKET_LEN, "Runt acknowledgement report");
        if packet[1] == command {
            return validate_ack(&packet, command);
        }
    }
    bail!("Timed out waiting for device acknowledgement of command {command:#04x}")
}

fn read_single_chunk(device: &HidDevice, expected_cmd: ReadCommand) -> Result<crate::wire::Chunk> {
    let mut buf = [0u8; WIRELESS_PACKET_LEN];
    let start = Instant::now();
    let mut ignored_async_count = 0usize;
    const MAX_IGNORED_ASYNC: usize = 64;

    while start.elapsed() < DURATION_READ_TIMEOUT {
        let n = device.read_timeout(&mut buf, READ_TIMEOUT_MS)?;
        if n == 0 {
            continue;
        }
        if n < WIRELESS_PACKET_LEN {
            bail!(
                "Received runt wireless packet (length {}, expected {})",
                n,
                WIRELESS_PACKET_LEN
            );
        }

        if buf[0] == crate::wire::WIRELESS_REPORT_ID && buf[1] == expected_cmd.opcode() {
            return decode_packet(&buf, expected_cmd)
                .context("Malformed response packet for command");
        } else {
            ignored_async_count += 1;
            ensure!(
                ignored_async_count <= MAX_IGNORED_ASYNC,
                "Exceeded maximum ignored asynchronous packets ({}) while waiting for {:?}",
                MAX_IGNORED_ASYNC,
                expected_cmd
            );
        }
    }

    bail!("Timed out waiting for valid response to {:?}", expected_cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Actual /dev/hidraw6 reply to the recovered command-5 read query.
    const CAPTURED_IDENTITY: [u8; 20] = [
        0x13, 0x05, 0x01, 0x00, 0x0a, 0x03, 0x00, 0x00, 0x00, 0x00, 0xcd, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0xf4,
    ];

    #[test]
    fn wireless_identity_accepts_captured_extended_reply_without_bypassing_model_check() {
        let catalogue = Catalogue::bundled().unwrap();
        let chunk = decode_packet(&CAPTURED_IDENTITY, ReadCommand::Identity).unwrap();
        assert_eq!(chunk.payload.len(), 10);
        assert_eq!(
            validate_wireless_identity(&chunk, catalogue.identity).unwrap(),
            catalogue.identity
        );
    }

    #[test]
    fn wireless_identity_still_accepts_an_exact_six_byte_reply() {
        // Synthesized six-byte variant of the captured reply.
        let mut packet = CAPTURED_IDENTITY;
        packet[4] = 6;
        packet[11..19].fill(0);
        packet[19] = crate::wire::checksum(&packet[..19]);
        let chunk = decode_packet(&packet, ReadCommand::Identity).unwrap();
        let expected = Catalogue::bundled().unwrap().identity;
        assert_eq!(
            validate_wireless_identity(&chunk, expected).unwrap(),
            expected
        );
    }

    #[test]
    fn wireless_identity_rejects_a_different_model_in_an_extended_reply() {
        let chunk = decode_packet(&CAPTURED_IDENTITY, ReadCommand::Identity).unwrap();
        let mut expected = Catalogue::bundled().unwrap().identity;
        expected[5] = 0xe1;
        assert!(validate_wireless_identity(&chunk, expected)
            .unwrap_err()
            .to_string()
            .contains("mismatch"));
    }

    #[test]
    fn wireless_identity_rejects_truncated_or_oversized_payloads() {
        let expected = Catalogue::bundled().unwrap().identity;
        for length in [0, 5, 15] {
            let mut chunk = decode_packet(&CAPTURED_IDENTITY, ReadCommand::Identity).unwrap();
            chunk.payload.resize(length, 0);
            assert!(validate_wireless_identity(&chunk, expected).is_err());
        }
    }

    #[test]
    fn wireless_identity_rejects_fragmented_or_out_of_sequence_replies() {
        let expected = Catalogue::bundled().unwrap().identity;
        for (count, index) in [(0, 0), (2, 0), (1, 1)] {
            let mut chunk = decode_packet(&CAPTURED_IDENTITY, ReadCommand::Identity).unwrap();
            chunk.count = count;
            chunk.index = index;
            assert!(validate_wireless_identity(&chunk, expected).is_err());
        }
    }

    #[test]
    fn wireless_identity_packet_still_requires_the_captured_checksum() {
        let mut packet = CAPTURED_IDENTITY;
        packet[5] ^= 1;
        assert!(decode_packet(&packet, ReadCommand::Identity).is_err());
    }
}
