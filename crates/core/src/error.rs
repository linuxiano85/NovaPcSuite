use thiserror::Error;

#[derive(Error, Debug)]
pub enum NovaError {
    #[error("ADB error: {0}")]
    Adb(String),
    
    #[error("Device error: {0}")]
    Device(String),
    
    #[error("File operation error: {0}")]
    FileOperation(String),
    
    #[error("Backup error: {0}")]
    Backup(String),
    
    #[error("Restore error: {0}")]
    Restore(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Manifest error: {0}")]
    Manifest(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    
    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),
}

pub type Result<T> = std::result::Result<T, NovaError>;