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

//! Nova Backup - Scanning logic, duplicate detection, backup planning

pub mod duplicates;
pub mod planner;
pub mod scanner;
pub mod types;

pub use duplicates::{DuplicateDetector, DuplicateGroup};
pub use planner::{BackupEntry, BackupPlan, BackupPlanner};
pub use scanner::{FileScanner, ScanProgress, ScanResult};
pub use types::{FileCategory, FileInfo, ScanOptions};
