//! # NovaPcSuite
//! 
//! A comprehensive backup and restore system with chunk-based deduplication,
//! integrity verification, and data recovery capabilities.
//!
//! ## Features
//!
//! - **Chunk-based deduplication**: Efficient storage using content-addressed chunks
//! - **Integrity verification**: BLAKE3 hashing with Merkle tree validation
//! - **Restore engine**: Full file reconstruction with conflict resolution
//! - **Data recovery**: Snapshot salvage and orphan chunk detection
//! - **Scheduling**: systemd integration for automated backups
//! - **CLI interface**: Comprehensive command-line tools

pub mod backup;
pub mod chunk;
pub mod error;
pub mod manifest;
pub mod restore;

#[cfg(feature = "recovery")]
pub mod recovery;

pub mod scheduling;

pub use error::{Error, Result};