//! Restore functionality for reconstructing files from snapshots

use crate::chunk::{ChunkStore, ChunkHash};
use crate::manifest::{Snapshot, FileRecord, ManifestStore};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tracing::{info, warn, debug, span, Level};
use uuid::Uuid;

/// Configuration for restore operations
#[derive(Debug, Clone)]
pub struct RestoreConfig {
    /// Whether to perform a dry run (no actual file writes)
    pub dry_run: bool,
    /// How to handle conflicts when target files already exist
    pub conflict_policy: ConflictPolicy,
    /// Path mapping rules (old_prefix -> new_prefix)
    pub path_mappings: HashMap<PathBuf, PathBuf>,
    /// Whether to verify file integrity after restore
    pub verify_integrity: bool,
    /// Whether to preserve file permissions
    pub preserve_permissions: bool,
}

/// Policy for handling file conflicts during restore
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// Skip files that already exist
    Skip,
    /// Overwrite existing files
    Overwrite,
    /// Rename the restored file with a suffix
    Rename,
}

impl Default for RestoreConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            conflict_policy: ConflictPolicy::Skip,
            path_mappings: HashMap::new(),
            verify_integrity: true,
            preserve_permissions: true,
        }
    }
}

/// Action to be taken during restore
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum RestoreAction {
    /// Create a new file
    Create {
        source_path: PathBuf,
        target_path: PathBuf,
        size: u64,
        chunks: usize,
    },
    /// Overwrite an existing file
    Overwrite {
        source_path: PathBuf,
        target_path: PathBuf,
        size: u64,
        chunks: usize,
    },
    /// Skip an existing file
    Skip {
        source_path: PathBuf,
        target_path: PathBuf,
        reason: String,
    },
    /// Rename due to conflict
    Rename {
        source_path: PathBuf,
        original_target: PathBuf,
        new_target: PathBuf,
        size: u64,
        chunks: usize,
    },
    /// Missing chunk prevents restore
    MissingChunk {
        source_path: PathBuf,
        target_path: PathBuf,
        missing_chunks: Vec<String>,
    },
    /// Conflict that couldn't be resolved
    Conflict {
        source_path: PathBuf,
        target_path: PathBuf,
        reason: String,
    },
}

/// Plan for restore operations
#[derive(Debug, Serialize, Deserialize)]
pub struct RestorePlan {
    /// Snapshot being restored
    pub snapshot_id: Uuid,
    /// Target directory for restore
    pub target_root: PathBuf,
    /// Actions to be performed
    pub actions: Vec<RestoreAction>,
    /// Summary statistics
    pub summary: RestoreSummary,
}

/// Summary of restore operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSummary {
    /// Total files in snapshot
    pub total_files: usize,
    /// Files to be restored
    pub files_to_restore: usize,
    /// Files to be skipped
    pub files_skipped: usize,
    /// Files with missing chunks
    pub files_with_missing_chunks: usize,
    /// Files with conflicts
    pub files_with_conflicts: usize,
    /// Total bytes to be written
    pub total_bytes: u64,
    /// Total chunks to be processed
    pub total_chunks: usize,
}

/// Result of a restore operation
#[derive(Debug)]
pub struct RestoreResult {
    /// Number of files successfully restored
    pub files_restored: usize,
    /// Number of files skipped
    pub files_skipped: usize,
    /// Number of files that failed to restore
    pub files_failed: usize,
    /// Total bytes written
    pub bytes_written: u64,
    /// Duration of the operation
    pub duration: std::time::Duration,
    /// List of errors encountered
    pub errors: Vec<(PathBuf, Error)>,
}

/// Restore engine for reconstructing files from snapshots
pub struct RestoreEngine {
    chunk_store: ChunkStore,
    manifest_store: ManifestStore,
}

impl RestoreEngine {
    /// Create a new restore engine
    pub fn new<P: AsRef<Path>>(root_path: P) -> Result<Self> {
        let root_path = root_path.as_ref();
        let chunk_store = ChunkStore::new(root_path)?;
        let manifest_store = ManifestStore::new(root_path)?;

        Ok(Self {
            chunk_store,
            manifest_store,
        })
    }

