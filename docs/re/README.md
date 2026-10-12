# AULA F75 reverse-engineering ledger

## Status and evidence boundary

This is an ongoing investigation, not a completed driver or feature-parity claim.
The original installer was unpacked with innoextract; native code was subsequently decompiled using REA 6.4.0 and Ghidra 12.1.2. The Windows target was not executed.

Analyzed payload: `OemDrv.exe`
SHA-256: `7c41a8af17b8d1577ac5660b8f0ce1be61da50fd33a5238158c62e6097292783`

Local raw results are in `evidence/`. The verified portable export is `evidence/complete-session.json` with 90 retained Evidence records, indexed by `evidence/index.json`. These local files are ignored by Git. Hypothesis-oriented filenames are not authoritative function names: use the returned function, requested address, assembly and Evidence metadata. In particular, some calls using an interior address resolved to the containing earlier function. Ghidra decompiler register-argument guesses must not be treated as original C++ signatures.

## Established observations

### Device and profile selection

- The three bundled configurations are standard F75, F75 KR and wired F75. All specify `Fw=24`, wired VID/PID `258a:010c`, `MatrixLen=128`, and `CRC=1`.
- Standard and KR profiles specify receiver VID/PID `3554:fa09`. Their six-byte `Psd` identifiers differ; the wired profile also has its own identifier. A matching VID/PID alone is not a matching keyboard identity.
- `0x4107b0` reads the profile's firmware-family and identity fields (Evidence `ev_93a7d5afe197fa5a84861992133536f2b4c699cbac14d666f177430aa8dd7d7a`).
- Wired discovery `0x48afa0` selects firmware family 24 through a vendor usage-page `ff00`, usage 1, feature-report length `0x208` (520 bytes); it also checks the six-byte identity (Evidence `ev_1bb9254e1566ca51c82882f316fab869fc3a1fa9f53b2e3cb86f61c745292ad5`).
- The attached receiver's input1 descriptor exposes report ID `0x13` with 19 payload bytes (20 including report ID), and feature report ID 6 with seven payload bytes. See `current-hid.json` for the actual descriptor and permissions.

### Wireless transport

- `0x486260` constructs 20-byte output reports, 14-byte payload chunks, a count, zero-based chunk index, sub-index/length byte and additive byte checksum. It has response and retry paths. A 19-byte write of arbitrary RGB fields is not this operation.
- `0x486f70` and its assembly establish discovery queries; the original watcher calls it with report ID `0x13`, command 7, expected length 1 for online state, and command 5, expected length 6 for identity. Evidence `ev_27bb01089588c0abadb46cf10770928194c7bd9bfca02abc7491125a18834b5d` and `ev_a890ae017b740e3edd65e25c51108eaee200316d14fb8dd3c3eb2757949baae5`. The watcher bytes not assigned to a Ghidra function are preserved in `evidence/keyboard-watcher-raw-assembly.txt`; that is a separate radare2 assembly observation.
- `0x4868f0` calls the wireless read operation with command `0x44` for GetLED (Evidence `ev_a84cbb4f12a7cb94f82ffc06f9d8d50683649404a35a511d9fc6030aceba755f`). The receive operation is `0x486500` (Evidence `ev_1812b3055f106ca9ed789f0f595bd01ef4dfb252ab4b4ad184f56855d2e14a4d`).
- Recovered setting operations include key-matrix write 1, custom-lighting-matrix write 2, macro write 3, lighting-profile write 4, and color-table write 9. Reset is command 6 and is explicitly excluded from experiments. These are static observations, not permission to issue raw commands or proof of device acceptance.

### Wired transport

- `0x48b350` uses a 520-byte feature report, header plus at most 512 payload bytes. For firmware family 24 the report ID resolves to 6, not the wireless report ID (Evidence `ev_839e5f970dc6ecf27138f936b71fcfa0ab79d3936e712ef813399b8b9797e3b9`).
- Wired GetLED `0x48ba80` uses command `0x84` and `HidD_SetFeature`/`HidD_GetFeature`; it delegates through a wireless backend when one is present (Evidence `ev_e15dce976dc5db311ad20e003d2b26a4f78fc81d8bcbc167ffbcb8a89e720cb1`). Do not confuse wired `0x84` with wireless `0x44`.

### Original feature/data model

- `profiles.json` contains the extracted options and key entries; each supplied profile has 81 `[KEY]` entries and an `[FN1]` section. `MatrixLen=128` does not mean the whole keymap is a 128-byte packet.
- `0x48d140` serializes four key layers, four bytes per matrix entry, with a separate macro buffer/index table (Evidence `ev_30fd1351643f2ca2ff8142b318bf179462f5a408ff8a6f37b5df3ae6bda30ea9`).
- `0x48da30` reads and preserves a 128-byte lighting/settings profile, checks its trailing `5a a5` signature, packs edits and invokes the transport. It also writes only dirty key layers and color groups (Evidence `ev_a5b57f58fa9fcc7eb70559d04e234d8a8a402e4c69f75e5d38feaf1e0a8154ca`).
- `0x48d680` edits configuration fields using loaded option tables. Its field semantics and hardware/UI index mapping still require careful corroboration (Evidence `ev_c00030d0f1a52aeb56f47b7fbd818192dbec9bfcbdbca90cd1c9b1ccc6631f71`).
- Bundled profiles provide 20 `LedOpt` entries, brightness values 0..9, speed values 0..4 and sleep UI/hardware mappings. The old Rust list of 16 invented modes must not be reused.
- `english-strings.json` contains 270 nonempty English localization entries. They are shared across keyboard/mouse families; presence of cloud, mouse, firmware, or music labels does not establish that this particular F75 profile enables those features. `Cfg.ini` sets `EnableYun=0`.

## Hardware observations

- A known GetLED request (`1344010000000000000000000000000000000058`) was accepted by the Linux write call, but no reports arrived within the bounded query window. `hardware-getled.json` explicitly records `validated=false`. This is not a successful profile read.
- A read of the receiver's eight-byte feature report 6 succeeded and returned `060002646464cb0c`; `hardware-feature6.json` retains the bytes. Their device-specific meaning is not yet established.
- The later discovery probe failed to open the control node: permissions had reverted to root-only `0600`, and non-interactive sudo required authentication. There is no validated online/identity reply yet.
- No settings, reset or firmware operation has been sent in the corrected investigation. An earlier guessed packet was sent before this correction; its setting effect is unknown and must not be represented as success.

## Required gates before enabling controls

1. Obtain device-specific access without granting all HID devices world-write permission.
2. Validate discovery online and six-byte identity replies against the selected profile.
3. Read and validate the complete lighting/settings profile; investigate zero replies instead of substituting defaults.
4. Corroborate field mappings and encodings, preserving unedited bytes and unknown fields.
5. Port encoders and parsers with byte-level tests based on identified source evidence and real captured responses. Label synthesized test vectors as such.
6. Exercise read-modify-write-readback for one ordinary setting and get physical confirmation. Only then claim hardware control works.
7. Implement and verify the other original features separately. A window opening, a handle opening, a packet being written and physical feature parity are four different gates.

Firmware flashing, factory reset, global HID permissions, and unverified opcodes are not part of these gates.
