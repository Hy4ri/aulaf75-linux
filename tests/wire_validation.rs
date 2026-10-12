use aulaf75_linux::wire::{decode_packet, encode_write, read_request, ReadCommand};

fn compute_checksum(packet: &[u8; 20]) -> u8 {
    packet[..19]
        .iter()
        .fold(0u8, |acc, byte| acc.wrapping_add(*byte))
}

fn make_synthetic_packet(
    report: u8,
    command: u8,
    count: u8,
    index: u8,
    len_or_subindex: u8,
    payload_data: &[u8],
) -> [u8; 20] {
    let mut packet = [0u8; 20];
    packet[0] = report;
    packet[1] = command;
    packet[2] = count;
    packet[3] = index;
    packet[4] = len_or_subindex;
    for (i, &b) in payload_data.iter().take(14).enumerate() {
        packet[5 + i] = b;
    }
    packet[19] = compute_checksum(&packet);
    packet
}

#[test]
fn read_request_known_opcodes_match_contract() {
    let lighting_req = read_request(ReadCommand::Lighting);
    assert_eq!(lighting_req.len(), 20);
    assert_eq!(lighting_req[0], 0x13); // report 13
    assert_eq!(lighting_req[1], 0x44); // opcode 0x44
    assert_eq!(lighting_req[2], 1); // count 1
    assert_eq!(lighting_req[3], 0); // index 0
    assert_eq!(lighting_req[19], compute_checksum(&lighting_req));

    let identity_req = read_request(ReadCommand::Identity);
    assert_eq!(identity_req.len(), 20);
    assert_eq!(identity_req[0], 0x13);
    assert_eq!(identity_req[1], 5); // opcode 5 for identity
    assert_eq!(identity_req[19], compute_checksum(&identity_req));

    let online_req = read_request(ReadCommand::Online);
    assert_eq!(online_req.len(), 20);
    assert_eq!(online_req[0], 0x13);
    assert_eq!(online_req[1], 7); // opcode 7 for online query
    assert_eq!(online_req[19], compute_checksum(&online_req));
}

#[test]
fn decode_packet_valid_synthetic_response() {
    // Contract prompt specification:
    // "A validresponse example reports count10,index0,length14 and zeros payload;
    // metadata checksum computed synthetically tests explicitlynot hardware captures."
    let synthetic = make_synthetic_packet(0x13, 0x44, 10, 0, 14, &[0u8; 14]);
    let chunk = decode_packet(&synthetic, ReadCommand::Lighting)
        .expect("valid synthetic lighting response should decode");

    assert_eq!(chunk.count, 10);
    assert_eq!(chunk.index, 0);
    assert_eq!(chunk.payload.len(), 14);
    assert_eq!(chunk.payload, vec![0u8; 14]);
}

#[test]
fn decode_packet_length_low_nibble_extraction() {
    // Contract: decoded payload length is the low nibble of byte 4, with 14 maximum.
    // If subindex or high bits are present in byte 4 (e.g. 0x24 -> subindex 2, length 4),
    // length low nibble 4 must extract 4 bytes.
    let payload = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
    let packet = make_synthetic_packet(0x13, 0x44, 1, 0, 0x24, &payload);
    let chunk = decode_packet(&packet, ReadCommand::Lighting)
        .expect("low nibble payload length decode should succeed");
    assert_eq!(chunk.payload.len(), 4);
    assert_eq!(chunk.payload, &[1, 2, 3, 4]);
}

#[test]
fn decode_packet_rejects_payload_length_greater_than_14() {
    // Low nibble 15 (> 14) is illegal payload length
    let packet = make_synthetic_packet(0x13, 0x44, 1, 0, 15, &[0u8; 14]);
    assert!(
        decode_packet(&packet, ReadCommand::Lighting).is_err(),
        "payload length nibble 15 > 14 must be rejected as illegal length"
    );
}

#[test]
fn decode_packet_rejects_short_frames() {
    let synthetic = make_synthetic_packet(0x13, 0x44, 10, 0, 14, &[0u8; 14]);
    for len in [0, 1, 4, 18, 19] {
        assert!(
            decode_packet(&synthetic[..len], ReadCommand::Lighting).is_err(),
            "short frame of length {len} must be rejected"
        );
    }
}

#[test]
fn decode_packet_rejects_wrong_report_or_command_opcode() {
    // Wrong report ID (not 0x13 / 13)
    let bad_report = make_synthetic_packet(0x12, 0x44, 1, 0, 14, &[0u8; 14]);
    assert!(
        decode_packet(&bad_report, ReadCommand::Lighting).is_err(),
        "wrong report ID must be rejected"
    );

    // Wrong opcode for expected ReadCommand::Lighting (expected 0x44)
    let bad_opcode = make_synthetic_packet(0x13, 0x45, 1, 0, 14, &[0u8; 14]);
    assert!(
        decode_packet(&bad_opcode, ReadCommand::Lighting).is_err(),
        "wrong opcode for lighting command must be rejected"
    );

    // Matching against wrong ReadCommand
    let lighting_packet = make_synthetic_packet(0x13, 0x44, 1, 0, 14, &[0u8; 14]);
    assert!(
        decode_packet(&lighting_packet, ReadCommand::Identity).is_err(),
        "lighting packet decoded as identity must fail opcode match"
    );
}

