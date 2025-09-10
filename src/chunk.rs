//! Chunk-based storage with content addressing using BLAKE3

use crate::{Error, Result};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Default chunk size for file splitting (1MB)
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// A content-addressed chunk identified by its BLAKE3 hash
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkHash(pub String);

impl ChunkHash {
    /// Create a new chunk hash from a BLAKE3 hash string
    pub fn new(hash: String) -> Self {
        Self(hash)
    }

    /// Get the hash as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create a chunk hash from raw bytes by computing BLAKE3
    pub fn from_bytes(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(hash.to_hex().to_string())
    }
}

impl std::fmt::Display for ChunkHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Metadata about a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub hash: ChunkHash,
    pub size: u64,
    pub compressed_size: Option<u64>,
}

/// A chunk store manages the storage and retrieval of content-addressed chunks
#[derive(Debug)]
pub struct ChunkStore {
    root_path: PathBuf,
    pub(crate) chunks_path: PathBuf,
}

impl ChunkStore {
    /// Create a new chunk store at the given root path
    pub fn new<P: AsRef<Path>>(root_path: P) -> Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        let chunks_path = root_path.join("chunks");
        
        // Create directories if they don't exist
        fs::create_dir_all(&chunks_path)?;
        
        Ok(Self {
            root_path,
            chunks_path,
        })
    }

    /// Store a chunk and return its hash and info
    pub fn store_chunk(&self, data: &[u8]) -> Result<ChunkInfo> {
        let hash = ChunkHash::from_bytes(data);
        let chunk_path = self.chunk_path(&hash);

        // Create directory structure if needed
        if let Some(parent) = chunk_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write chunk data
        let mut file = File::create(&chunk_path)?;
        file.write_all(data)?;
        file.sync_all()?;

        Ok(ChunkInfo {
            hash,
            size: data.len() as u64,
            compressed_size: None, // TODO: Add compression support
        })
    }

    /// Retrieve a chunk by its hash
    pub fn get_chunk(&self, hash: &ChunkHash) -> Result<Vec<u8>> {
        let chunk_path = self.chunk_path(hash);
        
        if !chunk_path.exists() {
            return Err(Error::ChunkNotFound {
                hash: hash.to_string(),
            });
        }

        let mut file = File::open(&chunk_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        // Verify integrity
        let computed_hash = ChunkHash::from_bytes(&data);
        if computed_hash != *hash {
            return Err(Error::IntegrityError {
                reason: format!(
                    "Chunk hash mismatch: expected {}, got {}",
                    hash, computed_hash
                ),
            });
        }

        Ok(data)
    }

    /// Check if a chunk exists in the store
    pub fn has_chunk(&self, hash: &ChunkHash) -> bool {
        self.chunk_path(hash).exists()
    }

    /// List all chunks in the store
    pub fn list_chunks(&self) -> Result<Vec<ChunkHash>> {
        let mut chunks = Vec::new();
        self.scan_chunks_dir(&self.chunks_path, &mut chunks)?;
        Ok(chunks)
    }

    /// Get chunk info without reading the full chunk
    pub fn get_chunk_info(&self, hash: &ChunkHash) -> Result<ChunkInfo> {
        let chunk_path = self.chunk_path(hash);
        
        if !chunk_path.exists() {
            return Err(Error::ChunkNotFound {
                hash: hash.to_string(),
            });
        }

        let metadata = fs::metadata(&chunk_path)?;
        Ok(ChunkInfo {
            hash: hash.clone(),
            size: metadata.len(),
            compressed_size: None,
        })
    }

    /// Remove a chunk from the store
    pub fn remove_chunk(&self, hash: &ChunkHash) -> Result<()> {
        let chunk_path = self.chunk_path(hash);
        if chunk_path.exists() {
            fs::remove_file(&chunk_path)?;
        }
        Ok(())
    }

    /// Get the file path for a chunk hash
    fn chunk_path(&self, hash: &ChunkHash) -> PathBuf {
        let hash_str = hash.as_str();
        // Use first 2 characters as directory for better file system performance
        let dir = &hash_str[..2];
        let file = &hash_str[2..];
        self.chunks_path.join(dir).join(file)
    }

    /// Recursively scan chunks directory
    fn scan_chunks_dir(&self, dir: &Path, chunks: &mut Vec<ChunkHash>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.scan_chunks_dir(&path, chunks)?;
            } else if path.is_file() {
                // Reconstruct hash from directory and filename
                if let (Some(dir_name), Some(file_name)) = (
                    path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()),
                    path.file_name().and_then(|n| n.to_str())
                ) {
                    let hash_str = format!("{}{}", dir_name, file_name);
                    chunks.push(ChunkHash::new(hash_str));
                }
            }
        }
        Ok(())
    }
}

/// Split a file into chunks
pub fn chunk_file<P: AsRef<Path>>(
    file_path: P,
    chunk_size: usize,
) -> Result<Vec<ChunkInfo>> {
    let mut file = File::open(&file_path)?;
    let mut chunks = Vec::new();
    let mut buffer = vec![0u8; chunk_size];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let chunk_data = &buffer[..bytes_read];
        let hash = ChunkHash::from_bytes(chunk_data);
        
        chunks.push(ChunkInfo {
            hash,
            size: bytes_read as u64,
            compressed_size: None,
        });
    }

    Ok(chunks)
}

/// Compute BLAKE3 hash for a file in streaming fashion
pub fn hash_file<P: AsRef<Path>>(file_path: P) -> Result<ChunkHash> {
    let mut file = File::open(file_path)?;
    let mut hasher = Hasher::new();
    
    io::copy(&mut file, &mut hasher)?;
    
    let hash = hasher.finalize();
    Ok(ChunkHash::new(hash.to_hex().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_chunk_hash_from_bytes() {
        let data = b"hello world";
        let hash = ChunkHash::from_bytes(data);
        
        // BLAKE3 hash of "hello world"
        assert_eq!(hash.as_str(), "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24");
    }

    #[test]
    fn test_chunk_store_basic_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ChunkStore::new(temp_dir.path())?;

        let data = b"test chunk data";
        let chunk_info = store.store_chunk(data)?;

        assert!(store.has_chunk(&chunk_info.hash));
        
        let retrieved = store.get_chunk(&chunk_info.hash)?;
        assert_eq!(retrieved, data);

        Ok(())
    }

    #[test]
    fn test_chunk_store_integrity_verification() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ChunkStore::new(temp_dir.path())?;

        let data = b"test data for integrity";
        let chunk_info = store.store_chunk(data)?;

        // Corrupt the chunk file
        let chunk_path = store.chunk_path(&chunk_info.hash);
        fs::write(&chunk_path, b"corrupted data")?;

        // Should fail integrity check
        let result = store.get_chunk(&chunk_info.hash);
        assert!(matches!(result, Err(Error::IntegrityError { .. })));

        Ok(())
    }
}