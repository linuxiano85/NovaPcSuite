//! Backup command implementation.

use clap::Args;
use std::path::PathBuf;
use crate::backup::{BackupEngine, LocalFsSource};
use crate::backup::report::ReportGenerator;
use crate::Result;

/// Arguments for the backup command
#[derive(Args)]
pub struct BackupArgs {
    /// Source directory to backup
    #[arg(short, long)]
    pub source: PathBuf,

    /// Output directory for backup data
    #[arg(short, long)]
    pub output: PathBuf,

    /// Label for this backup
    #[arg(short, long, default_value = "backup")]
    pub label: String,

    /// Chunk size in bytes (default: 2 MiB)
    #[arg(long, default_value = "2097152")]
    pub chunk_size: usize,

    /// Generate HTML report after backup
    #[arg(long)]
    pub generate_report: bool,
}

/// Run the backup command
pub async fn run(args: BackupArgs) -> Result<()> {
    println!("Starting backup: {} -> {}", args.source.display(), args.output.display());

    // Create backup engine with custom chunk size
    let engine = BackupEngine::new(&args.output).with_chunk_size(args.chunk_size);

    // Create source
    let source = LocalFsSource::new(&args.source);

    // Perform backup
    let manifest = engine.create_snapshot(&source, &args.label).await?;

    println!("Backup completed successfully!");
    println!("  Manifest ID: {}", manifest.id());
    println!("  Files backed up: {}", manifest.files.len());
    println!("  Total chunks: {}", manifest.chunk_count);
    println!("  Total size: {} bytes", manifest.total_size);

    // Generate report if requested
    if args.generate_report {
        println!("Generating backup report...");
        let report_generator = ReportGenerator::new(&args.output);
        let report = report_generator.generate_report(&manifest).await?;
        println!("Report generated: JSON and HTML formats available");
        println!("  Compression ratio: {:.1}%", report.compression_ratio * 100.0);
        println!("  Storage efficiency: {:.1}%", report.storage_efficiency * 100.0);
    }

    Ok(())
}