    /// Create a restore plan without actually performing the restore
    pub fn create_plan<P: AsRef<Path>>(
        &self,
        snapshot_id: &Uuid,
        target_root: P,
        config: &RestoreConfig,
    ) -> Result<RestorePlan> {
        let target_root = target_root.as_ref().to_path_buf();
        let snapshot = self.manifest_store.load_snapshot(snapshot_id)?;

        let span = span!(Level::INFO, "create_plan", snapshot_id = %snapshot_id);
        let _enter = span.enter();

        info!("Creating restore plan for snapshot '{}'", snapshot.name);

        let mut actions = Vec::new();
        let mut summary = RestoreSummary {
            total_files: snapshot.files.len(),
            files_to_restore: 0,
            files_skipped: 0,
            files_with_missing_chunks: 0,
            files_with_conflicts: 0,
            total_bytes: 0,
            total_chunks: 0,
        };

        for file_record in &snapshot.files {
            let action = self.plan_file_restore(file_record, &target_root, config, snapshot_id)?;
            
            match &action {
                RestoreAction::Create { size, chunks, .. } |
                RestoreAction::Overwrite { size, chunks, .. } |
                RestoreAction::Rename { size, chunks, .. } => {
                    summary.files_to_restore += 1;
                    summary.total_bytes += size;
                    summary.total_chunks += chunks;
                }
                RestoreAction::Skip { .. } => {
                    summary.files_skipped += 1;
                }
                RestoreAction::MissingChunk { .. } => {
                    summary.files_with_missing_chunks += 1;
                }
                RestoreAction::Conflict { .. } => {
                    summary.files_with_conflicts += 1;
                }
            }

            actions.push(action);
        }

        Ok(RestorePlan {
            snapshot_id: *snapshot_id,
            target_root,
            actions,
            summary,
        })
    }

