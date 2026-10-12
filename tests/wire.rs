use aulaf75_linux::wire::{decode_packet, encode_write, read_request, ReadCommand};

#[test]
fn getled_query_matches_original_disassembly() {
    let request = read_request(ReadCommand::Lighting);
    assert_eq!(
        request,
        [0x13, 0x44, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x58]
    );
}

#[test]
fn write_profile_is_split_into_14_byte_chunks() {
    let data: Vec<u8> = (0..128).collect();
    let packets = encode_write(4, 0, &data).unwrap();
    assert_eq!(packets.len(), 10);
    assert_eq!(&packets[0][..5], &[0x13, 4, 10, 0, 14]);
    assert_eq!(&packets[9][..5], &[0x13, 4, 10, 9, 2]);
    let decoded: Vec<u8> = packets
        .iter()
        .flat_map(|p| p[5..5 + (p[4] & 15) as usize].to_vec())
        .collect();
    assert_eq!(decoded, data);
    for p in packets {
        assert_eq!(p[19], p[..19].iter().fold(0u8, |a, b| a.wrapping_add(*b)));
    }
}

#[test]
fn reset_and_unverified_commands_are_rejected() {
    assert!(encode_write(6, 0, &[1]).is_err());
    assert!(encode_write(0xff, 0, &[1]).is_err());
    assert!(encode_write(4, 16, &[1]).is_err());
    assert!(encode_write(4, 0, &[]).is_err());
}

#[test]
fn malformed_read_response_is_rejected() {
    let request = read_request(ReadCommand::Lighting);
    assert!(decode_packet(&request[..19], ReadCommand::Lighting).is_err());
    let mut corrupt = request;
    corrupt[19] ^= 1;
    assert!(decode_packet(&corrupt, ReadCommand::Lighting).is_err());
    assert!(decode_packet(&request, ReadCommand::Identity).is_err());
}
