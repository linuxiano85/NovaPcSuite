//! # NovaPcSuite
//! 
//! Advanced PC backup and maintenance suite with chunked Merkle-based snapshots.
//! 
//! ## Features
//! 
//! - **Backup Engine**: Adaptive chunking with BLAKE3 hashing and Merkle trees
//! - **Deduplication**: Content-addressed storage with perceptual hashing for media
//! - **Plugin System**: WASM-based extensible architecture
//! - **Telephony Integration**: Remote notifications and companion app support
//! - **Scheduling**: Automated backup scheduling with systemd integration
//! - **Restore System**: Reliable data restoration from chunked snapshots
//! 
//! ## Quick Start
//! 
//! ```rust,no_run
//! use nova_pc_suite::backup::{BackupEngine, LocalFsSource};
//! use std::path::Path;
//! 
//! # #[tokio::main]
//! # async fn main() -> anyhow::Result<()> {
//! let engine = BackupEngine::new(Path::new("./backup-output"));
//! let source = LocalFsSource::new(Path::new("./my-data"));
//! 
//! let manifest = engine.create_snapshot(&source, "initial-backup").await?;
//! println!("Backup completed: {}", manifest.id());
//! # Ok(())
//! # }
//! ```

pub mod backup;
pub mod cli;
pub mod dedupe;

#[cfg(feature = "telephony")]
pub mod telephony;

pub mod plugins;
pub mod restore;
pub mod scheduler;

// Re-export commonly used types
pub use backup::{BackupEngine, LocalFsSource, Manifest};
pub use dedupe::DedupeEngine;

#[cfg(feature = "telephony")]
pub use telephony::TelephonyProvider;

/// Result type used throughout the library
pub type Result<T> = anyhow::Result<T>;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");