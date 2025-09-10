//! Data recovery functionality for salvaging corrupted backups

use crate::chunk::{ChunkStore, ChunkHash};
use crate::manifest::{Snapshot, ManifestStore, FileRecord};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn, debug, span, Level};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Report of orphaned chunks not referenced by any manifest
#[derive(Debug, Serialize, Deserialize)]
pub struct OrphanChunkReport {
    /// When this report was generated
    pub generated_at: DateTime<Utc>,
    /// Total number of orphaned chunks found
    pub total_orphans: usize,
    /// Total size of orphaned chunks in bytes
    pub total_size: u64,
    /// List of orphaned chunk details
    pub orphans: Vec<OrphanChunk>,
    /// Summary by chunk size ranges
    pub size_distribution: HashMap<String, usize>,
}

/// Information about an orphaned chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanChunk {
    /// Hash of the orphaned chunk
    pub hash: ChunkHash,
    /// Size of the chunk in bytes
    pub size: u64,
    /// Path to the chunk file
    pub path: PathBuf,
    /// Last modified time of the chunk file
    pub last_modified: DateTime<Utc>,
}

/// Result of snapshot salvage operation
#[derive(Debug, Serialize, Deserialize)]
pub struct SalvageResult {
    /// Number of manifests found and processed
    pub manifests_processed: usize,
    /// Number of manifests that were corrupted/invalid
    pub corrupted_manifests: usize,
    /// Number of file records recovered
    pub files_recovered: usize,
    /// Number of unique chunks referenced
    pub chunks_referenced: usize,
    /// Rebuilt manifest index
    pub rebuilt_index: Vec<SalvageSnapshot>,
    /// Errors encountered during salvage
    pub errors: Vec<String>,
}

/// Minimal snapshot information from salvage operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalvageSnapshot {
    /// Snapshot ID if recoverable
    pub id: Option<Uuid>,
    /// Snapshot name if recoverable
    pub name: Option<String>,
    /// Creation time if recoverable
    pub created: Option<DateTime<Utc>>,
    /// Number of files in this snapshot
    pub file_count: usize,
    /// Original manifest file path
    pub manifest_path: PathBuf,
    /// Whether the manifest was corrupted
    pub corrupted: bool,
}

/// Recovery engine for data recovery operations
pub struct RecoveryEngine {
    chunk_store: ChunkStore,
    manifest_store: ManifestStore,
}

impl RecoveryEngine {
    /// Create a new recovery engine
    pub fn new<P: AsRef<Path>>(root_path: P) -> Result<Self> {
        let root_path = root_path.as_ref();
        let chunk_store = ChunkStore::new(root_path)?;
        let manifest_store = ManifestStore::new(root_path)?;

        Ok(Self {
            chunk_store,
            manifest_store,
        })
    }

    /// Detect orphaned chunks not referenced by any manifest
    pub fn detect_orphan_chunks(&self) -> Result<OrphanChunkReport> {
        let span = span!(Level::INFO, "detect_orphan_chunks");
        let _enter = span.enter();

        info!("Starting orphan chunk detection");

        // Get all chunks from the chunk store
        let all_chunks = self.chunk_store.list_chunks()?;
        debug!("Found {} total chunks in store", all_chunks.len());

        // Get all referenced chunks from manifests
        let referenced_chunks = self.get_all_referenced_chunks()?;
        debug!("Found {} referenced chunks in manifests", referenced_chunks.len());

        // Find orphans
        let orphan_hashes: HashSet<_> = all_chunks
            .iter()
            .filter(|chunk| !referenced_chunks.contains(chunk))
            .collect();

        info!("Found {} orphaned chunks", orphan_hashes.len());

        // Collect detailed information about orphans
        let mut orphans = Vec::new();
        let mut total_size = 0u64;
        let mut size_distribution = HashMap::new();

        for chunk_hash in orphan_hashes {
            match self.get_orphan_chunk_info(chunk_hash) {
                Ok(orphan) => {
                    total_size += orphan.size;
                    
                    // Categorize by size
                    let size_category = self.categorize_chunk_size(orphan.size);
                    *size_distribution.entry(size_category).or_insert(0) += 1;
                    
                    orphans.push(orphan);
                }
                Err(e) => {
                    warn!("Failed to get info for orphan chunk {}: {}", chunk_hash, e);
                }
            }
        }

        // Sort orphans by size (largest first)
        orphans.sort_by(|a, b| b.size.cmp(&a.size));

        Ok(OrphanChunkReport {
            generated_at: Utc::now(),
            total_orphans: orphans.len(),
            total_size,
            orphans,
            size_distribution,
        })
    }

