use crate::{
    adb::AdbWrapper,
    manifest::{BackupManifest, BackupStatus},
    NovaError, Result
};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn, error};

pub struct RestoreExecutor {
    adb: AdbWrapper,
}

impl RestoreExecutor {
    pub fn new() -> Self {
        Self {
            adb: AdbWrapper::new(),
        }
    }

    /// Restore files from backup to target directory
    pub async fn restore_to_directory(&self, backup_id: &str, root_dir: &Path, target_dir: &Path) -> Result<RestoreStats> {
        info!("Starting restore of backup {} to {}", backup_id, target_dir.display());

        // Find backup directory
        let backup_dir = self.find_backup_directory(root_dir, backup_id)?;
        info!("Found backup directory: {}", backup_dir.display());

        // Load manifest
        let manifest = self.load_manifest(&backup_dir)?;
        info!("Loaded manifest for device: {} {} ({})", 
              manifest.device.brand, manifest.device.model, manifest.device.serial);

        // Create target directory
        fs::create_dir_all(target_dir)
            .map_err(|e| NovaError::Restore(format!("Failed to create target directory: {}", e)))?;

        // Restore files
        let stats = self.restore_files(&backup_dir, target_dir, &manifest).await?;

        info!("Restore completed: {}/{} files successful", stats.files_success, stats.total_files);
        Ok(stats)
    }

    /// Restore files from backup to Android device
    pub async fn restore_to_device(&self, backup_id: &str, root_dir: &Path, target_serial: &str) -> Result<RestoreStats> {
        info!("Starting restore of backup {} to device {}", backup_id, target_serial);

        // Find backup directory
        let backup_dir = self.find_backup_directory(root_dir, backup_id)?;
        
        // Load manifest
        let manifest = self.load_manifest(&backup_dir)?;
        
        // Restore files to device
        let stats = self.restore_files_to_device(&backup_dir, target_serial, &manifest).await?;

        info!("Device restore completed: {}/{} files successful", stats.files_success, stats.total_files);
        Ok(stats)
    }

    /// Find backup directory by ID
    fn find_backup_directory(&self, root_dir: &Path, backup_id: &str) -> Result<PathBuf> {
        // Search through all device directories and timestamps
        for device_entry in fs::read_dir(root_dir)
            .map_err(|e| NovaError::Restore(format!("Failed to read root directory: {}", e)))?
        {
            let device_dir = device_entry
                .map_err(|e| NovaError::Restore(format!("Failed to read device entry: {}", e)))?
                .path();

            if !device_dir.is_dir() {
                continue;
            }

            // Check each timestamp directory in this device folder
            for timestamp_entry in fs::read_dir(&device_dir)
                .map_err(|e| NovaError::Restore(format!("Failed to read device directory: {}", e)))?
            {
                let timestamp_dir = timestamp_entry
                    .map_err(|e| NovaError::Restore(format!("Failed to read timestamp entry: {}", e)))?
                    .path();

                if !timestamp_dir.is_dir() {
                    continue;
                }

                // Check if this directory contains the backup we're looking for
                let manifest_path = timestamp_dir.join("manifest.yaml");
                if manifest_path.exists() {
                    if let Ok(manifest) = self.load_manifest(&timestamp_dir) {
                        if manifest.id == backup_id {
                            return Ok(timestamp_dir);
                        }
                    }
                }
            }
        }

        Err(NovaError::Restore(format!("Backup with ID {} not found", backup_id)))
    }

    /// Load manifest from backup directory
    fn load_manifest(&self, backup_dir: &Path) -> Result<BackupManifest> {
        let manifest_path = backup_dir.join("manifest.yaml");
        
        if !manifest_path.exists() {
            return Err(NovaError::Restore("Manifest file not found".to_string()));
        }

        let manifest_content = fs::read_to_string(&manifest_path)
            .map_err(|e| NovaError::Restore(format!("Failed to read manifest: {}", e)))?;

        BackupManifest::from_yaml(&manifest_content)
    }

    /// Restore files to local directory
    async fn restore_files(&self, backup_dir: &Path, target_dir: &Path, manifest: &BackupManifest) -> Result<RestoreStats> {
        let files_dir = backup_dir.join("files");
        let mut stats = RestoreStats::default();

        for file_entry in &manifest.files {
            // Only restore successfully backed up files
            if file_entry.status != BackupStatus::Success {
                stats.files_skipped += 1;
                continue;
            }

            let source_path = files_dir.join(&file_entry.rel_dst);
            let target_path = target_dir.join(&file_entry.rel_dst);

            match self.restore_single_file(&source_path, &target_path, file_entry.mtime.as_deref()).await {
                Ok(()) => {
                    stats.files_success += 1;
                    debug!("Restored: {}", file_entry.path);
                }
                Err(e) => {
                    error!("Failed to restore {}: {}", file_entry.path, e);
                    stats.files_failed += 1;
                }
            }
        }

        stats.total_files = manifest.files.len();
        Ok(stats)
    }

