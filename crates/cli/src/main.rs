use clap::{Parser, Subcommand};
use novapcsuite_core::{
    backup::BackupExecutor,
    device::DeviceManager,
    restore::RestoreExecutor,
    Result,
};
use std::path::PathBuf;
use tracing::{info, error};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "novapcsuite")]
#[command(about = "Android device management and backup tool")]
#[command(version, author)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Device operations
    Device {
        #[command(subcommand)]
        command: DeviceCommands,
    },
    /// Backup operations
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },
    /// Application (APK) operations
    Apps {
        #[command(subcommand)]
        command: AppsCommands,
    },
    /// Restore operations
    Restore {
        /// Backup ID to restore
        backup_id: String,
        /// Root directory containing backups
        #[arg(long, default_value = "./backups")]
        root: PathBuf,
        /// Target directory for restore
        #[arg(long, default_value = "./restore_out")]
        target: PathBuf,
    },
}

#[derive(Subcommand)]
enum DeviceCommands {
    /// Show device information
    Info {
        /// Device serial (auto-detect if not specified)
        #[arg(short, long)]
        serial: Option<String>,
    },
    /// Show OEM/bootloader information
    OemInfo {
        /// Device serial (auto-detect if not specified)
        #[arg(short, long)]
        serial: Option<String>,
    },
}

#[derive(Subcommand)]
enum BackupCommands {
    /// Run full backup
    Run {
        /// Output directory for backups
        #[arg(long, default_value = "./backups")]
        output: PathBuf,
        /// Device serial (auto-detect if not specified)
        #[arg(short, long)]
        serial: Option<String>,
        /// Enable incremental backup
        #[arg(long)]
        incremental: bool,
    },
    /// List available backups
    List {
        /// Root directory containing backups
        #[arg(long, default_value = "./backups")]
        root: PathBuf,
    },
    /// Show backup details
    Show {
        /// Backup ID to show
        backup_id: String,
        /// Root directory containing backups
        #[arg(long, default_value = "./backups")]
        root: PathBuf,
    },
}

#[derive(Subcommand)]
enum AppsCommands {
    /// Backup user APKs
    Backup {
        /// Root directory for backups
        #[arg(long, default_value = "./backups")]
        root: PathBuf,
        /// Device serial (auto-detect if not specified)
        #[arg(short, long)]
        serial: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Device { command } => handle_device_command(command).await,
        Commands::Backup { command } => handle_backup_command(command).await,
        Commands::Apps { command } => handle_apps_command(command).await,
        Commands::Restore { backup_id, root, target } => handle_restore_command(backup_id, root, target).await,
    }
}

async fn handle_device_command(command: DeviceCommands) -> Result<()> {
    let device_manager = DeviceManager::new();

    match command {
        DeviceCommands::Info { serial } => {
            let serial = get_device_serial(serial, &device_manager)?;
            let device_info = device_manager.get_device_info(&serial)?;
            
            println!("Device Information:");
            println!("==================");
            println!("Serial:          {}", device_info.serial);
            println!("Model:           {}", device_info.model);
            println!("Brand:           {}", device_info.brand);
            println!("Manufacturer:    {}", device_info.manufacturer);
            println!("Product:         {}", device_info.product);
            println!("Android Version: {}", device_info.android_version);
            println!("SDK Level:       {}", device_info.sdk);
        }
        DeviceCommands::OemInfo { serial } => {
            let serial = get_device_serial(serial, &device_manager)?;
            let bootloader_info = device_manager.get_bootloader_info(&serial)?;
            
            println!("OEM/Bootloader Information:");
            println!("===========================");
            
            if let Some(locked) = bootloader_info.locked {
                println!("Bootloader Locked: {}", if locked { "Yes" } else { "No" });
            } else {
                println!("Bootloader Status: Unknown");
            }
            
            if let Some(ref state) = bootloader_info.verified_boot_state {
                println!("Verified Boot State: {}", state);
            }
            
            if let Some(ref guidance) = bootloader_info.unlock_guidance {
                println!("\nUnlock Guidance:");
                println!("================");
                println!("{}", guidance);
            }
        }
    }

    Ok(())
}

