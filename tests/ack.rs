use aulaf75_linux::wire::{checksum, validate_ack};
// Synthetic acknowledgement fixtures derived from original response helper0x485f20;
// these are parser tests, not captured hardware replies.
fn ack(command: u8, status: u8) -> [u8; 20] {
    let mut p = [0; 20];
    p[0] = 0x13;
    p[1] = command;
    p[2] = status;
    p[19] = checksum(&p[..19]);
    p
}
#[test]
fn ack_uses_command_and_error_flag_not_read_chunk_payload_format() {
    assert!(validate_ack(&ack(4, 0), 4).is_ok());
}
#[test]
fn ack_rejects_wrong_command_error_flag_checksum_and_size() {
    assert!(validate_ack(&ack(3, 0), 4).is_err());
    assert!(validate_ack(&ack(4, 0x80), 4).is_err());
    let mut bad = ack(4, 0);
    bad[19] ^= 1;
    assert!(validate_ack(&bad, 4).is_err());
    assert!(validate_ack(&ack(4, 0)[..19], 4).is_err());
}
