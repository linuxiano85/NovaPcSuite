// Copyright 2025 linuxiano85
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Nova ADB - Safe wrapper around ADB commands

use nova_core::{Device, DeviceCapabilities, DeviceInfo, Result};
use std::process::Command;
use tracing::{debug, info};

pub struct AdbClient;

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AdbClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl AdbClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn list_devices(&self) -> Result<Vec<Device>> {
        debug!("Listing ADB devices");

        let output = Command::new("adb")
            .args(["devices", "-l"])
            .output()
            .map_err(|e| nova_core::Error::Adb(format!("Failed to execute adb devices: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(nova_core::Error::Adb(format!(
                "adb devices failed: {}",
                error
            )));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        self.parse_devices(&output_str).await
    }

    async fn parse_devices(&self, output: &str) -> Result<Vec<Device>> {
        let mut devices = Vec::new();

        for line in output.lines().skip(1) {
            // Skip "List of devices attached"
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == "device" {
                let serial = parts[0].to_string();

                // Get device properties
                if let Ok(device) = self.get_device_info(&serial).await {
                    devices.push(device);
                }
            }
        }

        info!("Found {} connected devices", devices.len());
        Ok(devices)
    }

    async fn get_device_info(&self, serial: &str) -> Result<Device> {
        debug!("Getting device info for serial: {}", serial);

        // Get basic device properties
        let model = self
            .get_property(serial, "ro.product.model")
            .await
            .unwrap_or_else(|| "Unknown".to_string());
        let manufacturer = self
            .get_property(serial, "ro.product.manufacturer")
            .await
            .unwrap_or_else(|| "Unknown".to_string());
        let android_version = self
            .get_property(serial, "ro.build.version.release")
            .await
            .unwrap_or_else(|| "Unknown".to_string());
        let build_version = self
            .get_property(serial, "ro.build.display.id")
            .await
            .unwrap_or_else(|| "Unknown".to_string());

        let info = DeviceInfo {
            serial: serial.to_string(),
            model,
            manufacturer,
            android_version,
            build_version,
        };

        // Check device capabilities
        let capabilities = self.get_device_capabilities(serial).await;

        Ok(Device::new(info, capabilities))
    }

    async fn get_property(&self, serial: &str, property: &str) -> Option<String> {
        let output = Command::new("adb")
            .args(["-s", serial, "shell", "getprop", property])
            .output()
            .ok()?;

        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !value.is_empty() {
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }

    async fn get_device_capabilities(&self, serial: &str) -> DeviceCapabilities {
        // Check if device has root access
        let root_available = self.check_root_access(serial).await;

        DeviceCapabilities {
            root_available,
            can_backup_apps: root_available, // For now, app backup requires root
            mtp_available: false,            // TODO: Implement MTP detection
            adb_available: true,
        }
    }

    async fn check_root_access(&self, serial: &str) -> bool {
        debug!("Checking root access for device: {}", serial);

        let output = Command::new("adb")
            .args(["-s", serial, "shell", "id"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let id_output = String::from_utf8_lossy(&output.stdout);
                let has_root = id_output.contains("uid=0(root)");
                debug!("Root access check result: {}", has_root);
                has_root
            }
            _ => {
                debug!("Failed to check root access, assuming no root");
                false
            }
        }
    }

    pub async fn pull_file(&self, serial: &str, source: &str, destination: &str) -> Result<()> {
        debug!("Pulling file from {} to {}", source, destination);

        let output = Command::new("adb")
            .args(["-s", serial, "pull", source, destination])
            .output()
            .map_err(|e| nova_core::Error::Adb(format!("Failed to execute adb pull: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(nova_core::Error::Adb(format!("adb pull failed: {}", error)));
        }

        info!("Successfully pulled file: {} -> {}", source, destination);
        Ok(())
    }

    pub async fn shell_command(&self, serial: &str, command: &str) -> Result<String> {
        debug!("Executing shell command on {}: {}", serial, command);

        let output = Command::new("adb")
            .args(["-s", serial, "shell", command])
            .output()
            .map_err(|e| nova_core::Error::Adb(format!("Failed to execute adb shell: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(nova_core::Error::Adb(format!(
                "adb shell failed: {}",
                error
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
