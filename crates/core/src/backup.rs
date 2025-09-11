use crate::{
    adb::AdbWrapper,
    device::DeviceManager, 
    scanner::FileScanner,
    manifest::{BackupManifest, BackupStatus, ExportStatus, ApkEntry},
    NovaError, Result
};
use sha2::{Sha256, Digest};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn, error};

pub struct BackupExecutor {
    adb: AdbWrapper,
    device_manager: DeviceManager,
    scanner: FileScanner,
}

impl BackupExecutor {
    pub fn new() -> Self {
        Self {
            adb: AdbWrapper::new(),
            device_manager: DeviceManager::new(),
            scanner: FileScanner::new(),
        }
    }

    pub fn with_custom_scanner(scanner: FileScanner) -> Self {
        Self {
            adb: AdbWrapper::new(),
            device_manager: DeviceManager::new(),
            scanner,
        }
    }

    /// Execute full backup process
    pub async fn backup_device(&self, serial: &str, output_dir: &Path, incremental: bool) -> Result<BackupManifest> {
        info!("Starting backup for device {}", serial);

        // Get device info
        let device_info = self.device_manager.get_device_info(serial)?;
        info!("Device: {} {} (Android {})", device_info.brand, device_info.model, device_info.android_version);

        // Create backup directory structure
        let backup_dir = self.create_backup_directory(output_dir, &device_info.serial)?;
        info!("Backup directory: {}", backup_dir.display());

        // Initialize manifest
        let mut manifest = BackupManifest::new(device_info, incremental);

        // Scan files
        info!("Scanning device for files...");
        let scanned_files = self.scanner.scan_device(serial)?;
        info!("Found {} files to backup", scanned_files.len());
        manifest.add_files(scanned_files);

        // Backup files
        info!("Starting file backup...");
        self.backup_files(serial, &backup_dir, &mut manifest).await?;

        // Export contacts (stub)
        info!("Exporting contacts...");
        self.export_contacts_stub(&backup_dir, &mut manifest)?;

        // Export logs (stub)
        info!("Exporting logs...");
        self.export_logs_stub(&backup_dir, &mut manifest)?;

        // Detect recordings
        info!("Detecting recordings...");
        let recordings = self.scanner.detect_recordings(serial)?;
        manifest.set_recordings_info(ExportStatus::Success, recordings);

        // Save manifest
        self.save_manifest(&backup_dir, &manifest)?;

        let stats = manifest.get_stats();
        info!("Backup completed: {}/{} files successful ({:.1}%)", 
              stats.files_success, stats.total_files(), stats.success_rate());

        Ok(manifest)
    }

    /// Backup user APKs
    pub async fn backup_apks(&self, serial: &str, output_dir: &Path) -> Result<Vec<ApkEntry>> {
        info!("Starting APK backup for device {}", serial);

        let device_info = self.device_manager.get_device_info(serial)?;
        let backup_dir = self.create_backup_directory(output_dir, &device_info.serial)?;
        let apk_dir = backup_dir.join("apks");
        fs::create_dir_all(&apk_dir)?;

        // Get user packages
        let packages = self.adb.list_packages(serial, true)?;
        info!("Found {} user packages", packages.len());

        let mut apk_entries = Vec::new();
        let total_packages = packages.len();

        for package in packages {
            match self.backup_single_apk(serial, &package, &apk_dir).await {
                Ok(apk_entry) => {
                    info!("Backed up APK: {}", package);
                    apk_entries.push(apk_entry);
                }
                Err(e) => {
                    warn!("Failed to backup APK {}: {}", package, e);
                }
            }
        }

        info!("APK backup completed: {}/{} successful", apk_entries.len(), total_packages);
        Ok(apk_entries)
    }

    /// Backup a single APK
    async fn backup_single_apk(&self, serial: &str, package: &str, apk_dir: &Path) -> Result<ApkEntry> {
        // Get APK path
        let source_path = self.adb.get_package_path(serial, package)?;
        
        // Extract APK filename
        let apk_filename = Path::new(&source_path)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("base.apk"))
            .to_string_lossy();
        
        let local_path = apk_dir.join(format!("{}_{}", package, apk_filename));

        // Pull APK
        self.adb.pull(serial, &source_path, local_path.to_string_lossy().as_ref())?;

        // Calculate hash
        let sha256 = self.calculate_file_hash(&local_path)?;

