# aulaf75-linux ⌨️🦀

Native Linux GUI and HID driver for the **AULA F75** mechanical keyboard, written in Rust using [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui).

## Features
- 🚀 **100% Native & GPU Accelerated:** Built with GPUI for 120Hz+ fluid rendering under Wayland/X11.
- 🔌 **Direct HID Communication:** Talks straight to the keyboard (Wired `0x258A:0x010C` & 2.4G Wireless Dongle `0x3554:0xFA09`) without Wine or background bloat.
- 💡 **Lighting & Effects Control:** Realtime RGB modes, speeds, brightness, and custom colors.
- ⌨️ **Matrix Key Mapping & Profiles:** Full layout customization.

## Development

```bash
# Enter dev shell
nix develop

# Run the app
cargo run
```
