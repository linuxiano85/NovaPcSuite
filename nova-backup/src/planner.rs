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

use crate::types::{FileCategory, FileInfo};
use nova_core::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPlan {
    pub version: u32,
    pub created_at: u64,
    pub device_serial: String,
    pub entries: Vec<BackupEntry>,
    pub metadata: BackupPlanMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub source_path: PathBuf,
    pub relative_path: PathBuf,
    pub category: FileCategory,
    pub size: u64,
    pub hash: Option<String>,
    pub priority: BackupPriority,
    pub compression_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPlanMetadata {
    pub total_files: usize,
    pub total_size: u64,
    pub estimated_compressed_size: Option<u64>,
    pub include_paths: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub compression_algorithm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum BackupPriority {
    Low,
    Normal,
    High,
    Critical,
}

pub struct BackupPlanner;

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl BackupPlanner {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_plan(
        &self,
        device_serial: &str,
        files: &[FileInfo],
        include_paths: &[String],
        options: &BackupPlanOptions,
    ) -> Result<BackupPlan> {
        info!("Creating backup plan for {} files", files.len());

        let mut entries = Vec::new();
        let mut total_size = 0u64;

        // Filter files based on include paths
        let filtered_files = self.filter_files_by_paths(files, include_paths);

        debug!(
            "Filtered to {} files from include paths",
            filtered_files.len()
        );

        for file in filtered_files {
            // Determine priority based on file category and size
            let priority = self.determine_priority(&file, options);

            // Skip files below minimum size threshold
            if file.size < options.min_file_size {
                continue;
            }

            total_size += file.size;

            entries.push(BackupEntry {
                source_path: file.path.clone(),
                relative_path: file.relative_path.clone(),
                category: file.category.clone(),
                size: file.size,
                hash: file.hash.clone(),
                priority,
                compression_enabled: options.compression_enabled && self.should_compress(&file),
            });
        }

        // Sort entries by priority (highest first) then by size (largest first)
        entries.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| b.size.cmp(&a.size))
        });

        // Estimate compressed size if compression is enabled
        let estimated_compressed_size = if options.compression_enabled {
            Some(self.estimate_compressed_size(&entries))
        } else {
            None
        };

        let metadata = BackupPlanMetadata {
            total_files: entries.len(),
            total_size,
            estimated_compressed_size,
            include_paths: include_paths.to_vec(),
            exclude_patterns: options.exclude_patterns.clone(),
            compression_algorithm: if options.compression_enabled {
                Some("zstd".to_string())
            } else {
                None
            },
        };

        let plan = BackupPlan {
            version: 1,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            device_serial: device_serial.to_string(),
            entries,
            metadata,
        };

        info!(
            "Created backup plan: {} files, {} bytes total",
            plan.metadata.total_files, plan.metadata.total_size
        );

        Ok(plan)
    }

    fn filter_files_by_paths<'a>(
        &self,
        files: &'a [FileInfo],
        include_paths: &[String],
    ) -> Vec<&'a FileInfo> {
        files
            .iter()
            .filter(|file| {
                include_paths
                    .iter()
                    .any(|include_path| file.path.starts_with(include_path))
            })
            .collect()
    }

    fn determine_priority(&self, file: &FileInfo, options: &BackupPlanOptions) -> BackupPriority {
        // Priority based on category and user preferences
        match file.category {
            FileCategory::Images | FileCategory::Videos => {
                if options.prioritize_media {
                    BackupPriority::High
                } else {
                    BackupPriority::Normal
                }
            }
            FileCategory::Documents => BackupPriority::High,
            FileCategory::Audio => BackupPriority::Normal,
            FileCategory::Archives => BackupPriority::Low,
            FileCategory::Other => {
                // Large files get higher priority
                if file.size > 100 * 1024 * 1024 {
                    // 100MB
                    BackupPriority::Normal
                } else {
                    BackupPriority::Low
                }
            }
        }
    }

    fn should_compress(&self, file: &FileInfo) -> bool {
        // Don't compress already compressed formats
        match file.category {
            FileCategory::Images
            | FileCategory::Videos
            | FileCategory::Audio
            | FileCategory::Archives => false,
            FileCategory::Documents | FileCategory::Other => true,
        }
    }

    fn estimate_compressed_size(&self, entries: &[BackupEntry]) -> u64 {
        entries
            .iter()
            .map(|entry| {
                if entry.compression_enabled {
                    // Rough estimate: text/documents compress to ~30%, others to ~70%
                    match entry.category {
                        FileCategory::Documents => entry.size * 30 / 100,
                        FileCategory::Other => entry.size * 70 / 100,
                        _ => entry.size, // Already compressed formats
                    }
                } else {
                    entry.size
                }
            })
            .sum()
    }

    pub fn save_plan(&self, plan: &BackupPlan, output_path: &PathBuf) -> Result<()> {
        debug!("Saving backup plan to: {:?}", output_path);

        let json = serde_json::to_string_pretty(plan)?;
        std::fs::write(output_path, json)?;

        info!("Backup plan saved to: {:?}", output_path);
        Ok(())
    }

    pub fn load_plan(&self, plan_path: &PathBuf) -> Result<BackupPlan> {
        debug!("Loading backup plan from: {:?}", plan_path);

        let content = std::fs::read_to_string(plan_path)?;
        let plan: BackupPlan = serde_json::from_str(&content)?;

        info!("Loaded backup plan: {} files", plan.metadata.total_files);
        Ok(plan)
    }
}

#[derive(Debug, Clone)]
pub struct BackupPlanOptions {
    pub compression_enabled: bool,
    pub prioritize_media: bool,
    pub min_file_size: u64,
    pub exclude_patterns: Vec<String>,
}