    /// Salvage snapshot manifests and rebuild index
    pub fn salvage_snapshots(&self) -> Result<SalvageResult> {
        let span = span!(Level::INFO, "salvage_snapshots");
        let _enter = span.enter();

        info!("Starting snapshot salvage operation");

        let manifests_dir = self.manifest_store.manifests_path();
        let mut result = SalvageResult {
            manifests_processed: 0,
            corrupted_manifests: 0,
            files_recovered: 0,
            chunks_referenced: 0,
            rebuilt_index: Vec::new(),
            errors: Vec::new(),
        };

        // Scan for manifest files
        for entry in fs::read_dir(manifests_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                result.manifests_processed += 1;
                
                match self.salvage_single_manifest(&path) {
                    Ok(salvage_info) => {
                        if salvage_info.corrupted {
                            result.corrupted_manifests += 1;
                        }
                        
                        result.files_recovered += salvage_info.file_count;
                        result.rebuilt_index.push(salvage_info);
                    }
                    Err(e) => {
                        result.corrupted_manifests += 1;
                        result.errors.push(format!("Failed to process {}: {}", path.display(), e));
                        
                        // Still add an entry for the corrupted manifest
                        result.rebuilt_index.push(SalvageSnapshot {
                            id: None,
                            name: None,
                            created: None,
                            file_count: 0,
                            manifest_path: path,
                            corrupted: true,
                        });
                    }
                }
            }
        }

        // Count unique chunks
        let mut all_chunks = HashSet::new();
        for snapshot in &result.rebuilt_index {
            if !snapshot.corrupted {
                if let Ok(full_snapshot) = Snapshot::load(&snapshot.manifest_path) {
                    for file in &full_snapshot.files {
                        for chunk in &file.chunks {
                            all_chunks.insert(chunk.clone());
                        }
                    }
                }
            }
        }
        
        result.chunks_referenced = all_chunks.len();

        info!(
            "Salvage completed: {}/{} manifests processed, {} files recovered, {} chunks referenced",
            result.manifests_processed - result.corrupted_manifests,
            result.manifests_processed,
            result.files_recovered,
            result.chunks_referenced
        );

