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

use crate::state::{AppState, UiDevice};
use nova_adb::AdbClient;
use nova_backup::{FileScanner, ScanOptions, DuplicateDetector};
use nova_formats::{AndroidContactSource, ContactSource, VcfExporter, ContactExporter};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanRequest {
    pub device_serial: String,
    pub include_paths: Vec<String>,
    pub compute_hashes: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportContactsRequest {
    pub device_serial: String,
    pub output_path: String,
    pub format: String, // "vcf" or "csv"
}

#[tauri::command]
pub async fn list_devices() -> Result<Vec<UiDevice>, String> {
    info!("UI: Listing devices");
    
    let adb_client = AdbClient::new();
    let devices = adb_client.list_devices().await
        .map_err(|e| format!("Failed to list devices: {}", e))?;
    
    let ui_devices = devices.into_iter().map(UiDevice::from).collect();
    Ok(ui_devices)
}

#[tauri::command]
pub async fn scan_device(
    request: ScanRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!("UI: Starting device scan for {}", request.device_serial);
    
    let adb_client = AdbClient::new();
    let devices = adb_client.list_devices().await
        .map_err(|e| format!("Failed to list devices: {}", e))?;
    
    let device = devices.into_iter()
        .find(|d| d.info.serial == request.device_serial)
        .ok_or_else(|| format!("Device {} not found", request.device_serial))?;
    
    // Set up progress channel
    let (progress_tx, progress_rx) = mpsc::unbounded_channel();
    
    // Store the receiver in state for the frontend to poll
    if let Ok(mut guard) = state.progress_receiver.lock() {
        *guard = Some(progress_rx);
    }
    
    let scanner = FileScanner::new();
    let options = ScanOptions {
        include_paths: if request.include_paths.is_empty() {
            vec![
                "/storage/emulated/0/DCIM".to_string(),
                "/storage/emulated/0/Pictures".to_string(),
                "/storage/emulated/0/Movies".to_string(),
                "/storage/emulated/0/Music".to_string(),
                "/storage/emulated/0/Documents".to_string(),
            ]
        } else {
            request.include_paths
        },
        exclude_patterns: vec![".thumbnail".to_string(), ".cache".to_string()],
        max_depth: Some(10),
        follow_symlinks: false,
        compute_hashes: request.compute_hashes,
        max_parallel: 4,
    };
    
    // Clone state for the async task
    let state_clone = state.inner().clone();
    
    // Start scan in background
    tokio::spawn(async move {
        match scanner.scan_device(&device, &options, Some(progress_tx)).await {
            Ok(result) => {
                info!("UI: Scan completed successfully");
                state_clone.set_scan_result(result);
            }
            Err(e) => {
                error!("UI: Scan failed: {}", e);
            }
        }
    });
    
    Ok("Scan started".to_string())
}

#[tauri::command]
pub async fn get_scan_progress(state: State<'_, AppState>) -> Result<Option<nova_backup::ScanProgress>, String> {
    // Try to read from the progress receiver
    if let Ok(mut guard) = state.progress_receiver.lock() {
        if let Some(ref mut rx) = *guard {
            if let Ok(progress) = rx.try_recv() {
                state.set_scan_progress(progress.clone());
                return Ok(Some(progress));
            }
        }
    }
    
    // Return last known progress
    Ok(state.get_scan_progress())
}

#[tauri::command]
pub async fn export_contacts(request: ExportContactsRequest) -> Result<String, String> {
    info!("UI: Exporting contacts for device {}", request.device_serial);
    
    let adb_client = AdbClient::new();
    let devices = adb_client.list_devices().await
        .map_err(|e| format!("Failed to list devices: {}", e))?;
    
    let device = devices.into_iter()
        .find(|d| d.info.serial == request.device_serial)
        .ok_or_else(|| format!("Device {} not found", request.device_serial))?;
    
    let contact_source = AndroidContactSource::new();
    let contacts = contact_source.fetch_contacts(&device).await
        .map_err(|e| format!("Failed to fetch contacts: {}", e))?;
    
    let output_path = PathBuf::from(request.output_path);
    
    match request.format.as_str() {
        "vcf" => {
            let exporter = VcfExporter::new();
            exporter.export_contacts(&contacts, &output_path).await
                .map_err(|e| format!("Failed to export contacts: {}", e))?;
        }
        "csv" => {
            let exporter = nova_formats::CsvExporter::new();
            exporter.export_contacts(&contacts, &output_path).await
                .map_err(|e| format!("Failed to export contacts: {}", e))?;
        }
        _ => return Err("Invalid export format".to_string()),
    }
    
    Ok(format!("Exported {} contacts", contacts.len()))
}

#[tauri::command]
pub async fn get_scan_result(state: State<'_, AppState>) -> Result<Option<nova_backup::ScanResult>, String> {
    Ok(state.get_scan_result())
}

#[tauri::command]
pub async fn detect_duplicates(state: State<'_, AppState>) -> Result<Option<nova_backup::DuplicateDetectionResult>, String> {
    if let Some(scan_result) = state.get_scan_result() {
        let detector = DuplicateDetector::new();
        let duplicates = detector.detect_duplicates(&scan_result.files).await
            .map_err(|e| format!("Failed to detect duplicates: {}", e))?;
        Ok(Some(duplicates))
    } else {
        Ok(None)
    }
}