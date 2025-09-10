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

use crate::types::{FileCategory, FileInfo, ScanOptions};
use nova_adb::AdbClient;
use nova_core::{Device, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub files_scanned: usize,
    pub total_size: u64,
    pub current_path: PathBuf,
    pub phase: ScanPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanPhase {
    Discovering,
    Scanning,
    Categorizing,
    Hashing,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub files: Vec<FileInfo>,
    pub summary: ScanSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_files: usize,
    pub total_size: u64,
    pub categories: std::collections::HashMap<FileCategory, usize>,
    pub scan_duration_ms: u64,
}

pub struct FileScanner {
    adb_client: AdbClient,
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl FileScanner {
    pub fn new() -> Self {
        Self {
            adb_client: AdbClient::new(),
        }
    }

    pub async fn scan_device(
        &self,
        device: &Device,
        options: &ScanOptions,
        progress_tx: Option<mpsc::UnboundedSender<ScanProgress>>,
    ) -> Result<ScanResult> {
        let start_time = std::time::Instant::now();
        info!("Starting scan of device: {}", device.info.serial);

        let mut files = Vec::new();
        let mut total_size = 0u64;

        // Send initial progress
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(ScanProgress {
                files_scanned: 0,
                total_size: 0,
                current_path: PathBuf::from("Initializing..."),
                phase: ScanPhase::Discovering,
            });
        }

        // Scan each included path
        for include_path in &options.include_paths {
            debug!("Scanning path: {}", include_path);

            if let Some(ref tx) = progress_tx {
                let _ = tx.send(ScanProgress {
                    files_scanned: files.len(),
                    total_size,
                    current_path: PathBuf::from(include_path),
                    phase: ScanPhase::Scanning,
                });
            }

            let path_files = self.scan_path(device, include_path, options).await?;
            for file in path_files {
                total_size += file.size;
                files.push(file);

                // Send progress update every 100 files
                if files.len() % 100 == 0 {
                    if let Some(ref tx) = progress_tx {
                        let _ = tx.send(ScanProgress {
                            files_scanned: files.len(),
                            total_size,
                            current_path: files.last().unwrap().path.clone(),
                            phase: ScanPhase::Scanning,
                        });
                    }
                }
            }
        }

        // Categorize files
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(ScanProgress {
                files_scanned: files.len(),
                total_size,
                current_path: PathBuf::from("Categorizing..."),
                phase: ScanPhase::Categorizing,
            });
        }

        self.categorize_files(&mut files).await?;

        // Compute hashes if requested
        if options.compute_hashes {
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(ScanProgress {
                    files_scanned: files.len(),
                    total_size,
                    current_path: PathBuf::from("Computing hashes..."),
                    phase: ScanPhase::Hashing,
                });
            }

            self.compute_hashes(device, &mut files).await?;
        }

        // Build summary
        let mut categories = std::collections::HashMap::new();
        for file in &files {
            *categories.entry(file.category.clone()).or_insert(0) += 1;
        }

        let summary = ScanSummary {
            total_files: files.len(),
            total_size,
            categories,
            scan_duration_ms: start_time.elapsed().as_millis() as u64,
        };

        if let Some(ref tx) = progress_tx {
            let _ = tx.send(ScanProgress {
                files_scanned: files.len(),
                total_size,
                current_path: PathBuf::from("Complete"),
                phase: ScanPhase::Complete,
            });
        }

        info!("Scan complete: {} files, {} bytes", files.len(), total_size);

        Ok(ScanResult { files, summary })
    }

    async fn scan_path(
        &self,
        device: &Device,
        path: &str,
        _options: &ScanOptions,
    ) -> Result<Vec<FileInfo>> {
        debug!("Scanning device path: {}", path);

        // Use adb shell to find files
        let find_command = format!("find '{}' -type f 2>/dev/null || true", path);
        let output = self
            .adb_client
            .shell_command(&device.info.serial, &find_command)
            .await?;

        let mut files = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let file_path = PathBuf::from(line);

            // Get file stats
            if let Ok(file_info) = self.get_file_info(device, &file_path).await {
                files.push(file_info);
            }
        }

        debug!("Found {} files in path: {}", files.len(), path);
        Ok(files)
    }

    async fn get_file_info(&self, device: &Device, path: &PathBuf) -> Result<FileInfo> {
        // Get file size and modification time using stat
        let stat_command = format!("stat -c '%s %Y' '{}'", path.display());
        let stat_output = self
            .adb_client
            .shell_command(&device.info.serial, &stat_command)
            .await
            .unwrap_or_default();

        let (size, modified) = if let Some(parts) = stat_output.trim().split_once(' ') {
            let size = parts.0.parse::<u64>().unwrap_or(0);
            let modified = parts.1.parse::<u64>().unwrap_or(0);
            (size, modified)
        } else {
            (0, 0)
        };

        let relative_path = path.strip_prefix("/").unwrap_or(path).to_path_buf();

        // Determine category from extension
        let category = if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                FileCategory::from_extension(ext_str)
            } else {
                FileCategory::Other
            }
        } else {
            FileCategory::Other
        };

        Ok(FileInfo {
            path: path.clone(),
            relative_path,
            size,
            modified,
            category,
            mime_type: None,
            hash: None,
        })
    }

    async fn categorize_files(&self, files: &mut [FileInfo]) -> Result<()> {
        debug!("Categorizing {} files", files.len());

        // For now, categorization is done during file info gathering
        // In the future, we could use tree_magic_mini for better MIME detection

        Ok(())
    }

    async fn compute_hashes(&self, _device: &Device, files: &mut [FileInfo]) -> Result<()> {
        debug!("Computing hashes for {} files", files.len());

        // For now, skip hash computation as it would require pulling files
        // This would be implemented in a future version
        warn!("Hash computation not yet implemented for remote files");

        Ok(())
    }
}