#[test]
fn decode_packet_rejects_checksum_mismatch() {
    let mut packet = make_synthetic_packet(0x13, 0x44, 10, 0, 14, &[0u8; 14]);
    packet[19] = packet[19].wrapping_add(1);
    assert!(
        decode_packet(&packet, ReadCommand::Lighting).is_err(),
        "packet with corrupted checksum byte must be rejected"
    );

    // Checksum altered on an internal payload byte
    let mut packet2 = make_synthetic_packet(0x13, 0x44, 10, 0, 14, &[0u8; 14]);
    packet2[5] ^= 0xaa;
    assert!(
        decode_packet(&packet2, ReadCommand::Lighting).is_err(),
        "payload modification without recomputed checksum must fail validation"
    );
}

#[test]
fn decode_packet_rejects_inconsistent_index_and_count() {
    // Index >= count (e.g. index 10 when count is 10)
    let packet_idx_ge_count = make_synthetic_packet(0x13, 0x44, 10, 10, 14, &[0u8; 14]);
    assert!(
        decode_packet(&packet_idx_ge_count, ReadCommand::Lighting).is_err(),
        "index >= count must be rejected"
    );

    // Index 1 when count is 1
    let packet_idx_ge_count_single = make_synthetic_packet(0x13, 0x44, 1, 1, 14, &[0u8; 14]);
    assert!(
        decode_packet(&packet_idx_ge_count_single, ReadCommand::Lighting).is_err(),
        "index == count in 1-packet response must be rejected"
    );

    // Count 0
    let packet_count_zero = make_synthetic_packet(0x13, 0x44, 0, 0, 14, &[0u8; 14]);
    assert!(
        decode_packet(&packet_count_zero, ReadCommand::Lighting).is_err(),
        "count == 0 must be rejected"
    );
}

#[test]
fn decode_packet_rejects_error_flags_or_unknown_response_codes() {
    // High bits or error indicators on opcode/report
    let err_packet = make_synthetic_packet(0x13, 0x44 | 0x80, 1, 0, 14, &[0u8; 14]);
    assert!(
        decode_packet(&err_packet, ReadCommand::Lighting).is_err(),
        "error flag in command opcode must be rejected"
    );
}

#[test]
fn encode_write_boundary_and_preservation_rules() {
    // Contract: only recovered opcodes 1, 2, 3, 4, 9 are allowed.
    for allowed_cmd in [1, 2, 3, 4, 9] {
        let single_byte = [0x42];
        let packets = encode_write(allowed_cmd, 0, &single_byte)
            .unwrap_or_else(|_| panic!("opcode {allowed_cmd} should be accepted"));
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0][0], 0x13);
        assert_eq!(packets[0][1], allowed_cmd);
        assert_eq!(packets[0][2], 1); // count 1
        assert_eq!(packets[0][3], 0); // index 0
        assert_eq!(packets[0][4] & 0x0f, 1); // low nibble length 1
        assert_eq!(packets[0][5], 0x42); // payload preserved
        assert_eq!(packets[0][19], compute_checksum(&packets[0]));
    }

    // Subindex in high nibble: e.g. subindex 5
    let packets = encode_write(4, 5, &[0x11, 0x22]).expect("subindex 5 should encode");
    assert_eq!(packets[0][4] >> 4, 5, "high nibble must contain subindex");
    assert_eq!(packets[0][4] & 0x0f, 2, "low nibble must contain length");

    // Exact 14-byte boundary: 1 packet
    let exact_14: Vec<u8> = (0..14).collect();
    let packets_14 = encode_write(4, 0, &exact_14).unwrap();
    assert_eq!(packets_14.len(), 1);
    assert_eq!(packets_14[0][2], 1);
    assert_eq!(packets_14[0][3], 0);
    assert_eq!(packets_14[0][4] & 0x0f, 14);
    assert_eq!(&packets_14[0][5..19], &exact_14[..]);

    // 15 bytes: splits into 2 packets (14 + 1)
    let bytes_15: Vec<u8> = (0..15).collect();
    let packets_15 = encode_write(4, 0, &bytes_15).unwrap();
    assert_eq!(packets_15.len(), 2);
    assert_eq!(packets_15[0][2], 2);
    assert_eq!(packets_15[0][3], 0);
    assert_eq!(packets_15[0][4] & 0x0f, 14);
    assert_eq!(&packets_15[0][5..19], &bytes_15[..14]);
    assert_eq!(packets_15[1][2], 2);
    assert_eq!(packets_15[1][3], 1);
    assert_eq!(packets_15[1][4] & 0x0f, 1);
    assert_eq!(packets_15[1][5], bytes_15[14]);

    // Trailing zero-padding up to 14 bytes must not corrupt checksum or data
    assert_eq!(packets_15[1][19], compute_checksum(&packets_15[1]));
}

#[test]
fn encode_write_rejection_rules() {
    // Subindex > 15 must be rejected
    assert!(
        encode_write(4, 16, &[1]).is_err(),
        "subindex 16 must be rejected"
    );
    assert!(
        encode_write(4, 255, &[1]).is_err(),
        "subindex 255 must be rejected"
    );

    // Empty data must be rejected
    assert!(
        encode_write(4, 0, &[]).is_err(),
        "empty data must be rejected"
    );

    // Reset opcode 6 must be rejected
    assert!(
        encode_write(6, 0, &[1]).is_err(),
        "opcode 6 must be rejected"
    );

    // Unknown opcodes (anything other than 1, 2, 3, 4, 9) must be rejected
    let invalid_opcodes = [0, 5, 7, 8, 10, 11, 0x44, 0xff];
    for op in invalid_opcodes {
        assert!(
            encode_write(op, 0, &[1]).is_err(),
            "unverified write opcode {op} must be rejected"
        );
    }
}
