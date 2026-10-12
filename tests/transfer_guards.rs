use aulaf75_linux::wire::{checksum, decode_packet, ReadCommand};
fn fixture(count: u8, index: u8, len: u8) -> [u8; 20] {
    let mut p = [0; 20];
    p[0] = 0x13;
    p[1] = 0x44;
    p[2] = count;
    p[3] = index;
    p[4] = len;
    p[19] = checksum(&p[..19]);
    p
}
#[test]
fn read_chunk_zero_count_is_invalid() {
    assert!(decode_packet(&fixture(0, 0, 14), ReadCommand::Lighting).is_err());
}
#[test]
fn read_chunk_index_must_be_below_total() {
    assert!(decode_packet(&fixture(10, 10, 14), ReadCommand::Lighting).is_err());
    assert!(decode_packet(&fixture(1, 2, 14), ReadCommand::Lighting).is_err());
}