async fn handle_backup_command(command: BackupCommands) -> Result<()> {
    match command {
        BackupCommands::Run { output, serial, incremental } => {
            let device_manager = DeviceManager::new();
            let serial = get_device_serial(serial, &device_manager)?;
            
            info!("Starting backup for device {}", serial);
            
            let backup_executor = BackupExecutor::new();
            let manifest = backup_executor.backup_device(&serial, &output, incremental).await?;
            
            let stats = manifest.get_stats();
            
            println!("Backup completed successfully!");
            println!("==============================");
            println!("Backup ID:       {}", manifest.id);
            println!("Device:          {} {} ({})", manifest.device.brand, manifest.device.model, manifest.device.serial);
            println!("Files backed up: {}/{} ({:.1}%)", stats.files_success, stats.total_files(), stats.success_rate());
            println!("Total size:      {} bytes", stats.total_size);
            println!("APKs:           {} packages", stats.apks_count);
            
            if stats.files_failed > 0 {
                println!("Failed files:    {}", stats.files_failed);
            }
        }
        BackupCommands::List { root } => {
            let restore_executor = RestoreExecutor::new();
            let backups = restore_executor.list_backups(&root)?;
            
            if backups.is_empty() {
                println!("No backups found in {}", root.display());
                return Ok(());
            }
            
            println!("Available Backups:");
            println!("==================");
            println!("{:<36} {:<15} {:<25} {:<20} {:<10} {:<10}", 
                     "ID", "Device", "Model", "Created", "Files", "Size");
            println!("{}", "-".repeat(120));
            
            for backup in backups {
                let size_mb = backup.total_size as f64 / 1024.0 / 1024.0;
                println!("{:<36} {:<15} {:<25} {:<20} {:<10} {:<7.1} MB", 
                         backup.id[..8].to_string() + "...",
                         backup.device_serial,
                         backup.device_model,
                         backup.created_at[..19].replace('T', " "),
                         format!("{}/{}", backup.success_files, backup.total_files),
                         size_mb);
            }
        }
        BackupCommands::Show { backup_id, root } => {
            let restore_executor = RestoreExecutor::new();
            let backups = restore_executor.list_backups(&root)?;
            
            if let Some(backup) = backups.iter().find(|b| b.id.starts_with(&backup_id)) {
                // Load and display full manifest
                let manifest_path = backup.backup_path.join("manifest.yaml");
                let manifest_content = std::fs::read_to_string(&manifest_path)?;
                
                println!("Backup Manifest:");
                println!("================");
                println!("{}", manifest_content);
            } else {
                error!("Backup with ID {} not found", backup_id);
                return Err(novapcsuite_core::NovaError::Restore(format!("Backup not found: {}", backup_id)));
            }
        }
    }

    Ok(())
}

async fn handle_apps_command(command: AppsCommands) -> Result<()> {
    match command {
        AppsCommands::Backup { root, serial } => {
            let device_manager = DeviceManager::new();
            let serial = get_device_serial(serial, &device_manager)?;
            
            info!("Starting APK backup for device {}", serial);
            
            let backup_executor = BackupExecutor::new();
            let apk_entries = backup_executor.backup_apks(&serial, &root).await?;
            
            println!("APK backup completed!");
            println!("====================");
            println!("Device:     {}", serial);
            println!("APKs saved: {}", apk_entries.len());
            
            for apk in &apk_entries {
                println!("  - {} ({})", apk.package, apk.source_path);
            }
        }
    }

    Ok(())
}

async fn handle_restore_command(backup_id: String, root: PathBuf, target: PathBuf) -> Result<()> {
    info!("Starting restore of backup {} to {}", backup_id, target.display());
    
    let restore_executor = RestoreExecutor::new();
    let stats = restore_executor.restore_to_directory(&backup_id, &root, &target).await?;
    
    println!("Restore completed!");
    println!("==================");
    println!("Files restored: {}/{}", stats.files_success, stats.total_files);
    println!("Target directory: {}", target.display());
    
    if stats.files_failed > 0 {
        println!("Failed files: {}", stats.files_failed);
    }
    if stats.files_skipped > 0 {
        println!("Skipped files: {}", stats.files_skipped);
    }

    Ok(())
}

fn get_device_serial(serial: Option<String>, device_manager: &DeviceManager) -> Result<String> {
    if let Some(serial) = serial {
        Ok(serial)
    } else {
        device_manager.get_default_device()
    }
}