//! Report command implementation for viewing backup reports.

use clap::Args;
use std::path::PathBuf;
use crate::Result;
use serde_json;
use tokio::fs;

/// Arguments for the report command
#[derive(Args)]
pub struct ReportArgs {
    /// Backup output directory containing reports
    #[arg(short, long)]
    pub backup_dir: PathBuf,

    /// List all available reports
    #[arg(short, long)]
    pub list: bool,

    /// Show detailed report for specific manifest ID
    #[arg(long)]
    pub manifest_id: Option<String>,

    /// Output format (json, summary)
    #[arg(long, default_value = "summary")]
    pub format: String,
}

/// Run the report command
pub async fn run(args: ReportArgs) -> Result<()> {
    let reports_dir = args.backup_dir.join("reports");

    if !reports_dir.exists() {
        println!("No reports directory found at: {}", reports_dir.display());
        return Ok(());
    }

    if args.list {
        list_reports(&reports_dir).await?;
        return Ok(());
    }

    if let Some(manifest_id) = args.manifest_id {
        show_specific_report(&reports_dir, &manifest_id, &args.format).await?;
    } else {
        show_latest_report(&reports_dir, &args.format).await?;
    }

    Ok(())
}

async fn list_reports(reports_dir: &PathBuf) -> Result<()> {
    println!("Available backup reports:\n");

    let mut entries = fs::read_dir(reports_dir).await?;
    let mut reports = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                if filename.starts_with("report-") {
                    let manifest_id = filename.strip_prefix("report-").unwrap();
                    
                    // Try to read the report to get metadata
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(report) = serde_json::from_str::<crate::backup::report::BackupReport>(&content) {
                            reports.push((manifest_id.to_string(), report));
                        }
                    }
                }
            }
        }
    }

    if reports.is_empty() {
        println!("No backup reports found.");
        return Ok(());
    }

    // Sort by creation date (most recent first)
    reports.sort_by(|a, b| b.1.completed_at.cmp(&a.1.completed_at));

    println!("{:<38} {:<20} {:<15} {:<10} {:<15}", "Manifest ID", "Completed", "Label", "Files", "Size");
    println!("{}", "-".repeat(100));

    for (manifest_id, report) in reports {
        println!(
            "{:<38} {:<20} {:<15} {:<10} {:<15}",
            manifest_id,
            report.completed_at.format("%Y-%m-%d %H:%M:%S"),
            report.label,
            report.total_files,
            format_bytes(report.total_size)
        );
    }

    Ok(())
}

async fn show_specific_report(reports_dir: &PathBuf, manifest_id: &str, format: &str) -> Result<()> {
    let report_path = reports_dir.join(format!("report-{}.json", manifest_id));

    if !report_path.exists() {
        println!("Report not found for manifest ID: {}", manifest_id);
        return Ok(());
    }

    let content = fs::read_to_string(&report_path).await?;
    let report: crate::backup::report::BackupReport = serde_json::from_str(&content)?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "summary" | _ => {
            print_report_summary(&report);
        }
    }

    Ok(())
}

async fn show_latest_report(reports_dir: &PathBuf, format: &str) -> Result<()> {
    let mut entries = fs::read_dir(reports_dir).await?;
    let mut latest_report = None;
    let mut latest_time = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                if filename.starts_with("report-") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(report) = serde_json::from_str::<crate::backup::report::BackupReport>(&content) {
                            if latest_time.is_none() || report.completed_at > latest_time.unwrap() {
                                latest_time = Some(report.completed_at);
                                latest_report = Some(report);
                            }
                        }
                    }
                }
            }
        }
    }

    match latest_report {
        Some(report) => {
            match format {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                "summary" | _ => {
                    print_report_summary(&report);
                }
            }
        }
        None => {
            println!("No backup reports found.");
        }
    }

    Ok(())
}

fn print_report_summary(report: &crate::backup::report::BackupReport) {
    println!("Backup Report Summary");
    println!("====================\n");
    println!("Manifest ID:     {}", report.manifest_id);
    println!("Label:           {}", report.label);
    println!("Completed:       {}", report.completed_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Source Path:     {}", report.source_path.display());
    println!("Total Files:     {}", report.total_files);
    println!("Total Size:      {}", format_bytes(report.total_size));
    println!("Total Chunks:    {}", report.total_chunks);
    println!("Compression:     {:.1}%", report.compression_ratio * 100.0);
    println!("Storage Efficiency: {:.1}%", report.storage_efficiency * 100.0);

    println!("\nFile Types Analysis:");
    // Group files by extension
    let mut extensions = std::collections::HashMap::new();
    for file in &report.files {
        let ext = file.path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("(no extension)")
            .to_lowercase();
        let entry = extensions.entry(ext).or_insert((0, 0u64));
        entry.0 += 1;
        entry.1 += file.size;
    }

    let mut ext_vec: Vec<_> = extensions.into_iter().collect();
    ext_vec.sort_by(|a, b| b.1.1.cmp(&a.1.1)); // Sort by total size

    for (ext, (count, size)) in ext_vec.iter().take(10) {
        println!("  {:<15} {:>8} files  {:>12}", ext, count, format_bytes(*size));
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}