        Ok(result)
    }

    /// Validate the integrity of a snapshot
    pub fn validate_snapshot(&self, snapshot_id: &Uuid) -> Result<ValidationResult> {
        let span = span!(Level::INFO, "validate_snapshot", snapshot_id = %snapshot_id);
        let _enter = span.enter();

        let snapshot = self.manifest_store.load_snapshot(snapshot_id)?;
        
        info!("Validating snapshot '{}' with {} files", snapshot.name, snapshot.files.len());

        let mut result = ValidationResult {
            snapshot_id: *snapshot_id,
            total_files: snapshot.files.len(),
            valid_files: 0,
            corrupted_files: 0,
            missing_chunks: 0,
            integrity_errors: Vec::new(),
        };

        for file_record in &snapshot.files {
            match self.validate_file_record(file_record) {
                Ok(file_valid) => {
                    if file_valid {
                        result.valid_files += 1;
                    } else {
                        result.corrupted_files += 1;
                    }
                }
                Err(e) => {
                    result.corrupted_files += 1;
                    result.integrity_errors.push(IntegrityError {
                        file_path: file_record.path.clone(),
                        error_type: IntegrityErrorType::ValidationFailed,
                        details: e.to_string(),
                    });
                }
            }
        }

        info!(
            "Validation completed: {}/{} files valid, {} corrupted",
            result.valid_files, result.total_files, result.corrupted_files
        );

        Ok(result)
    }

    /// Clean up orphaned chunks
    pub fn cleanup_orphans(&self, orphan_report: &OrphanChunkReport, confirm: bool) -> Result<CleanupResult> {
        if !confirm {
            return Err(Error::Cancelled);
        }

        let span = span!(Level::INFO, "cleanup_orphans", count = orphan_report.total_orphans);
        let _enter = span.enter();

        info!("Starting cleanup of {} orphaned chunks", orphan_report.total_orphans);

        let mut result = CleanupResult {
            chunks_removed: 0,
            bytes_freed: 0,
            errors: Vec::new(),
        };

        for orphan in &orphan_report.orphans {
            match self.chunk_store.remove_chunk(&orphan.hash) {
                Ok(_) => {
                    result.chunks_removed += 1;
                    result.bytes_freed += orphan.size;
                    debug!("Removed orphan chunk: {}", orphan.hash);
                }
                Err(e) => {
                    result.errors.push(format!("Failed to remove {}: {}", orphan.hash, e));
                }
            }
        }

        info!(
            "Cleanup completed: {} chunks removed, {} bytes freed",
            result.chunks_removed, result.bytes_freed
        );

        Ok(result)
    }

    /// Get all chunks referenced by all manifests
    fn get_all_referenced_chunks(&self) -> Result<HashSet<ChunkHash>> {
        let mut referenced = HashSet::new();
        
        let snapshot_ids = self.manifest_store.list_snapshots()?;
        
        for snapshot_id in snapshot_ids {
            match self.manifest_store.load_snapshot(&snapshot_id) {
                Ok(snapshot) => {
                    for file in &snapshot.files {
                        for chunk in &file.chunks {
                            referenced.insert(chunk.clone());
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load snapshot {}: {}", snapshot_id, e);
                }
            }
        }
        
        Ok(referenced)
    }

    /// Get detailed information about an orphaned chunk
    fn get_orphan_chunk_info(&self, chunk_hash: &ChunkHash) -> Result<OrphanChunk> {
        let chunk_info = self.chunk_store.get_chunk_info(chunk_hash)?;
        
        // Get file path using the chunk store's private method
        let hash_str = chunk_hash.as_str();
        let dir = &hash_str[..2];
        let file = &hash_str[2..];
        let chunk_path = self.chunk_store.chunks_path.join(dir).join(file);
        
        // Get file metadata for last modified time
        let metadata = fs::metadata(&chunk_path)?;
        let last_modified = DateTime::from(metadata.modified()?);
        
        Ok(OrphanChunk {
            hash: chunk_hash.clone(),
            size: chunk_info.size,
            path: chunk_path,
            last_modified,
        })
    }

    /// Categorize chunk size for distribution analysis
    fn categorize_chunk_size(&self, size: u64) -> String {
        match size {
            0..=1024 => "tiny (≤1KB)".to_string(),
            1025..=10240 => "small (1-10KB)".to_string(),
            10241..=102400 => "medium (10-100KB)".to_string(),
            102401..=1048576 => "large (100KB-1MB)".to_string(),
            _ => "huge (>1MB)".to_string(),
        }
    }

    /// Salvage a single manifest file
    fn salvage_single_manifest(&self, manifest_path: &Path) -> Result<SalvageSnapshot> {
        match Snapshot::load(manifest_path) {
            Ok(snapshot) => {
                Ok(SalvageSnapshot {
                    id: Some(snapshot.id),
                    name: Some(snapshot.name),
                    created: Some(snapshot.created),
                    file_count: snapshot.files.len(),
                    manifest_path: manifest_path.to_path_buf(),
                    corrupted: false,
                })
            }
            Err(_) => {
                // Try to extract partial information from corrupted manifest
                match fs::read_to_string(manifest_path) {
                    Ok(content) => {
                        // Try to extract basic info with partial parsing
                        let file_count = content.matches("\"path\"").count();
                        
                        Ok(SalvageSnapshot {
                            id: None,
                            name: Some("CORRUPTED".to_string()),
                            created: None,
                            file_count,
                            manifest_path: manifest_path.to_path_buf(),
                            corrupted: true,
                        })
                    }
                    Err(e) => Err(Error::Recovery {
                        reason: format!("Cannot read manifest file: {}", e),
                    }),
                }
            }
        }
    }

    /// Validate a single file record
    fn validate_file_record(&self, file_record: &FileRecord) -> Result<bool> {
        // Check if all chunks exist
        for chunk_hash in &file_record.chunks {
            if !self.chunk_store.has_chunk(chunk_hash) {
                return Ok(false);
            }
        }

        // Verify Merkle root
        if !file_record.verify_integrity() {
            return Ok(false);
        }

        // TODO: Could also verify individual chunk hashes
        
        Ok(true)
    }
}

/// Result of snapshot validation
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// ID of the validated snapshot
    pub snapshot_id: Uuid,
    /// Total number of files in snapshot
    pub total_files: usize,
    /// Number of valid files
    pub valid_files: usize,
    /// Number of corrupted files
    pub corrupted_files: usize,
    /// Number of missing chunks
    pub missing_chunks: usize,
    /// List of integrity errors
    pub integrity_errors: Vec<IntegrityError>,
}

