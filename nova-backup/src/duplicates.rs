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

use crate::types::FileInfo;
use nova_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub size: u64,
    pub files: Vec<FileInfo>,
    pub potential_savings: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateDetectionResult {
    pub groups: Vec<DuplicateGroup>,
    pub total_duplicates: usize,
    pub total_savings: u64,
}

pub struct DuplicateDetector;

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl DuplicateDetector {
    pub fn new() -> Self {
        Self
    }

    pub async fn detect_duplicates(&self, files: &[FileInfo]) -> Result<DuplicateDetectionResult> {
        info!("Detecting duplicates among {} files", files.len());

        // Step 1: Group by size
        let mut size_groups: HashMap<u64, Vec<&FileInfo>> = HashMap::new();

        for file in files {
            if file.size > 0 {
                // Skip empty files
                size_groups.entry(file.size).or_default().push(file);
            }
        }

        // Step 2: Find groups with multiple files of same size
        let mut duplicate_groups = Vec::new();
        let mut total_duplicates = 0;
        let mut total_savings = 0;

        for (size, group_files) in size_groups {
            if group_files.len() > 1 {
                debug!("Found {} files with size {}", group_files.len(), size);

                // For now, consider all files of same size as potential duplicates
                // In a full implementation, we would compare hashes here
                let files_owned: Vec<FileInfo> = group_files.into_iter().cloned().collect();

                // Calculate potential savings (all but one file can be saved)
                let savings = size * (files_owned.len() as u64 - 1);

                total_duplicates += files_owned.len() - 1; // All but one are duplicates
                total_savings += savings;

                duplicate_groups.push(DuplicateGroup {
                    size,
                    files: files_owned,
                    potential_savings: savings,
                });
            }
        }

        // Sort groups by potential savings (highest first)
        duplicate_groups.sort_by(|a, b| b.potential_savings.cmp(&a.potential_savings));

        info!(
            "Found {} duplicate groups with {} total duplicates, {} bytes savings potential",
            duplicate_groups.len(),
            total_duplicates,
            total_savings
        );

        Ok(DuplicateDetectionResult {
            groups: duplicate_groups,
            total_duplicates,
            total_savings,
        })
    }

    /// Detect duplicates using hash comparison (for more accurate detection)
    pub async fn detect_duplicates_by_hash(
        &self,
        files: &[FileInfo],
    ) -> Result<DuplicateDetectionResult> {
        info!("Detecting duplicates by hash among {} files", files.len());

        // Group by hash
        let mut hash_groups: HashMap<String, Vec<&FileInfo>> = HashMap::new();

        for file in files {
            if let Some(ref hash) = file.hash {
                hash_groups.entry(hash.clone()).or_default().push(file);
            }
        }

        // Find groups with multiple files having same hash
        let mut duplicate_groups = Vec::new();
        let mut total_duplicates = 0;
        let mut total_savings = 0;

        for (hash, group_files) in hash_groups {
            if group_files.len() > 1 {
                debug!("Found {} files with hash {}", group_files.len(), hash);

                let files_owned: Vec<FileInfo> = group_files.into_iter().cloned().collect();
                let size = files_owned[0].size; // All files should have same size

                // Calculate savings
                let savings = size * (files_owned.len() as u64 - 1);

                total_duplicates += files_owned.len() - 1;
                total_savings += savings;

                duplicate_groups.push(DuplicateGroup {
                    size,
                    files: files_owned,
                    potential_savings: savings,
                });
            }
        }

        // Sort by potential savings
        duplicate_groups.sort_by(|a, b| b.potential_savings.cmp(&a.potential_savings));

        info!(
            "Found {} exact duplicate groups with {} total duplicates, {} bytes savings",
            duplicate_groups.len(),
            total_duplicates,
            total_savings
        );

        Ok(DuplicateDetectionResult {
            groups: duplicate_groups,
            total_duplicates,
            total_savings,
        })
    }

    /// Quick duplicate detection based on size and filename
    pub async fn quick_duplicate_scan(
        &self,
        files: &[FileInfo],
    ) -> Result<DuplicateDetectionResult> {
        info!("Quick duplicate scan among {} files", files.len());

        // Group by (size, filename)
        let mut groups: HashMap<(u64, String), Vec<&FileInfo>> = HashMap::new();

        for file in files {
            if let Some(filename) = file.path.file_name().and_then(|n| n.to_str()) {
                let key = (file.size, filename.to_string());
                groups.entry(key).or_default().push(file);
            }
        }

        // Find potential duplicates
        let mut duplicate_groups = Vec::new();
        let mut total_duplicates = 0;
        let mut total_savings = 0;

        for ((size, _filename), group_files) in groups {
            if group_files.len() > 1 && size > 0 {
                let files_owned: Vec<FileInfo> = group_files.into_iter().cloned().collect();
                let savings = size * (files_owned.len() as u64 - 1);

                total_duplicates += files_owned.len() - 1;
                total_savings += savings;

                duplicate_groups.push(DuplicateGroup {
                    size,
                    files: files_owned,
                    potential_savings: savings,
                });
            }
        }

        duplicate_groups.sort_by(|a, b| b.potential_savings.cmp(&a.potential_savings));

        info!(
            "Quick scan found {} potential duplicate groups with {} total files, {} bytes potential savings",
            duplicate_groups.len(), total_duplicates, total_savings
        );

        Ok(DuplicateDetectionResult {
            groups: duplicate_groups,
            total_duplicates,
            total_savings,
        })
    }
}