        // TODO: Extract version info from APK (future enhancement)
        Ok(ApkEntry {
            package: package.to_string(),
            version_name: None,
            version_code: None,
            source_path,
            sha256: Some(sha256),
        })
    }

    /// Create backup directory structure
    fn create_backup_directory(&self, output_dir: &Path, serial: &str) -> Result<PathBuf> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir = output_dir.join(serial).join(timestamp.to_string());
        
        fs::create_dir_all(&backup_dir)
            .map_err(|e| NovaError::Backup(format!("Failed to create backup directory: {}", e)))?;
        
        Ok(backup_dir)
    }

    /// Backup all files from manifest
    async fn backup_files(&self, serial: &str, backup_dir: &Path, manifest: &mut BackupManifest) -> Result<()> {
        let files_dir = backup_dir.join("files");
        fs::create_dir_all(&files_dir)?;

        for i in 0..manifest.files.len() {
            let file_path = manifest.files[i].path.clone();
            let rel_dst = manifest.files[i].rel_dst.clone();
            
            match self.backup_single_file(serial, &file_path, &files_dir, &rel_dst).await {
                Ok(sha256) => {
                    manifest.update_file_status(&file_path, BackupStatus::Success, Some(sha256));
                    debug!("Backed up: {}", file_path);
                }
                Err(e) => {
                    error!("Failed to backup {}: {}", file_path, e);
                    manifest.update_file_status(&file_path, BackupStatus::Failed, None);
                }
            }
        }

        Ok(())
    }

    /// Backup a single file
    async fn backup_single_file(&self, serial: &str, remote_path: &str, files_dir: &Path, rel_dst: &str) -> Result<String> {
        let local_path = files_dir.join(rel_dst);
        
        // Create parent directories if needed
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Pull file
        self.adb.pull(serial, remote_path, local_path.to_string_lossy().as_ref())?;

        // Calculate hash
        self.calculate_file_hash(&local_path)
    }

    /// Calculate SHA256 hash of a file
    fn calculate_file_hash(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path)
            .map_err(|e| NovaError::FileOperation(format!("Failed to open file for hashing: {}", e)))?;
        
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer)
                .map_err(|e| NovaError::FileOperation(format!("Failed to read file for hashing: {}", e)))?;
            
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Export contacts (stub implementation)
    fn export_contacts_stub(&self, backup_dir: &Path, manifest: &mut BackupManifest) -> Result<()> {
        let contacts_dir = backup_dir.join("contacts");
        fs::create_dir_all(&contacts_dir)?;

        // Create empty stub files
        let vcf_path = contacts_dir.join("contacts.vcf");
        let csv_path = contacts_dir.join("contacts.csv");
        let json_path = contacts_dir.join("contacts.json");

        fs::write(&vcf_path, "# No contacts exported - permissions required\n")?;
        fs::write(&csv_path, "# No contacts exported - permissions required\n")?;
        fs::write(&json_path, r#"{"error": "No contacts exported - permissions required"}"#)?;

        manifest.set_contacts_info(
            ExportStatus::NoPermissions,
            Some((
                vcf_path.to_string_lossy().to_string(),
                csv_path.to_string_lossy().to_string(),
                json_path.to_string_lossy().to_string(),
            ))
        );

        Ok(())
    }

    /// Export logs (stub implementation)
    fn export_logs_stub(&self, backup_dir: &Path, manifest: &mut BackupManifest) -> Result<()> {
        let logs_dir = backup_dir.join("logs");
        fs::create_dir_all(&logs_dir)?;

        // Create empty stub files
        let calls_path = logs_dir.join("call_log.json");
        let sms_path = logs_dir.join("sms.json");

        fs::write(&calls_path, r#"{"error": "No call log exported - permissions required"}"#)?;
        fs::write(&sms_path, r#"{"error": "No SMS exported - permissions required"}"#)?;

        manifest.set_logs_info(
            ExportStatus::NoPermissions,
            Some((
                calls_path.to_string_lossy().to_string(),
                sms_path.to_string_lossy().to_string(),
            ))
        );

        Ok(())
    }

    /// Save manifest to files
    fn save_manifest(&self, backup_dir: &Path, manifest: &BackupManifest) -> Result<()> {
        let yaml_path = backup_dir.join("manifest.yaml");
        let json_path = backup_dir.join("manifest.json");

        fs::write(&yaml_path, manifest.to_yaml()?)?;
        fs::write(&json_path, manifest.to_json()?)?;

        info!("Manifest saved to {} and {}", yaml_path.display(), json_path.display());
        Ok(())
    }
}