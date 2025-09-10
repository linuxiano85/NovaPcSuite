//! Manifest management for tracking files and their chunks

use crate::chunk::{ChunkHash, ChunkInfo};
use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Manifest format version
pub const MANIFEST_VERSION: u32 = 2;

/// A file record in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// Original file path
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// File modification time
    pub modified: DateTime<Utc>,
    /// File permissions (Unix-style)
    pub mode: Option<u32>,
    /// Ordered list of chunks that make up this file
    pub chunks: Vec<ChunkHash>,
    /// BLAKE3 hash of the complete file
    pub file_hash: ChunkHash,
    /// Merkle root of chunk hashes
    pub merkle_root: ChunkHash,
}

impl FileRecord {
    /// Create a new file record
    pub fn new(
        path: PathBuf,
        size: u64,
        modified: DateTime<Utc>,
        mode: Option<u32>,
        chunks: Vec<ChunkHash>,
        file_hash: ChunkHash,
    ) -> Self {
        let merkle_root = Self::compute_merkle_root(&chunks);
        Self {
            path,
            size,
            modified,
            mode,
            chunks,
            file_hash,
            merkle_root,
        }
    }

    /// Compute Merkle root from chunk hashes
    pub fn compute_merkle_root(chunks: &[ChunkHash]) -> ChunkHash {
        if chunks.is_empty() {
            return ChunkHash::from_bytes(b"");
        }

        let mut level: Vec<String> = chunks.iter().map(|h| h.as_str().to_string()).collect();

        while level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in level.chunks(2) {
                let combined = if chunk.len() == 2 {
                    format!("{}{}", chunk[0], chunk[1])
                } else {
                    chunk[0].clone()
                };
                
                let hash = blake3::hash(combined.as_bytes());
                next_level.push(hash.to_hex().to_string());
            }
            
            level = next_level;
        }

        ChunkHash::new(level[0].clone())
    }

    /// Verify the integrity of this file record
    pub fn verify_integrity(&self) -> bool {
        let computed_merkle = Self::compute_merkle_root(&self.chunks);
        computed_merkle == self.merkle_root
    }
}

/// Snapshot manifest containing file records and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Manifest format version
    pub version: u32,
    /// Unique snapshot identifier
    pub id: Uuid,
    /// Snapshot creation timestamp
    pub created: DateTime<Utc>,
    /// Human-readable snapshot name
    pub name: String,
    /// Source root path that was backed up
    pub source_root: PathBuf,
    /// All file records in this snapshot
    pub files: Vec<FileRecord>,
    /// Metadata about chunk usage
    pub chunk_stats: ChunkStats,
}

/// Statistics about chunk usage in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkStats {
    /// Total number of unique chunks
    pub total_chunks: usize,
    /// Total bytes stored in chunks
    pub total_bytes: u64,
    /// Number of deduplicated chunks (chunks used more than once)
    pub dedup_chunks: usize,
    /// Bytes saved through deduplication
    pub dedup_savings: u64,
}

impl Snapshot {
    /// Create a new snapshot
    pub fn new(name: String, source_root: PathBuf) -> Self {
        Self {
            version: MANIFEST_VERSION,
            id: Uuid::new_v4(),
            created: Utc::now(),
            name,
            source_root,
            files: Vec::new(),
            chunk_stats: ChunkStats {
                total_chunks: 0,
                total_bytes: 0,
                dedup_chunks: 0,
                dedup_savings: 0,
            },
        }
    }

    /// Add a file record to the snapshot
    pub fn add_file(&mut self, file_record: FileRecord) {
        self.files.push(file_record);
        self.update_chunk_stats();
    }

    /// Update chunk statistics
    fn update_chunk_stats(&mut self) {
        let mut chunk_usage: HashMap<&ChunkHash, usize> = HashMap::new();
        let mut total_bytes = 0u64;

        for file in &self.files {
            total_bytes += file.size;
            for chunk in &file.chunks {
                *chunk_usage.entry(chunk).or_insert(0) += 1;
            }
        }

        let total_chunks = chunk_usage.len();
        let dedup_chunks = chunk_usage.values().filter(|&&count| count > 1).count();
        
        // Calculate dedup savings (rough estimate)
        let dedup_savings = chunk_usage
            .values()
            .filter(|&&count| count > 1)
            .map(|&count| (count - 1) as u64)
            .sum::<u64>() * 1024 * 1024; // Rough estimate using average chunk size

        self.chunk_stats = ChunkStats {
            total_chunks,
            total_bytes,
            dedup_chunks,
            dedup_savings,
        };
    }

    /// Get all unique chunks referenced by this snapshot
    pub fn get_referenced_chunks(&self) -> Vec<&ChunkHash> {
        let mut chunks = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for file in &self.files {
            for chunk in &file.chunks {
                if seen.insert(chunk) {
                    chunks.push(chunk);
                }
            }
        }

        chunks
    }

    /// Find a file record by path
    pub fn find_file<P: AsRef<Path>>(&self, path: P) -> Option<&FileRecord> {
        let path = path.as_ref();
        self.files.iter().find(|f| f.path == path)
    }

