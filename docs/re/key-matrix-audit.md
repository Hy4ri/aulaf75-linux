# Key matrix audit: parent disposition

Checked the delegated claims against `key-encoding-assembly.txt` and `key-matrix-assembly.txt`, raw disassembly of the original OemDrv.exe.

Accepted: G5 keyboard translator at 0x48c690; UI descriptors use a major action type in their first word, and a hardware layer has MatrixLen (128) four-byte entries. The formatter at 0x48d140 writes each translated dword at `matrix_index * 4` and advances layer output pointers by MatrixLen*4. Existing padding/trailing bytes must be preserved from a real read; neither zero-fill nor adding a generic signature is justified.

Corrected byte order (i386 little endian):

- Normal key: the SHL/OR sequence at 0x48c885..0x48c89f produces bytes `[0x00, modifiers, HID_key_2, HID_key_1]`, NOT the delegate's `[modifiers, HID_key_1, HID_key_2, 0]`. For one key use `[0, modifiers, 0, usage]`.
- Media: original immediates e.g. 0xe9000002 (0x48c7e8) and 0x92010002 (0x48c806) produce `[0x02,0,usage_high,usage_low]`. The type is byte ZERO, not byte three. The delegate's Mute=0xcd example was also mislabeled: consumer usage cd is Play/Pause, e2 is Mute.
- Disabled: translator case 0x48ca40..0x48ca44 returns all zeroes. Accept.
- Macro reference: 0x48c8cc..0x48c8ff initially produces `[3, playback_mode, loop_count, 0]`; matrix formatter's macro-allocation pass replaces the final byte with a table slot. Do not encode references until table allocation and all related existing references are preserved.

Vtable correction: raw vtable 0x60ff9c has SetMatrix 0x48b590 at+0x50 and key translator0x48c690 at+0x84. Raw base 3632 vtable0x60dc54 has SetMatrix0x486730 at+0x50 but translator0x45a7c0 at+0x84, NOT0x48c690. A common keyboard layout or matching generic method name does not independently prove identical action formats for every wireless subclass. No automatic hardware writes are authorized by these static observations alone.

Model applicability: original INI Psd identifiers differ: standard03 00 00 00 00 cd; wired03 00 00 00 00 e1; KR03 00 00 00 01 91. A device of another identity must be rejected or matched to its actual configuration; never substitute the requested catalogue identity after a failed query.
