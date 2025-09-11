use crate::{adb::AdbWrapper, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileCategory {
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "video")]
    Video,
    #[serde(rename = "audio")]
    Audio,
    #[serde(rename = "document")]
    Document,
    #[serde(rename = "apk")]
    Apk,
    #[serde(rename = "other")]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedFile {
    pub path: String,
    pub category: FileCategory,
    pub size: Option<u64>,
    pub mtime: Option<String>,
    pub rel_dst: String, // Relative destination path for backup
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingEntry {
    pub path: String,
    pub exists: bool,
}

pub struct FileScanner {
    adb: AdbWrapper,
    whitelisted_dirs: Vec<String>,
}

impl FileScanner {
    pub fn new() -> Self {
        Self {
            adb: AdbWrapper::new(),
            whitelisted_dirs: vec![
                "/sdcard/DCIM".to_string(),
                "/sdcard/Pictures".to_string(),
                "/sdcard/Movies".to_string(),
                "/sdcard/Music".to_string(),
                "/sdcard/Documents".to_string(),
                "/sdcard/Download".to_string(),
                "/sdcard/WhatsApp/Media".to_string(),
                "/sdcard/Telegram".to_string(),
                "/sdcard/Recordings".to_string(),
                "/sdcard/MIUI/sound_recorder".to_string(),
            ],
        }
    }

    pub fn with_custom_dirs(dirs: Vec<String>) -> Self {
        Self {
            adb: AdbWrapper::new(),
            whitelisted_dirs: dirs,
        }
    }

    /// Scan device for files in whitelisted directories
    pub fn scan_device(&self, serial: &str) -> Result<Vec<ScannedFile>> {
        debug!("Scanning device {} for files", serial);
        let mut all_files = Vec::new();

        for dir in &self.whitelisted_dirs {
            debug!("Scanning directory: {}", dir);
            match self.scan_directory(serial, dir) {
                Ok(mut files) => {
                    debug!("Found {} files in {}", files.len(), dir);
                    all_files.append(&mut files);
                }
                Err(e) => {
                    warn!("Failed to scan directory {}: {}", dir, e);
                    // Continue with other directories
                }
            }
        }

        debug!("Total files found: {}", all_files.len());
        Ok(all_files)
    }

    /// Scan a specific directory for files
    fn scan_directory(&self, serial: &str, dir: &str) -> Result<Vec<ScannedFile>> {
        // First check if directory exists
        let check_cmd = format!("[ -d '{}' ] && echo 'exists' || echo 'not_found'", dir);
        let exists_result = self.adb.shell(serial, &check_cmd)?;
        
        if exists_result.trim() != "exists" {
            debug!("Directory {} does not exist", dir);
            return Ok(Vec::new());
        }

        // Use find command to list files with details
        let find_cmd = format!(
            "find '{}' -type f -printf '%p|%s|%T@\\n' 2>/dev/null || find '{}' -type f",
            dir, dir
        );
        
        let output = self.adb.shell(serial, &find_cmd)?;
        let mut files = Vec::new();

        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let file = if line.contains('|') {
                // Detailed format with size and mtime
                self.parse_detailed_file_line(line, dir)?
            } else {
                // Fallback format (just paths)
                self.parse_simple_file_line(line.trim(), dir)?
            };

            if let Some(file) = file {
                files.push(file);
            }
        }

        Ok(files)
    }

    /// Parse detailed file line (path|size|mtime)
    fn parse_detailed_file_line(&self, line: &str, base_dir: &str) -> Result<Option<ScannedFile>> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 3 {
            return self.parse_simple_file_line(line, base_dir);
        }

        let path = parts[0].trim();
        let size = parts[1].parse::<u64>().ok();
        let mtime = parts[2].parse::<f64>()
            .ok()
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            });

        Ok(Some(ScannedFile {
            path: path.to_string(),
            category: self.classify_file(path),
            size,
            mtime,
            rel_dst: self.compute_relative_path(path, base_dir),
        }))
    }

    /// Parse simple file line (just path)
    fn parse_simple_file_line(&self, path: &str, base_dir: &str) -> Result<Option<ScannedFile>> {
        if path.is_empty() {
            return Ok(None);
        }

        Ok(Some(ScannedFile {
            path: path.to_string(),
            category: self.classify_file(path),
            size: None,
            mtime: None,
            rel_dst: self.compute_relative_path(path, base_dir),
        }))
    }

    /// Classify file based on extension
    fn classify_file(&self, path: &str) -> FileCategory {
        let path_lower = path.to_lowercase();
        
        if let Some(ext) = Path::new(&path_lower).extension() {
            let ext_str = ext.to_string_lossy();
            
            match ext_str.as_ref() {
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "heif" => FileCategory::Image,
                "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "3gp" => FileCategory::Video,
                "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" | "wma" => FileCategory::Audio,
                "pdf" | "doc" | "docx" | "txt" | "rtf" | "odt" | "xls" | "xlsx" | "ppt" | "pptx" => FileCategory::Document,
                "apk" => FileCategory::Apk,
                _ => FileCategory::Other,
            }
        } else {
            FileCategory::Other
        }
    }

    /// Compute relative destination path for backup
    fn compute_relative_path(&self, full_path: &str, base_dir: &str) -> String {
        if let Some(stripped) = full_path.strip_prefix(base_dir) {
            stripped.trim_start_matches('/').to_string()
        } else {
            // Fallback: use the full path
            full_path.replace('/', "_")
        }
    }

    /// Detect recording files/directories
    pub fn detect_recordings(&self, serial: &str) -> Result<Vec<RecordingEntry>> {
        debug!("Detecting recording paths on device {}", serial);
        
        let candidate_paths = vec![
            "/sdcard/Recordings",
            "/sdcard/MIUI/sound_recorder", 
            "/sdcard/SoundRecorder",
            "/sdcard/Voice Recorder",
            "/sdcard/AudioRecorder",
            "/sdcard/Call Recordings",
            "/sdcard/Music/Recordings",
            "/sdcard/Android/data/com.miui.soundrecorder/files",
        ];

        let mut recordings = Vec::new();

        for path in candidate_paths {
            let check_cmd = format!("[ -e '{}' ] && echo 'exists' || echo 'not_found'", path);
            let result = self.adb.shell(serial, &check_cmd)
                .unwrap_or_else(|_| "not_found".to_string());
            
            recordings.push(RecordingEntry {
                path: path.to_string(),
                exists: result.trim() == "exists",
            });
        }

        debug!("Checked {} recording paths", recordings.len());
        Ok(recordings)
    }

    /// Get file categories statistics
    pub fn get_category_stats(&self, files: &[ScannedFile]) -> HashMap<FileCategory, usize> {
        let mut stats = HashMap::new();
        
        for file in files {
            *stats.entry(file.category.clone()).or_insert(0) += 1;
        }

        stats
    }
}