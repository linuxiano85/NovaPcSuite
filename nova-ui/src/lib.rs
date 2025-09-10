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

//! Nova UI - Tauri-based user interface

pub mod commands;
pub mod state;

use tauri::Manager;
use tracing::info;

pub fn run() {
    info!("Starting Nova UI");
    
    tauri::Builder::default()
        .setup(|app| {
            info!("Tauri app setup complete");
            Ok(())
        })
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::list_devices,
            commands::scan_device,
            commands::get_scan_progress,
            commands::export_contacts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}