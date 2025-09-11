use crate::{adb::AdbWrapper, NovaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub serial: String,
    pub model: String,
    pub brand: String,
    pub android_version: String,
    pub sdk: String,
    pub product: String,
    pub manufacturer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootloaderInfo {
    pub locked: Option<bool>,
    pub verified_boot_state: Option<String>,
    pub unlock_guidance: Option<String>,
}

pub struct DeviceManager {
    adb: AdbWrapper,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            adb: AdbWrapper::new(),
        }
    }

    /// Get device information using getprop
    pub fn get_device_info(&self, serial: &str) -> Result<DeviceInfo> {
        debug!("Collecting device info for {}", serial);

        let properties = [
            "ro.product.model",
            "ro.product.brand", 
            "ro.build.version.release",
            "ro.build.version.sdk",
            "ro.product.name",
            "ro.product.manufacturer",
        ];

        let mut prop_values = HashMap::new();
        
        for prop in &properties {
            match self.adb.getprop(serial, Some(prop)) {
                Ok(value) => {
                    prop_values.insert(prop.to_string(), value);
                }
                Err(e) => {
                    debug!("Failed to get property {}: {}", prop, e);
                    prop_values.insert(prop.to_string(), "Unknown".to_string());
                }
            }
        }

        Ok(DeviceInfo {
            serial: serial.to_string(),
            model: prop_values.get("ro.product.model")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
            brand: prop_values.get("ro.product.brand")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
            android_version: prop_values.get("ro.build.version.release")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
            sdk: prop_values.get("ro.build.version.sdk")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
            product: prop_values.get("ro.product.name")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
            manufacturer: prop_values.get("ro.product.manufacturer")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
        })
    }

    /// Get bootloader information and unlock guidance
    pub fn get_bootloader_info(&self, serial: &str) -> Result<BootloaderInfo> {
        debug!("Collecting bootloader info for {}", serial);

        let locked = self.adb.getprop(serial, Some("ro.boot.flash.locked"))
            .ok()
            .and_then(|val| {
                match val.as_str() {
                    "1" | "true" => Some(true),
                    "0" | "false" => Some(false),
                    _ => None,
                }
            });

        let verified_boot_state = self.adb.getprop(serial, Some("ro.boot.verifiedbootstate"))
            .ok()
            .filter(|s| !s.is_empty());

        // Generate unlock guidance based on device info
        let device_info = self.get_device_info(serial)?;
        let unlock_guidance = self.generate_unlock_guidance(&device_info);

        Ok(BootloaderInfo {
            locked,
            verified_boot_state,
            unlock_guidance,
        })
    }

    /// Generate unlock guidance based on device brand/model
    fn generate_unlock_guidance(&self, device_info: &DeviceInfo) -> Option<String> {
        match device_info.brand.to_lowercase().as_str() {
            "xiaomi" | "redmi" => {
                Some(format!(
                    "Xiaomi/Redmi Unlock Guide for {}:\n\
                    1. Enable Developer Options and USB Debugging\n\
                    2. Add Mi Account in Developer Options\n\
                    3. Request unlock permission at https://en.miui.com/unlock/\n\
                    4. Wait for approval (1-30 days)\n\
                    5. Download Mi Unlock Tool\n\
                    6. Boot device to fastboot mode (Power + Vol Down)\n\
                    7. Use Mi Unlock Tool to unlock\n\
                    Note: This will void warranty and erase all data!",
                    device_info.model
                ))
            }
            "samsung" => {
                Some(format!(
                    "Samsung Unlock Guide for {}:\n\
                    1. Enable Developer Options and USB Debugging\n\
                    2. Enable OEM Unlocking in Developer Options\n\
                    3. Boot to Download Mode (Power + Vol Down + Vol Up)\n\
                    4. Use Odin or Heimdall tools\n\
                    Note: Knox will be triggered and warranty voided!",
                    device_info.model
                ))
            }
            "oneplus" => {
                Some(format!(
                    "OnePlus Unlock Guide for {}:\n\
                    1. Enable Developer Options and USB Debugging\n\
                    2. Enable Advanced Reboot and OEM Unlocking\n\
                    3. Boot to fastboot mode\n\
                    4. Run: fastboot oem unlock\n\
                    Note: This will erase all data!",
                    device_info.model
                ))
            }
            _ => {
                Some(format!(
                    "Generic Unlock Guide for {}:\n\
                    1. Enable Developer Options and USB Debugging\n\
                    2. Enable OEM Unlocking (if available)\n\
                    3. Boot to fastboot/download mode\n\
                    4. Check manufacturer-specific unlock process\n\
                    5. Use appropriate tools (fastboot, manufacturer tools)\n\
                    Warning: Unlocking bootloader voids warranty!",
                    device_info.model
                ))
            }
        }
    }

    /// List all connected devices
    pub fn list_devices(&self) -> Result<Vec<crate::adb::Device>> {
        self.adb.list_devices()
    }

    /// Get first available device serial
    pub fn get_default_device(&self) -> Result<String> {
        let devices = self.list_devices()?;
        
        // Find first device in "device" state
        for device in &devices {
            if device.state == "device" {
                return Ok(device.serial.clone());
            }
        }

        if devices.is_empty() {
            Err(NovaError::Device("No devices connected".to_string()))
        } else {
            Err(NovaError::Device(format!(
                "No devices in 'device' state. Found: {}",
                devices.iter()
                    .map(|d| format!("{}:{}", d.serial, d.state))
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        }
    }
}