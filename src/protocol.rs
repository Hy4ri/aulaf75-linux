//! AULA F75 Hardware Definitions & Protocol constants

pub const VENDOR_ID_WIRED: u16 = 0x258A;
pub const PRODUCT_ID_WIRED: u16 = 0x010C;

pub const VENDOR_ID_WIRELESS: u16 = 0x3554;
pub const PRODUCT_ID_WIRELESS: u16 = 0xFA09;

pub const REPORT_ID_COMMAND: u8 = 0x13;
pub const REPORT_ID_FEATURE: u8 = 0x06;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    Wired,
    Wireless24G,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct LightingEffect {
    pub id: u8,
    pub name: &'static str,
}

pub const LIGHTING_EFFECTS: &[LightingEffect] = &[
    LightingEffect { id: 1, name: "Wave" },
    LightingEffect { id: 2, name: "Ripple" },
    LightingEffect { id: 3, name: "Raindrops" },
    LightingEffect { id: 4, name: "Snake" },
    LightingEffect { id: 5, name: "Follow" },
    LightingEffect { id: 6, name: "Convergence" },
    LightingEffect { id: 7, name: "Sine Wave" },
    LightingEffect { id: 8, name: "Windmill" },
    LightingEffect { id: 9, name: "Waterfall" },
    LightingEffect { id: 10, name: "Static" },
    LightingEffect { id: 11, name: "Breathing" },
    LightingEffect { id: 12, name: "Neon" },
    LightingEffect { id: 13, name: "Shadow" },
    LightingEffect { id: 14, name: "Laser" },
    LightingEffect { id: 15, name: "Sparkle" },
    LightingEffect { id: 16, name: "Cross" },
];