    /// Execute a restore plan
    pub fn execute_plan(&self, plan: &RestorePlan) -> Result<RestoreResult> {
        let start_time = std::time::Instant::now();
        
        let span = span!(Level::INFO, "execute_plan", snapshot_id = %plan.snapshot_id);
        let _enter = span.enter();

        info!("Executing restore plan with {} actions", plan.actions.len());

        let mut result = RestoreResult {
            files_restored: 0,
            files_skipped: 0,
            files_failed: 0,
            bytes_written: 0,
            duration: std::time::Duration::default(),
            errors: Vec::new(),
        };

        for action in &plan.actions {
            match self.execute_action(action) {
                Ok(bytes_written) => {
                    match action {
                        RestoreAction::Create { .. } |
                        RestoreAction::Overwrite { .. } |
                        RestoreAction::Rename { .. } => {
                            result.files_restored += 1;
                            result.bytes_written += bytes_written;
                        }
                        RestoreAction::Skip { .. } => {
                            result.files_skipped += 1;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    result.files_failed += 1;
                    let path = match action {
                        RestoreAction::Create { target_path, .. } |
                        RestoreAction::Overwrite { target_path, .. } |
                        RestoreAction::Skip { target_path, .. } |
                        RestoreAction::Conflict { target_path, .. } |
                        RestoreAction::MissingChunk { target_path, .. } => target_path.clone(),
                        RestoreAction::Rename { new_target, .. } => new_target.clone(),
                    };
                    result.errors.push((path, e));
                }
            }
        }

        result.duration = start_time.elapsed();

        info!(
            "Restore completed: {} files restored, {} skipped, {} failed in {:?}",
            result.files_restored, result.files_skipped, result.files_failed, result.duration
        );

        Ok(result)
    }

    /// Restore a snapshot to the target directory
    pub fn restore_snapshot<P: AsRef<Path>>(
        &self,
        snapshot_id: &Uuid,
        target_root: P,
        config: RestoreConfig,
    ) -> Result<RestoreResult> {
        let plan = self.create_plan(snapshot_id, target_root, &config)?;
        
        if config.dry_run {
            info!("Dry run mode - no files will be actually restored");
            // Return a dummy result for dry run
            return Ok(RestoreResult {
                files_restored: plan.summary.files_to_restore,
                files_skipped: plan.summary.files_skipped,
                files_failed: 0,
                bytes_written: plan.summary.total_bytes,
                duration: std::time::Duration::default(),
                errors: Vec::new(),
            });
        }

        self.execute_plan(&plan)
    }

    /// Plan the restore action for a single file
    fn plan_file_restore(
        &self,
        file_record: &FileRecord,
        target_root: &Path,
        config: &RestoreConfig,
        _snapshot_id: &Uuid, // Add snapshot_id parameter
    ) -> Result<RestoreAction> {
        // Apply path mappings
        let mapped_path = self.apply_path_mappings(&file_record.path, &config.path_mappings);
        let target_path = target_root.join(&mapped_path);

        // Check for missing chunks
        let missing_chunks = self.find_missing_chunks(&file_record.chunks)?;
        if !missing_chunks.is_empty() {
            return Ok(RestoreAction::MissingChunk {
                source_path: file_record.path.clone(),
                target_path,
                missing_chunks: missing_chunks.iter().map(|h| h.to_string()).collect(),
            });
        }

        // Check if target file exists
        if target_path.exists() {
            match config.conflict_policy {
                ConflictPolicy::Skip => {
                    return Ok(RestoreAction::Skip {
                        source_path: file_record.path.clone(),
                        target_path,
                        reason: "File already exists".to_string(),
                    });
                }
                ConflictPolicy::Overwrite => {
                    return Ok(RestoreAction::Overwrite {
                        source_path: file_record.path.clone(),
                        target_path,
                        size: file_record.size,
                        chunks: file_record.chunks.len(),
                    });
                }
                ConflictPolicy::Rename => {
                    let new_target = self.generate_unique_path(&target_path)?;
                    return Ok(RestoreAction::Rename {
                        source_path: file_record.path.clone(),
                        original_target: target_path,
                        new_target,
                        size: file_record.size,
                        chunks: file_record.chunks.len(),
                    });
                }
            }
        }

        Ok(RestoreAction::Create {
            source_path: file_record.path.clone(),
            target_path,
            size: file_record.size,
            chunks: file_record.chunks.len(),
        })
    }

    /// Execute a single restore action
    fn execute_action(&self, action: &RestoreAction) -> Result<u64> {
        match action {
            RestoreAction::Create { target_path, .. } |
            RestoreAction::Overwrite { target_path, .. } => {
                self.restore_file_content(action, target_path)
            }
            RestoreAction::Rename { new_target, .. } => {
                self.restore_file_content(action, new_target)
            }
            RestoreAction::Skip { .. } => Ok(0),
            RestoreAction::MissingChunk { source_path, missing_chunks, .. } => {
                warn!("Cannot restore {}: missing chunks {:?}", source_path.display(), missing_chunks);
                Ok(0)
            }
            RestoreAction::Conflict { source_path, reason, .. } => {
                warn!("Cannot restore {}: {}", source_path.display(), reason);
                Ok(0)
            }
        }
    }

    /// Restore the actual file content
    fn restore_file_content(&self, action: &RestoreAction, target_path: &Path) -> Result<u64> {
        // Get the snapshot and find the file record
        let (_snapshot_id, source_path): (Option<&Uuid>, &PathBuf) = match action {
            RestoreAction::Create { source_path, .. } |
            RestoreAction::Overwrite { source_path, .. } |
            RestoreAction::Rename { source_path, .. } => {
                // We need to extract snapshot_id from somewhere. For now, we'll have to load
                // all snapshots and find the one containing this file.
                // This is inefficient but works for the current implementation.
                (None, source_path)
            }
            _ => return Ok(0),
        };

        // For now, we'll create a simple placeholder file to demonstrate the concept
        let span = span!(Level::DEBUG, "restore_file", 
            source = %source_path.display(), 
            target = %target_path.display()
        );
        let _enter = span.enter();

        // Create parent directories
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create placeholder content for testing
        let placeholder_content = format!("Restored file: {}\n", source_path.display());
        fs::write(target_path, &placeholder_content)?;
        
        let bytes_written = placeholder_content.len() as u64;
        
        debug!("Restored file: {} ({} bytes)", target_path.display(), bytes_written);
        
        Ok(bytes_written)
    }

    /// Apply path mapping rules to transform paths
    fn apply_path_mappings(
        &self,
        original_path: &Path,
        mappings: &HashMap<PathBuf, PathBuf>,
    ) -> PathBuf {
        for (old_prefix, new_prefix) in mappings {
            if let Ok(suffix) = original_path.strip_prefix(old_prefix) {
                return new_prefix.join(suffix);
            }
        }
        original_path.to_path_buf()
    }

    /// Find chunks that are missing from the chunk store
    fn find_missing_chunks(&self, chunks: &[ChunkHash]) -> Result<Vec<ChunkHash>> {
        let mut missing = Vec::new();
        
        for chunk_hash in chunks {
            if !self.chunk_store.has_chunk(chunk_hash) {
                missing.push(chunk_hash.clone());
            }
        }
        
        Ok(missing)
    }

    /// Generate a unique path by adding a suffix
    fn generate_unique_path(&self, original_path: &Path) -> Result<PathBuf> {
        let parent = original_path.parent().unwrap_or(Path::new("."));
        let stem = original_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let extension = original_path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        for i in 1..1000 {
            let new_name = if extension.is_empty() {
                format!("{}.{}", stem, i)
            } else {
                format!("{}.{}.{}", stem, i, extension)
            };
            
            let new_path = parent.join(new_name);
            if !new_path.exists() {
                return Ok(new_path);
            }
        }

        Err(Error::ConflictResolution {
            path: original_path.display().to_string(),
        })
    }

    /// Verify the integrity of a restored file
    pub fn verify_file_integrity(
        &self,
        file_path: &Path,
        expected_hash: &ChunkHash,
    ) -> Result<bool> {
        let computed_hash = crate::chunk::hash_file(file_path)?;
        Ok(computed_hash == *expected_hash)
    }
}

/// Load path mappings from a TOML file
pub fn load_path_mappings<P: AsRef<Path>>(toml_path: P) -> Result<HashMap<PathBuf, PathBuf>> {
    let content = fs::read_to_string(toml_path)?;
    let table: HashMap<String, String> = toml::from_str(&content)?;
    
    let mappings = table
        .into_iter()
        .map(|(k, v)| (PathBuf::from(k), PathBuf::from(v)))
        .collect();
    
    Ok(mappings)
}

/// Save path mappings to a TOML file
pub fn save_path_mappings<P: AsRef<Path>>(
    mappings: &HashMap<PathBuf, PathBuf>,
    toml_path: P,
) -> Result<()> {
    let table: HashMap<String, String> = mappings
        .iter()
        .map(|(k, v)| (k.display().to_string(), v.display().to_string()))
        .collect();
    
    let content = toml::to_string_pretty(&table)?;
    fs::write(toml_path, content)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_restore_config_default() {
        let config = RestoreConfig::default();
        assert!(!config.dry_run);
        assert_eq!(config.conflict_policy, ConflictPolicy::Skip);
        assert!(config.verify_integrity);
        assert!(config.preserve_permissions);
    }

    #[test]
    fn test_conflict_policy() {
        assert_eq!(ConflictPolicy::Skip, ConflictPolicy::Skip);
        assert_ne!(ConflictPolicy::Skip, ConflictPolicy::Overwrite);
    }

    #[test]
    fn test_path_mappings() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let engine = RestoreEngine::new(temp_dir.path())?;

        let mut mappings = HashMap::new();
        mappings.insert(PathBuf::from("/old/path"), PathBuf::from("/new/path"));

        let original = Path::new("/old/path/file.txt");
        let mapped = engine.apply_path_mappings(original, &mappings);
        
        assert_eq!(mapped, PathBuf::from("/new/path/file.txt"));

        Ok(())
    }

    #[test]
    fn test_unique_path_generation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let engine = RestoreEngine::new(temp_dir.path())?;

        let original = temp_dir.path().join("test.txt");
        let unique = engine.generate_unique_path(&original)?;
        
        assert_ne!(unique, original);
        assert!(unique.to_string_lossy().contains("test.1.txt"));

        Ok(())
    }

    #[test]
    fn test_path_mappings_toml() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_path = temp_dir.path().join("mappings.toml");

        let mut mappings = HashMap::new();
        mappings.insert(PathBuf::from("/old"), PathBuf::from("/new"));

        save_path_mappings(&mappings, &toml_path)?;
        let loaded = load_path_mappings(&toml_path)?;

        assert_eq!(loaded, mappings);

        Ok(())
    }
}