    /// Save snapshot to file
    pub fn save<P: AsRef<Path>>(&self, manifest_path: P) -> Result<()> {
        let file = File::create(manifest_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }

    /// Load snapshot from file
    pub fn load<P: AsRef<Path>>(manifest_path: P) -> Result<Self> {
        let file = File::open(&manifest_path).map_err(|_| Error::ManifestNotFound {
            path: manifest_path.as_ref().display().to_string(),
        })?;
        
        let reader = BufReader::new(file);
        let snapshot: Snapshot = serde_json::from_reader(reader)?;

        // Verify version compatibility
        if snapshot.version != MANIFEST_VERSION {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "Unsupported manifest version: {} (expected {})",
                    snapshot.version, MANIFEST_VERSION
                ),
            });
        }

        Ok(snapshot)
    }
}

/// Manages manifest storage and retrieval
#[derive(Debug)]
pub struct ManifestStore {
    root_path: PathBuf,
    manifests_path: PathBuf,
}

impl ManifestStore {
    /// Create a new manifest store
    pub fn new<P: AsRef<Path>>(root_path: P) -> Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        let manifests_path = root_path.join("manifests");
        
        fs::create_dir_all(&manifests_path)?;
        
        Ok(Self {
            root_path,
            manifests_path,
        })
    }

    /// Store a snapshot manifest
    pub fn store_snapshot(&self, snapshot: &Snapshot) -> Result<PathBuf> {
        let filename = format!("{}.json", snapshot.id);
        let manifest_path = self.manifests_path.join(&filename);
        snapshot.save(&manifest_path)?;
        Ok(manifest_path)
    }

    /// Load a snapshot by ID
    pub fn load_snapshot(&self, id: &Uuid) -> Result<Snapshot> {
        let filename = format!("{}.json", id);
        let manifest_path = self.manifests_path.join(&filename);
        Snapshot::load(manifest_path)
    }

    /// List all available snapshots
    pub fn list_snapshots(&self) -> Result<Vec<Uuid>> {
        let mut snapshots = Vec::new();
        
        for entry in fs::read_dir(&self.manifests_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(uuid) = Uuid::parse_str(stem) {
                        snapshots.push(uuid);
                    }
                }
            }
        }
        
        Ok(snapshots)
    }

    /// Get the latest snapshot
    pub fn get_latest_snapshot(&self) -> Result<Option<Snapshot>> {
        let snapshot_ids = self.list_snapshots()?;
        
        if snapshot_ids.is_empty() {
            return Ok(None);
        }

        // Load all snapshots and find the most recent one
        let mut latest: Option<Snapshot> = None;
        
        for id in snapshot_ids {
            match self.load_snapshot(&id) {
                Ok(snapshot) => {
                    if latest.as_ref().map_or(true, |latest| snapshot.created > latest.created) {
                        latest = Some(snapshot);
                    }
                }
                Err(_) => continue, // Skip corrupted manifests
            }
        }
        
        Ok(latest)
    }

    /// Remove a snapshot manifest
    pub fn remove_snapshot(&self, id: &Uuid) -> Result<()> {
        let filename = format!("{}.json", id);
        let manifest_path = self.manifests_path.join(&filename);
        
        if manifest_path.exists() {
            fs::remove_file(manifest_path)?;
        }
        
        Ok(())
    }

    /// Get the manifests directory path
    pub fn manifests_path(&self) -> &Path {
        &self.manifests_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_merkle_root_computation() {
        let chunks = vec![
            ChunkHash::new("hash1".to_string()),
            ChunkHash::new("hash2".to_string()),
            ChunkHash::new("hash3".to_string()),
        ];

        let merkle_root = FileRecord::compute_merkle_root(&chunks);
        assert!(!merkle_root.as_str().is_empty());

        // Empty chunks should produce empty hash
        let empty_root = FileRecord::compute_merkle_root(&[]);
        assert_eq!(empty_root.as_str(), ChunkHash::from_bytes(b"").as_str());
    }

    #[test]
    fn test_snapshot_serialization() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let snapshot = Snapshot::new("test".to_string(), PathBuf::from("/test"));
        
        let manifest_path = temp_dir.path().join("test.json");
        snapshot.save(&manifest_path)?;
        
        let loaded = Snapshot::load(&manifest_path)?;
        assert_eq!(loaded.id, snapshot.id);
        assert_eq!(loaded.name, snapshot.name);
        
        Ok(())
    }

    #[test]
    fn test_manifest_store() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ManifestStore::new(temp_dir.path())?;
        
        let snapshot = Snapshot::new("test".to_string(), PathBuf::from("/test"));
        let id = snapshot.id;
        
        store.store_snapshot(&snapshot)?;
        
        let loaded = store.load_snapshot(&id)?;
        assert_eq!(loaded.id, id);
        
        let snapshots = store.list_snapshots()?;
        assert!(snapshots.contains(&id));
        
        Ok(())
    }
}