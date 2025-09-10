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

use clap::{Parser, Subcommand};
use nova_adb::AdbClient;
use nova_backup::{BackupPlanner, FileScanner, ScanOptions};
use nova_formats::{
    contacts::AndroidContactSource, ContactExporter, ContactSource, CsvExporter, VcfExporter,
};
use std::path::PathBuf;
use tracing::info;

#[derive(Parser)]
#[command(name = "nova-cli")]
#[command(about = "NovaPcSuite - Linux-first Android device management")]
#[command(version)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List connected Android devices
    Devices,

    /// Scan device files and directories
    Scan {
        #[arg(long)]
        json_output: Option<PathBuf>,

        #[arg(long, default_value = "4")]
        max_parallel: usize,

        #[arg(long)]
        compute_hashes: bool,

        #[arg(long)]
        device_serial: Option<String>,
    },

    /// Create backup plan
    Plan {
        #[arg(long, action = clap::ArgAction::Append)]
        include: Vec<String>,

        #[arg(long)]
        out: PathBuf,

        #[arg(long)]
        device_serial: Option<String>,

        #[arg(long)]
        compression: bool,
    },

    /// Export contacts
    Contacts {
        #[command(subcommand)]
        action: ContactCommands,
    },
}

#[derive(Subcommand)]
enum ContactCommands {
    Export {
        #[arg(long, value_enum)]
        format: ContactFormat,

        #[arg(long)]
        out: PathBuf,

        #[arg(long)]
        device_serial: Option<String>,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum ContactFormat {
    Vcf,
    Csv,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    nova_core::logging::init_logging(cli.verbose)?;

    info!("NovaPcSuite CLI starting");

    match cli.command {
        Commands::Devices => {
            handle_devices().await?;
        }
        Commands::Scan {
            json_output,
            max_parallel,
            compute_hashes,
            device_serial,
        } => {
            handle_scan(json_output, max_parallel, compute_hashes, device_serial).await?;
        }
        Commands::Plan {
            include,
            out,
            device_serial,
            compression,
        } => {
            handle_plan(include, out, device_serial, compression).await?;
        }
        Commands::Contacts { action } => match action {
            ContactCommands::Export {
                format,
                out,
                device_serial,
            } => {
                handle_contacts_export(format, out, device_serial).await?;
            }
        },
    }

    Ok(())
}

async fn handle_devices() -> anyhow::Result<()> {
    let adb_client = AdbClient::new();
    let devices = adb_client.list_devices().await?;

    if devices.is_empty() {
        println!("No devices connected");
        return Ok(());
    }

    println!("Connected devices:");
    for device in devices {
        println!("  Serial: {}", device.info.serial);
        println!(
            "  Model: {} {}",
            device.info.manufacturer, device.info.model
        );
        println!("  Android: {}", device.info.android_version);
        println!(
            "  Root: {}",
            if device.is_root_available() {
                "Yes"
            } else {
                "No"
            }
        );
        println!(
            "  Can backup apps: {}",
            if device.can_backup_apps() {
                "Yes"
            } else {
                "No"
            }
        );
        println!();
    }

    Ok(())
}

async fn handle_scan(
    json_output: Option<PathBuf>,
    max_parallel: usize,
    compute_hashes: bool,
    device_serial: Option<String>,
) -> anyhow::Result<()> {
    let adb_client = AdbClient::new();
    let devices = adb_client.list_devices().await?;

    let device = if let Some(serial) = device_serial {
        devices
            .into_iter()
            .find(|d| d.info.serial == serial)
            .ok_or_else(|| anyhow::anyhow!("Device with serial {} not found", serial))?
    } else {
        devices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No devices connected"))?
    };

    println!(
        "Scanning device: {} {}",
        device.info.manufacturer, device.info.model
    );

    let scanner = FileScanner::new();
    let options = ScanOptions {
        include_paths: vec![
            "/storage/emulated/0/DCIM".to_string(),
            "/storage/emulated/0/Pictures".to_string(),
            "/storage/emulated/0/Movies".to_string(),
            "/storage/emulated/0/Music".to_string(),
            "/storage/emulated/0/Documents".to_string(),
        ],
        exclude_patterns: vec![".thumbnail".to_string(), ".cache".to_string()],
        max_depth: Some(10),
        follow_symlinks: false,
        compute_hashes,
        max_parallel,
    };

    let result = scanner.scan_device(&device, &options, None).await?;

    println!("Scan complete:");
    println!("  Files found: {}", result.summary.total_files);
    println!("  Total size: {} bytes", result.summary.total_size);
    println!("  Categories:");
    for (category, count) in &result.summary.categories {
        println!("    {:?}: {} files", category, count);
    }

    if let Some(output_path) = json_output {
        let json = serde_json::to_string_pretty(&result)?;
        std::fs::write(&output_path, json)?;
        println!("  Output saved to: {:?}", output_path);
    }

    Ok(())
}

async fn handle_plan(
    include_paths: Vec<String>,
    output_path: PathBuf,
    device_serial: Option<String>,
    compression: bool,
) -> anyhow::Result<()> {
    let adb_client = AdbClient::new();
    let devices = adb_client.list_devices().await?;

    let device = if let Some(serial) = device_serial {
        devices
            .into_iter()
            .find(|d| d.info.serial == serial)
            .ok_or_else(|| anyhow::anyhow!("Device with serial {} not found", serial))?
    } else {
        devices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No devices connected"))?
    };

    println!(
        "Creating backup plan for device: {} {}",
        device.info.manufacturer, device.info.model
    );

    // First scan the device
    let scanner = FileScanner::new();
    let scan_options = ScanOptions {
        include_paths: include_paths.clone(),
        exclude_patterns: vec![],
        max_depth: Some(10),
        follow_symlinks: false,
        compute_hashes: false,
        max_parallel: 4,
    };

    let scan_result = scanner.scan_device(&device, &scan_options, None).await?;

    // Create backup plan
    let planner = BackupPlanner::new();
    let plan_options = nova_backup::planner::BackupPlanOptions {
        compression_enabled: compression,
        prioritize_media: true,
        min_file_size: 1024, // 1KB minimum
        exclude_patterns: vec![],
    };

    let plan = planner
        .create_plan(
            &device.info.serial,
            &scan_result.files,
            &include_paths,
            &plan_options,
        )
        .await?;

    planner.save_plan(&plan, &output_path)?;

    println!("Backup plan created:");
    println!("  Files to backup: {}", plan.metadata.total_files);
    println!("  Total size: {} bytes", plan.metadata.total_size);
    if let Some(compressed_size) = plan.metadata.estimated_compressed_size {
        println!("  Estimated compressed size: {} bytes", compressed_size);
    }
    println!("  Plan saved to: {:?}", output_path);

    Ok(())
}

async fn handle_contacts_export(
    format: ContactFormat,
    output_path: PathBuf,
    device_serial: Option<String>,
) -> anyhow::Result<()> {
    let adb_client = AdbClient::new();
    let devices = adb_client.list_devices().await?;

    let device = if let Some(serial) = device_serial {
        devices
            .into_iter()
            .find(|d| d.info.serial == serial)
            .ok_or_else(|| anyhow::anyhow!("Device with serial {} not found", serial))?
    } else {
        devices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No devices connected"))?
    };

    println!(
        "Exporting contacts from device: {} {}",
        device.info.manufacturer, device.info.model
    );

    let contact_source = AndroidContactSource::new();
    let contacts = contact_source.fetch_contacts(&device).await?;

    println!("Found {} contacts", contacts.len());

    match format {
        ContactFormat::Vcf => {
            let exporter = VcfExporter::new();
            exporter.export_contacts(&contacts, &output_path).await?;
            println!("Contacts exported to VCF format: {:?}", output_path);
        }
        ContactFormat::Csv => {
            let exporter = CsvExporter::new();
            exporter.export_contacts(&contacts, &output_path).await?;
            println!("Contacts exported to CSV format: {:?}", output_path);
        }
    }

    Ok(())
}
