use anyhow::{Context, Result};
use hidapi::{HidApi, HidDevice};
use crate::protocol::*;

pub struct AulaDevice {
    device: Option<HidDevice>,
    pub mode: ConnectionMode,
}

impl AulaDevice {
    pub fn new() -> Self {
        Self {
            device: None,
            mode: ConnectionMode::Disconnected,
        }
    }

    pub fn connect(&mut self) -> Result<bool> {
        let api = HidApi::new().context("Failed to initialize HID API")?;

        // 1. Try Wireless (Dongle)
        for dev in api.device_list() {
            if dev.vendor_id() == VENDOR_ID_WIRELESS && dev.product_id() == PRODUCT_ID_WIRELESS {
                // Look for the control interface (usage page 0xff02 or interface 1)
                if let Ok(handle) = dev.open_device(&api) {
                    self.device = Some(handle);
                    self.mode = ConnectionMode::Wireless24G;
                    return Ok(true);
                }
            }
        }

        // 2. Try Wired USB
        for dev in api.device_list() {
            if dev.vendor_id() == VENDOR_ID_WIRED && dev.product_id() == PRODUCT_ID_WIRED {
                if let Ok(handle) = dev.open_device(&api) {
                    self.device = Some(handle);
                    self.mode = ConnectionMode::Wired;
                    return Ok(true);
                }
            }
        }

        self.mode = ConnectionMode::Disconnected;
        Ok(false)
    }

    pub fn set_rgb_mode(&self, mode_id: u8, brightness: u8, speed: u8) -> Result<()> {
        if let Some(ref dev) = self.device {
            let mut buf = [0u8; 19];
            buf[0] = REPORT_ID_COMMAND;
            buf[1] = 0x04; // CMD: lighting
            buf[2] = mode_id;
            buf[3] = brightness;
            buf[4] = speed;
            dev.write(&buf).context("Failed to write HID packet")?;
        }
        Ok(())
    }
}
