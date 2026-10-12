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

        // 1. Try Wireless (Dongle) - target Interface 1 (UsagePage 0xff02)
        for dev in api.device_list() {
            if dev.vendor_id() == VENDOR_ID_WIRELESS && dev.product_id() == PRODUCT_ID_WIRELESS {
                if dev.interface_number() == 1 {
                    if let Ok(handle) = dev.open_device(&api) {
                        self.device = Some(handle);
                        self.mode = ConnectionMode::Wireless24G;
                        return Ok(true);
                    }
                }
            }
        }

        // 2. Try Wired USB - target Interface 1
        for dev in api.device_list() {
            if dev.vendor_id() == VENDOR_ID_WIRED && dev.product_id() == PRODUCT_ID_WIRED {
                if dev.interface_number() == 1 {
                    if let Ok(handle) = dev.open_device(&api) {
                        self.device = Some(handle);
                        self.mode = ConnectionMode::Wired;
                        return Ok(true);
                    }
                }
            }
        }

        self.mode = ConnectionMode::Disconnected;
        Ok(false)
    }

    /// Hardware writes are blocked until the F75-specific protocol and replies
    /// have been recovered and validated. Opening the dongle is not a handshake.
    pub fn set_rgb_mode(&self, _mode_id: u8, _brightness: u8, _speed: u8) -> Result<()> {
        anyhow::bail!("Lighting writes are unavailable: F75 protocol validation is incomplete")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnected_lighting_request_must_not_report_success() {
        let device = AulaDevice::new();
        assert!(device.set_rgb_mode(1, 9, 2).is_err());
    }
}
