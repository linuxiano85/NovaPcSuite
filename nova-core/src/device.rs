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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub serial: String,
    pub model: String,
    pub manufacturer: String,
    pub android_version: String,
    pub build_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub root_available: bool,
    pub can_backup_apps: bool,
    pub mtp_available: bool,
    pub adb_available: bool,
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            root_available: false,
            can_backup_apps: false,
            mtp_available: false,
            adb_available: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub info: DeviceInfo,
    pub capabilities: DeviceCapabilities,
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Adb,
    Mtp,
    Both,
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl Device {
    pub fn new(info: DeviceInfo, capabilities: DeviceCapabilities) -> Self {
        let connection_type = match (capabilities.adb_available, capabilities.mtp_available) {
            (true, true) => ConnectionType::Both,
            (true, false) => ConnectionType::Adb,
            (false, true) => ConnectionType::Mtp,
            (false, false) => ConnectionType::Adb, // Default fallback
        };

        Self {
            info,
            capabilities,
            connection_type,
        }
    }

    pub fn is_root_available(&self) -> bool {
        self.capabilities.root_available
    }

    pub fn can_backup_apps(&self) -> bool {
        self.capabilities.can_backup_apps
    }
}