    /// Restore files directly to Android device
    async fn restore_files_to_device(&self, backup_dir: &Path, target_serial: &str, manifest: &BackupManifest) -> Result<RestoreStats> {
        let files_dir = backup_dir.join("files");
        let mut stats = RestoreStats::default();

        for file_entry in &manifest.files {
            // Only restore successfully backed up files
            if file_entry.status != BackupStatus::Success {
                stats.files_skipped += 1;
                continue;
            }

            let source_path = files_dir.join(&file_entry.rel_dst);
            
            match self.restore_file_to_device(&source_path, &file_entry.path, target_serial).await {
                Ok(()) => {
                    stats.files_success += 1;
                    debug!("Restored to device: {}", file_entry.path);
                }
                Err(e) => {
                    error!("Failed to restore to device {}: {}", file_entry.path, e);
                    stats.files_failed += 1;
                }
            }
        }

        stats.total_files = manifest.files.len();
        Ok(stats)
    }

    /// Restore a single file to local directory
    async fn restore_single_file(&self, source_path: &Path, target_path: &Path, mtime: Option<&str>) -> Result<()> {
        if !source_path.exists() {
            return Err(NovaError::Restore(format!("Source file not found: {}", source_path.display())));
        }

        // Create parent directories
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| NovaError::Restore(format!("Failed to create parent directory: {}", e)))?;
        }

        // Copy file
        fs::copy(source_path, target_path)
            .map_err(|e| NovaError::Restore(format!("Failed to copy file: {}", e)))?;

        // Restore mtime if available
        if let Some(mtime_str) = mtime {
            if let Err(e) = self.set_file_mtime(target_path, mtime_str) {
                warn!("Failed to restore mtime for {}: {}", target_path.display(), e);
            }
        }

        Ok(())
    }

    /// Restore a single file to Android device
    async fn restore_file_to_device(&self, source_path: &Path, target_device_path: &str, serial: &str) -> Result<()> {
        if !source_path.exists() {
            return Err(NovaError::Restore(format!("Source file not found: {}", source_path.display())));
        }

        // Create parent directory on device if needed
        if let Some(parent) = Path::new(target_device_path).parent() {
            let mkdir_cmd = format!("mkdir -p '{}'", parent.display());
            self.adb.shell(serial, &mkdir_cmd)
                .map_err(|e| NovaError::Restore(format!("Failed to create device directory: {}", e)))?;
        }

        // Push file to device
        self.adb.push(serial, source_path.to_string_lossy().as_ref(), target_device_path)
            .map_err(|e| NovaError::Restore(format!("Failed to push file to device: {}", e)))?;

        Ok(())
    }

    /// Set file modification time
    fn set_file_mtime(&self, file_path: &Path, mtime_str: &str) -> Result<()> {
        // Parse mtime string (format: "YYYY-MM-DD HH:MM:SS")
        let dt = chrono::NaiveDateTime::parse_from_str(mtime_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| NovaError::Restore(format!("Failed to parse mtime: {}", e)))?;

        let _timestamp = dt.and_utc().timestamp();
        
        // Set file times using system command (cross-platform)
        #[cfg(unix)]
        {
            use std::process::Command;
            let touch_result = Command::new("touch")
                .args(["-t", &format!("{}", dt.format("%Y%m%d%H%M.%S")), file_path.to_string_lossy().as_ref()])
                .output();

            if let Err(e) = touch_result {
                return Err(NovaError::Restore(format!("Failed to set mtime: {}", e)));
            }
        }

        #[cfg(windows)]
        {
            // For Windows, we could use SetFileTime API, but for simplicity we'll skip it
            warn!("Setting file mtime not implemented on Windows");
        }

        Ok(())
    }

    /// List available backups in root directory
    pub fn list_backups(&self, root_dir: &Path) -> Result<Vec<BackupInfo>> {
        let mut backups = Vec::new();

        for device_entry in fs::read_dir(root_dir)
            .map_err(|e| NovaError::Restore(format!("Failed to read root directory: {}", e)))?
        {
            let device_dir = device_entry
                .map_err(|e| NovaError::Restore(format!("Failed to read device entry: {}", e)))?
                .path();

            if !device_dir.is_dir() {
                continue;
            }

            let device_serial = device_dir.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Check each timestamp directory
            for timestamp_entry in fs::read_dir(&device_dir)
                .map_err(|e| NovaError::Restore(format!("Failed to read device directory: {}", e)))?
            {
                let timestamp_dir = timestamp_entry
                    .map_err(|e| NovaError::Restore(format!("Failed to read timestamp entry: {}", e)))?
                    .path();

                if !timestamp_dir.is_dir() {
                    continue;
                }

                // Try to load manifest
                if let Ok(manifest) = self.load_manifest(&timestamp_dir) {
                    let stats = manifest.get_stats();
                    
                    backups.push(BackupInfo {
                        id: manifest.id,
                        device_serial: device_serial.clone(),
                        device_model: format!("{} {}", manifest.device.brand, manifest.device.model),
                        created_at: manifest.created_at,
                        total_files: stats.total_files(),
                        success_files: stats.files_success,
                        total_size: stats.total_size,
                        backup_path: timestamp_dir,
                    });
                }
            }
        }

        // Sort by creation time (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }
}

#[derive(Debug, Default)]
pub struct RestoreStats {
    pub files_success: usize,
    pub files_failed: usize,
    pub files_skipped: usize,
    pub total_files: usize,
}

#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub id: String,
    pub device_serial: String,
    pub device_model: String,
    pub created_at: String,
    pub total_files: usize,
    pub success_files: usize,
    pub total_size: u64,
    pub backup_path: PathBuf,
}