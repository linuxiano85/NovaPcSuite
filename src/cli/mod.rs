//! Command-line interface for NovaPcSuite.
//! 
//! This module provides a comprehensive CLI using clap for backup operations,
//! scanning, reporting, and manifest management.

use clap::{Parser, Subcommand};

pub mod backup;
pub mod scan;
pub mod report;
pub mod manifest;
pub mod devices;

/// NovaPcSuite - Advanced PC backup and maintenance suite
#[derive(Parser)]
#[command(name = "nova-pc-suite")]
#[command(about = "Advanced PC backup and maintenance suite with chunked Merkle-based snapshots")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands
#[derive(Subcommand)]
pub enum Commands {
    /// Create a backup snapshot
    Backup(backup::BackupArgs),
    /// Scan files for analysis
    Scan(scan::ScanArgs),
    /// Generate and view backup reports
    Report(report::ReportArgs),
    /// Manage backup manifests
    Manifest(manifest::ManifestArgs),
    /// Manage connected devices (future)
    Devices(devices::DevicesArgs),
}