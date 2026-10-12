# Lighting audit: verified fields and corrections

These are static byte-level observations, not proof that the attached hardware accepts them. The 128-byte profile must first be read from the actual matched device and validated; unknown fields must be preserved.

## Sources

- Keyboard-specific option loader 0x410a70: `ev_1a1ed72c03f0908300c8d3568af9d5771dce06bf34181b720546d81fa0df1a5a`.
- Its assembly: `ev_539ebe1149f9b67f2b793cc72946078f3082873b5aad82b1cede3c92ee02796d`.
- Profile packer 0x48d680: `ev_c00030d0f1a52aeb56f47b7fbd818192dbec9bfcbdbca90cd1c9b1ccc6631f71`.
- Profile packer assembly: `ev_56f13efa8a39f16666b80f989cc57e517154a4ee9fdf2c706cc97c0e5cdf0b9f`.
- Apply routine 0x48da30: `ev_a5b57f58fa9fcc7eb70559d04e234d8a8a402e4c69f75e5d38feaf1e0a8154ca`.
- Comma-separated byte parser 0x4609c0: `ev_18290390cdc8c723ffe1419d1d38d449238728d5e3bb31c393d5d3fcc6744bc9`.

Raw pseudocode/assembly files are retained locally under evidence/. The prior audit used the mouse-oriented loader 0x40f330 for some semantics; that is insufficient for the keyboard's structure. The parent traced the second LedOpt string reference at 0x412112 to the actual keyboard loader before accepting the map.

## Accepted field observations

- Profile bytes 0x7e and 0x7f are checked for 0x5a and 0xa5 by ApplySetting.
- Byte 0x09 records whether the selected effect index equals `GaoshouIndex` (18 in the provided F75 INI). It must not be called a generic active-profile flag.
- Byte 0x0a is copied from descriptor byte +2. Keyboard loader assembly 0x41218a..0x4121a5 places the FIRST LedOpt number in this descriptor byte: input[0] is stored at [ESI+1] where ESI is descriptor+1. The second LedOpt number is stored at descriptor+1, not in profile byte 0x0a.
- Consequently, the provided entries' first values (1..18, then 21, then 0) are the encoded mode values. The previous audit's proposed sequence 1,3,2,19,15,...,29 for profile byte 0x0a was rejected: those are the SECOND INI values. Their precise presentation/translation role still needs UI-consumer tracing; do not rename them hardware IDs.
- Byte 0x0b is `StartGp + selected custom-color-group index`. The loader reads StartGp with default 0x20 into the config field consumed by the packer. It is NOT a combined brightness/speed field.
- The dynamic pair array starts at byte 0x3a, not 0x3b. For each parameter entry j, byte `0x3a + 2*j` is a value selected through LightHW; byte `0x3b + 2*j` is `(SpeedHW << 4) | color_low_nibble`, using RandomVal (default 7) when the random/colorful flag is set. This follows the corresponding LightHW/SpeedHW storage in the keyboard loader and MOV/SHL/OR instructions 0x48d6f0..0x48d724.
- The supplied profile's LightHW and SpeedHW tables are 0..9 and 0..4. The default LedParam loader initializes pairs with maximum brightness and maximum speed plus color value 7. Keep index bounds, mode-to-parameter selection and the dynamic entry count explicit rather than inferring them from a display label.
- Byte 0x16 is copied from a user-state field when its neighboring control flag is zero, otherwise cleared. Debounce semantics are strongly suggested but require tracing the corresponding UI state writes before implementation.
- Byte 0x18 selects the SleepHW table entry when the neighboring enable/control field is zero, otherwise clears it. Its UI mapping is present in the supplied INI. Byte 0x17 is written conditionally under a device flag; preserve it until the precise feature condition is established.
- The side-LED block is conditional on ShowSideLED. The keyboard loader defaults ShowSideLED to zero and the supplied F75 INI does not override it. Therefore bytes 0x12..0x15 and the alternate firmware-family-26 block 0x32..0x35 must not be exposed as enabled F75 controls merely because generic code exists.

## Implementation boundary

This corrects the research map; it does not enable hardware writes. Needed next: a validated identity and live 128-byte profile, caller/consumer proof for dynamic parameter indexing and debounce/sleep controls, then byte-preservation tests and hardware readback. Never replace an unavailable profile with zeroes or synthesized defaults and send it to a real device.
