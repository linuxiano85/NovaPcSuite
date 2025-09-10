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

use nova_core::Device;
use nova_backup::{ScanResult, ScanProgress};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiDevice {
    pub serial: String,
    pub model: String,
    pub manufacturer: String,
    pub android_version: String,
    pub root_available: bool,
    pub can_backup_apps: bool,
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Device> for UiDevice {
    fn from(device: Device) -> Self {
        Self {
            serial: device.info.serial,
            model: device.info.model,
            manufacturer: device.info.manufacturer,
            android_version: device.info.android_version,
            root_available: device.capabilities.root_available,
            can_backup_apps: device.capabilities.can_backup_apps,
        }
    }
}

#[derive(Debug, Default)]
pub struct AppState {
    pub scan_progress: Arc<Mutex<Option<ScanProgress>>>,
    pub scan_result: Arc<Mutex<Option<ScanResult>>>,
    pub progress_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<ScanProgress>>>>,
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn set_scan_progress(&self, progress: ScanProgress) {
        if let Ok(mut guard) = self.scan_progress.lock() {
            *guard = Some(progress);
        }
    }
    
    pub fn get_scan_progress(&self) -> Option<ScanProgress> {
        self.scan_progress.lock().ok()?.clone()
    }
    
    pub fn set_scan_result(&self, result: ScanResult) {
        if let Ok(mut guard) = self.scan_result.lock() {
            *guard = Some(result);
        }
    }
    
    pub fn get_scan_result(&self) -> Option<ScanResult> {
        self.scan_result.lock().ok()?.clone()
    }
}