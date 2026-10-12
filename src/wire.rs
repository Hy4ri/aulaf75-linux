//! HID wire protocol encoding, decoding, and checksum verification for AULA F75.
//!
//! Recovered from OemDrv.exe reverse engineering evidence:
//! - Wireless transport framing & checksum: 0x486c80, 0x486500, 0x486f70, 0x486260
//! - Lighting query (GetLED opcode 0x44): 0x4868f0
//! - Profile write (SetLED opcode 0x04): 0x4868a0, 0x486260
//! - Status / Identity queries (commands 7, 5): 0x486f70, 0x485e90, 0x40c530

use anyhow::{bail, ensure, Result};

pub const WIRELESS_REPORT_ID: u8 = 0x13;
pub const WIRELESS_PACKET_LEN: usize = 20;
pub const WIRELESS_PAYLOAD_MAX: usize = 14;

pub const WIRED_REPORT_ID_FEATURE: u8 = 0x06;
pub const WIRED_FEATURE_LEN: usize = 520;
pub const WIRED_CMD_GET_LED: u8 = 0x84;
pub const WIRED_CMD_SET_LED: u8 = 0x04;
pub const WIRED_CMD_GET_ID: u8 = 0x82;

/// Recovered read commands supported across wire transports.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ReadCommand {
    Online,
    Identity,
    Lighting,
}

impl ReadCommand {
    /// Wireless command byte (opcode).
    pub fn opcode(self) -> u8 {
        match self {
            ReadCommand::Online => 7,
            ReadCommand::Identity => 5,
            ReadCommand::Lighting => 0x44,
        }
    }
}

/// Decoded chunk from a wireless read response packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub count: u8,
    pub index: u8,
    pub payload: Vec<u8>,
}

/// Calculate the 1-byte wrapping additive checksum over preceding bytes.
///
/// Recovered from FUN_00486c80: sum of all preceding bytes (modulo 256).
pub fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// Validate a write acknowledgement, not a read data chunk.
/// Original helper0x485f20 (assembly0x485f41..0x485f8e) checks command at1 and
/// error bit7 at2. It does not interpret count/index as read-chunk metadata.
/// Our parser additionally requires the transport report ID and checksum.
/// Physical keyboard compatibility remains a separate hardware gate.
pub fn validate_ack(packet: &[u8], command: u8) -> Result<()> {
    ensure!(
        matches!(command, 1..=4 | 9),
        "Unverified acknowledgement command"
    );
    ensure!(
        packet.len() == WIRELESS_PACKET_LEN,
        "Acknowledgement length mismatch"
    );
    ensure!(
        packet[0] == WIRELESS_REPORT_ID,
        "Acknowledgement report ID mismatch"
    );
    ensure!(packet[1] == command, "Acknowledgement command mismatch");
    ensure!(
        packet[2] & 0x80 == 0,
        "Device rejected packet: CRC/status error"
    );
    ensure!(
        packet[19] == checksum(&packet[..19]),
        "Acknowledgement checksum mismatch"
    );
    Ok(())
}

/// Generate a 20-byte wireless read request report.
///
/// Format per FUN_00486500 / FUN_00486f70:
/// - Byte 0: Report ID (0x13)
/// - Byte 1: Command opcode (e.g. 7, 5, 0x44)
/// - Byte 2: Count / subflag (1)
/// - Bytes 3..19: zeros
/// - Byte 19: Checksum (wrapping sum of bytes 0..18)
pub fn read_request(cmd: ReadCommand) -> [u8; WIRELESS_PACKET_LEN] {
    let mut req = [0u8; WIRELESS_PACKET_LEN];
    req[0] = WIRELESS_REPORT_ID;
    req[1] = cmd.opcode();
    req[2] = 1;
    // req[3..19] remain 0
    req[19] = checksum(&req[..19]);
    req
}

/// Decode a wireless read response packet.
///
/// Validates:
/// - Length exactly 20 bytes
/// - Report ID is 0x13
/// - Byte 1 matches the requested command opcode (defensive check of MSB error bit)
/// - Byte 2 MSB (error flag from operational helper 0x485f20) is not set
/// - Byte 19 matches wrapping additive checksum of bytes 0..19
/// - Payload length from lower nibble of byte 4 (clamped to max 14)
/// - Count > 0 and index < count (strictly 0-based index below positive count)
pub fn decode_packet(packet: &[u8], expected_cmd: ReadCommand) -> Result<Chunk> {
    ensure!(
        packet.len() == WIRELESS_PACKET_LEN,
        "Packet length must be exactly {}, got {}",
        WIRELESS_PACKET_LEN,
        packet.len()
    );

    ensure!(
        packet[0] == WIRELESS_REPORT_ID,
        "Invalid report ID {:#04x}, expected {:#04x}",
        packet[0],
        WIRELESS_REPORT_ID
    );

    // Checksum verification
    let expected_csum = checksum(&packet[..19]);
    ensure!(
        packet[19] == expected_csum,
        "Checksum mismatch: computed {:#04x}, packet has {:#04x}",
        expected_csum,
        packet[19]
    );

    // Command verification: byte 1
    // Discovery helper 0x486f70 checks byte 1's MSB: (packet[1] & 0x80) == 0.
    // Operational helper 0x485f20 checks byte 2's MSB: (packet[2] & 0x80) == 0.
    ensure!(
        (packet[1] & 0x80) == 0,
        "Error bit set in response command byte {:#04x}",
        packet[1]
    );
    ensure!(
        (packet[2] & 0x80) == 0,
        "Error bit set in response status byte {:#04x}",
        packet[2]
    );

    let opcode = packet[1];
    ensure!(
        opcode == expected_cmd.opcode(),
        "Opcode mismatch: expected {:#04x} ({:?}), got {:#04x}",
        expected_cmd.opcode(),
        expected_cmd,
        opcode
    );

    let count = packet[2];
    let index = packet[3];

    ensure!(count > 0, "Packet count must be > 0, got 0");
    ensure!(
        index < count,
        "Packet index {} must be strictly less than count {}",
        index,
        count
    );

    // Payload size is in the lower nibble of byte 4
    let length = (packet[4] & 0x0f) as usize;
    ensure!(
        length <= WIRELESS_PAYLOAD_MAX,
        "Invalid payload length {} > max 14 in byte 4 ({:#04x})",
        length,
        packet[4]
    );

    let payload = packet[5..5 + length].to_vec();

    Ok(Chunk {
        count,
        index,
        payload,
    })
}

