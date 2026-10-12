# Wireless-audit disposition

Read-only audit findings were checked against the saved decompilation and original watcher assembly. Treat the following distinctions as part of the investigation, not as a verified runtime fix.

Accepted observations:
- The original watcher requests online status with command 7, then a six-byte device identifier with command 5, before constructing/binding the CDev3632 backend.
- For an offline response it waits for a separate incoming event with command byte 0x0a and byte 6 equal to 1. The incoming-event handler additionally interprets byte 5 equal to 2 as an online-status event.
- The original Windows implementation uses a dedicated reader thread and buffered replies.
- Outgoing 20-byte reports use the additive checksum routine with the checksum byte initially zero.

Rejected or narrowed claims:
- The six-byte identifier is not established as VID/PID plus a pairing token. The code obtains USB VID/PID separately through HidD_GetAttributes and passes the identifier separately to profile matching. Preserve it as the six-byte Psd/model identifier until further evidence establishes semantics.
- The original application's discovery sequence is not proof that sending its read queries awakens/unlocks the receiver or that the receiver firmware requires them before GetLED. That is a hardware hypothesis requiring observations.
- Windows' dedicated reader thread is not proof that Linux needs a concurrent reader to avoid dropping every reply. The existing Linux probe polls its open hidraw descriptor after a write; zero reports must not be explained by invented kernel behavior.
- Error-bit semantics differ between helpers: discovery at 0x486f70 checks command byte 1's high bit; operational reply helper 0x485f20 checks byte 2's high bit. Do not describe these as one interchangeable flag.
- Strict sequence-index validation is a defensive porting choice, not a check shown in every original receive helper. The discovery routine uses the received index to detect the last chunk, without showing an expected-index comparison on every chunk.

Sources: keyboard-watcher-raw-assembly.txt, wireless-discovery-query.c, wireless-incoming-validation.c, response-485f20-decoded.json, on-new-dongle.json, and the original model profiles. Full Evidence IDs are indexed in evidence/index.json.

Current blocker remains raw-HID permissions; no causal explanation for the earlier zero-reply GetLED result is verified.
