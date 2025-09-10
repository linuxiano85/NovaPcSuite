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
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub size: u64,
    pub modified: u64,
    pub category: FileCategory,
    pub mime_type: Option<String>,
    pub hash: Option<String>, // SHA256 hash, computed lazily
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileCategory {
    Images,
    Videos,
    Audio,
    Documents,
    Archives,
    Other,
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl FileCategory {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "raw" | "dng" => {
                Self::Images
            }
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "3gp" => Self::Videos,
            "mp3" | "aac" | "flac" | "ogg" | "wav" | "m4a" | "wma" | "opus" => Self::Audio,
            "pdf" | "doc" | "docx" | "txt" | "rtf" | "odt" | "xls" | "xlsx" | "ppt" | "pptx" => {
                Self::Documents
            }
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => Self::Archives,
            _ => Self::Other,
        }
    }

    pub fn from_mime_type(mime: &str) -> Self {
        if mime.starts_with("image/") {
            Self::Images
        } else if mime.starts_with("video/") {
            Self::Videos
        } else if mime.starts_with("audio/") {
            Self::Audio
        } else if mime.starts_with("text/") || mime.contains("document") || mime.contains("pdf") {
            Self::Documents
        } else if mime.contains("archive") || mime.contains("compressed") {
            Self::Archives
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    pub include_paths: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_depth: Option<usize>,
    pub follow_symlinks: bool,
    pub compute_hashes: bool,
    pub max_parallel: usize,
}