/// Encode data into 20-byte wireless write packets.
///
/// Recovered from FUN_00486260:
/// - Only recovered write opcodes: 1, 2, 3, 4, 9.
/// - Reject opcode 6 (reset), empty data, subindex > 15, overflowing counts.
/// - Each packet has 14 bytes of payload max.
/// - Byte 0: 0x13
/// - Byte 1: command opcode
/// - Byte 2: total packets count
/// - Byte 3: 0-based chunk index
/// - Byte 4: (subindex << 4) | chunk_payload_length
/// - Bytes 5..19: payload padded with 0
/// - Byte 19: wrapping checksum of bytes 0..18
pub fn encode_write(
    command: u8,
    subindex: u8,
    data: &[u8],
) -> Result<Vec<[u8; WIRELESS_PACKET_LEN]>> {
    // Whitelist only recovered opcodes: 1, 2, 3, 4, 9
    match command {
        1 | 2 | 3 | 4 | 9 => (),
        6 => bail!("Command 6 (reset) is strictly rejected to protect device state"),
        other => bail!(
            "Command {:#04x} is unverified or unsupported for hardware write",
            other
        ),
    }

    ensure!(!data.is_empty(), "Cannot encode empty write data");
    ensure!(
        subindex <= 15,
        "Subindex {} exceeds 4-bit field limit (15)",
        subindex
    );

    let total_len = data.len();
    let total_chunks = total_len.div_ceil(WIRELESS_PAYLOAD_MAX);
    ensure!(
        total_chunks <= 127,
        "Data length {} requires {} chunks, which exceeds maximum chunk count 127",
        total_len,
        total_chunks
    );
    let count = total_chunks as u8;

    let mut packets = Vec::with_capacity(total_chunks);

    for (chunk_idx, chunk_slice) in data.chunks(WIRELESS_PAYLOAD_MAX).enumerate() {
        let mut pkt = [0u8; WIRELESS_PACKET_LEN];
        pkt[0] = WIRELESS_REPORT_ID;
        pkt[1] = command;
        pkt[2] = count;
        pkt[3] = chunk_idx as u8;
        pkt[4] = (subindex << 4) | (chunk_slice.len() as u8);
        pkt[5..5 + chunk_slice.len()].copy_from_slice(chunk_slice);
        pkt[19] = checksum(&pkt[..19]);
        packets.push(pkt);
    }

    Ok(packets)
}

/// Create a wired 520-byte feature report request for GetLED (84).
///
/// Header: [6, 84, 0, 0, 1, 0, 128, 0] + 512 zeros.
pub fn wired_read_lighting_request() -> [u8; WIRED_FEATURE_LEN] {
    let mut buf = [0u8; WIRED_FEATURE_LEN];
    buf[0] = WIRED_REPORT_ID_FEATURE;
    buf[1] = WIRED_CMD_GET_LED;
    buf[2] = 0;
    buf[3] = 0;
    buf[4] = 1;
    buf[5] = 0;
    buf[6] = 128; // size low byte (0x80 = 128)
    buf[7] = 0; // size high byte
    buf
}

/// Create a wired 520-byte feature report request for identity query [6, 82, 1, 0, 1, 0, 6, 0] + 512 zeros.
pub fn wired_read_identity_request() -> [u8; WIRED_FEATURE_LEN] {
    let mut buf = [0u8; WIRED_FEATURE_LEN];
    buf[0] = WIRED_REPORT_ID_FEATURE;
    buf[1] = WIRED_CMD_GET_ID;
    buf[2] = 1;
    buf[3] = 0;
    buf[4] = 1;
    buf[5] = 0;
    buf[6] = 6; // 6 bytes identity
    buf[7] = 0;
    buf
}

/// Create a wired 520-byte feature report for SetLED (04).
///
/// Header: [6, 4, 0, 0, 1, 0, 128, 0] + 128 bytes profile + trailing zeros.
pub fn wired_write_lighting_request(profile: &[u8; 128]) -> [u8; WIRED_FEATURE_LEN] {
    let mut buf = [0u8; WIRED_FEATURE_LEN];
    buf[0] = WIRED_REPORT_ID_FEATURE;
    buf[1] = WIRED_CMD_SET_LED;
    buf[2] = 0;
    buf[3] = 0;
    buf[4] = 1;
    buf[5] = 0;
    buf[6] = 128;
    buf[7] = 0;
    buf[8..8 + 128].copy_from_slice(profile);
    buf
}
