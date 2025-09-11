use crate::{device::DeviceInfo, scanner::{ScannedFile, RecordingEntry}, Result};
use serde::{Deserialize, Serialize};

use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: u32,
    pub id: String,
    pub created_at: String,
    pub device: DeviceInfo,
    pub strategy: BackupStrategy,
    pub files: Vec<FileEntry>,
    pub apks: Vec<ApkEntry>,
    pub contacts: ContactsInfo,
    pub logs: LogsInfo,
    pub recordings: RecordingsInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStrategy {
    pub incremental: bool,
    pub hash_algo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub category: crate::scanner::FileCategory,
    pub size: Option<u64>,
    pub mtime: Option<String>,
    pub rel_dst: String,
    pub sha256: Option<String>,
    pub status: BackupStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApkEntry {
    pub package: String,
    pub version_name: Option<String>,
    pub version_code: Option<String>,
    pub source_path: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsInfo {
    pub status: ExportStatus,
    pub exported_vcf: Option<String>,
    pub exported_csv: Option<String>,
    pub exported_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsInfo {
    pub status: ExportStatus,
    pub calls_json: Option<String>,
    pub sms_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingsInfo {
    pub status: ExportStatus,
    pub entries: Vec<RecordingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "skipped")]
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "no_permissions")]
    NoPermissions,
    #[serde(rename = "not_attempted")]
    NotAttempted,
}

impl BackupManifest {
    /// Create a new backup manifest
    pub fn new(device: DeviceInfo, incremental: bool) -> Self {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();

        Self {
            version: 1,
            id,
            created_at,
            device,
            strategy: BackupStrategy {
                incremental,
                hash_algo: "sha256".to_string(),
            },
            files: Vec::new(),
            apks: Vec::new(),
            contacts: ContactsInfo {
                status: ExportStatus::NotAttempted,
                exported_vcf: None,
                exported_csv: None,
                exported_json: None,
            },
            logs: LogsInfo {
                status: ExportStatus::NotAttempted,
                calls_json: None,
                sms_json: None,
            },
            recordings: RecordingsInfo {
                status: ExportStatus::NotAttempted,
                entries: Vec::new(),
            },
        }
    }

    /// Add scanned files to manifest
    pub fn add_files(&mut self, scanned_files: Vec<ScannedFile>) {
        self.files = scanned_files
            .into_iter()
            .map(|file| FileEntry {
                path: file.path,
                category: file.category,
                size: file.size,
                mtime: file.mtime,
                rel_dst: file.rel_dst,
                sha256: None,
                status: BackupStatus::Pending,
            })
            .collect();
    }

    /// Add APK entries to manifest
    pub fn add_apks(&mut self, apk_entries: Vec<ApkEntry>) {
        self.apks = apk_entries;
    }

    /// Set contacts export info
    pub fn set_contacts_info(&mut self, status: ExportStatus, files: Option<(String, String, String)>) {
        self.contacts.status = status;
        if let Some((vcf, csv, json)) = files {
            self.contacts.exported_vcf = Some(vcf);
            self.contacts.exported_csv = Some(csv);
            self.contacts.exported_json = Some(json);
        }
    }

    /// Set logs export info
    pub fn set_logs_info(&mut self, status: ExportStatus, files: Option<(String, String)>) {
        self.logs.status = status;
        if let Some((calls, sms)) = files {
            self.logs.calls_json = Some(calls);
            self.logs.sms_json = Some(sms);
        }
    }

    /// Set recordings info
    pub fn set_recordings_info(&mut self, status: ExportStatus, entries: Vec<RecordingEntry>) {
        self.recordings.status = status;
        self.recordings.entries = entries;
    }

    /// Update file entry status and hash
    pub fn update_file_status(&mut self, path: &str, status: BackupStatus, sha256: Option<String>) {
        if let Some(file) = self.files.iter_mut().find(|f| f.path == path) {
            file.status = status;
            file.sha256 = sha256;
        }
    }

    /// Get statistics about the backup
    pub fn get_stats(&self) -> BackupStats {
        let mut stats = BackupStats::default();
        
        for file in &self.files {
            match file.status {
                BackupStatus::Success => stats.files_success += 1,
                BackupStatus::Failed => stats.files_failed += 1,
                BackupStatus::Skipped => stats.files_skipped += 1,
                BackupStatus::Pending => stats.files_pending += 1,
            }
            
            if let Some(size) = file.size {
                stats.total_size += size;
            }
        }

        stats.apks_count = self.apks.len();
        stats
    }

    /// Convert to YAML string
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Load from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(yaml)?)
    }

    /// Load from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

#[derive(Debug, Default)]
pub struct BackupStats {
    pub files_success: usize,
    pub files_failed: usize,
    pub files_skipped: usize,
    pub files_pending: usize,
    pub total_size: u64,
    pub apks_count: usize,
}

impl BackupStats {
    pub fn total_files(&self) -> usize {
        self.files_success + self.files_failed + self.files_skipped + self.files_pending
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_files();
        if total == 0 {
            0.0
        } else {
            self.files_success as f64 / total as f64 * 100.0
        }
    }
}