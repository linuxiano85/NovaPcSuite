//! Backup module providing the core backup engine functionality.
//! 
//! This module implements adaptive chunking, BLAKE3 hashing, Merkle tree construction,
//! and content-addressed storage for efficient backup operations.

pub mod nova_pc_suite_backup;
pub mod report;

// Re-export main types
pub use nova_pc_suite_backup::{BackupEngine, LocalFsSource, Manifest, ChunkInfo, BackupPlan, FileEntry};
pub use report::{ReportGenerator, BackupReport};