/// Information about an integrity error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityError {
    /// Path of the affected file
    pub file_path: PathBuf,
    /// Type of integrity error
    pub error_type: IntegrityErrorType,
    /// Detailed error description
    pub details: String,
}

/// Types of integrity errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrityErrorType {
    /// Missing chunk file
    MissingChunk,
    /// Chunk hash mismatch
    ChunkHashMismatch,
    /// Merkle root mismatch
    MerkleRootMismatch,
    /// File hash mismatch
    FileHashMismatch,
    /// Validation failed for other reasons
    ValidationFailed,
}

/// Result of orphan cleanup operation
#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupResult {
    /// Number of chunks successfully removed
    pub chunks_removed: usize,
    /// Total bytes freed
    pub bytes_freed: u64,
    /// List of errors encountered
    pub errors: Vec<String>,
}

// Private method for chunk_path access - Remove this since it conflicts
// impl ChunkStore {
//     pub(crate) fn chunk_path(&self, hash: &ChunkHash) -> PathBuf {
//         let hash_str = hash.as_str();
//         let dir = &hash_str[..2];
//         let file = &hash_str[2..];
//         self.chunks_path.join(dir).join(file)
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::manifest::Snapshot;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_recovery_engine_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let _engine = RecoveryEngine::new(temp_dir.path())?;
        Ok(())
    }

    #[test]
    fn test_chunk_size_categorization() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let engine = RecoveryEngine::new(temp_dir.path())?;
        
        assert_eq!(engine.categorize_chunk_size(500), "tiny (≤1KB)");
        assert_eq!(engine.categorize_chunk_size(5000), "small (1-10KB)");
        assert_eq!(engine.categorize_chunk_size(50000), "medium (10-100KB)");
        assert_eq!(engine.categorize_chunk_size(500000), "large (100KB-1MB)");
        assert_eq!(engine.categorize_chunk_size(5000000), "huge (>1MB)");
        
        Ok(())
    }

    #[test]
    fn test_orphan_detection_empty_store() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let engine = RecoveryEngine::new(temp_dir.path())?;
        
        let report = engine.detect_orphan_chunks()?;
        assert_eq!(report.total_orphans, 0);
        assert_eq!(report.total_size, 0);
        
        Ok(())
    }

    #[test]
    fn test_salvage_empty_manifests() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let engine = RecoveryEngine::new(temp_dir.path())?;
        
        let result = engine.salvage_snapshots()?;
        assert_eq!(result.manifests_processed, 0);
        assert_eq!(result.files_recovered, 0);
        
        Ok(())
    }

    #[test]
    fn test_salvage_corrupted_manifest() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let engine = RecoveryEngine::new(temp_dir.path())?;
        
        // Create a corrupted manifest file
        let manifests_dir = temp_dir.path().join("manifests");
        fs::create_dir_all(&manifests_dir)?;
        
        let corrupted_path = manifests_dir.join("corrupted.json");
        let mut file = File::create(&corrupted_path)?;
        writeln!(file, "{{ corrupted json content")?;
        
        let result = engine.salvage_snapshots()?;
        assert_eq!(result.manifests_processed, 1);
        assert_eq!(result.corrupted_manifests, 1);
        assert_eq!(result.rebuilt_index.len(), 1);
        assert!(result.rebuilt_index[0].corrupted);
        
        Ok(())
    }
}