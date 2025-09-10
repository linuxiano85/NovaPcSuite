//! Error types for NovaPcSuite

use thiserror::Error;

/// Main error type for NovaPcSuite operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Chunk not found: {hash}")]
    ChunkNotFound { hash: String },

    #[error("Manifest not found: {path}")]
    ManifestNotFound { path: String },

    #[error("Invalid manifest format: {reason}")]
    InvalidManifest { reason: String },

    #[error("Integrity verification failed: {reason}")]
    IntegrityError { reason: String },

    #[error("Path mapping error: {reason}")]
    PathMapping { reason: String },

    #[error("Conflict resolution failed: {path}")]
    ConflictResolution { path: String },

    #[error("Scheduling error: {reason}")]
    Scheduling { reason: String },

    #[error("Recovery operation failed: {reason}")]
    Recovery { reason: String },

    #[error("Invalid configuration: {reason}")]
    Configuration { reason: String },

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("Feature not available: {feature}")]
    FeatureNotAvailable { feature: String },
}

/// Result type alias for NovaPcSuite operations
pub type Result<T> = std::result::Result<T, Error>;