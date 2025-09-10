package backup

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/linuxiano85/NovaPcSuite/internal/chunk"
	"github.com/linuxiano85/NovaPcSuite/internal/manifest"
	"github.com/linuxiano85/NovaPcSuite/internal/progress"
)

// Engine is the main backup engine
type Engine struct {
	chunkStore  *chunk.Store
	manifest    *manifest.Manager
	broadcaster *progress.Broadcaster
	backupRoot  string
}

// NewEngine creates a new backup engine
func NewEngine(backupRoot string) *Engine {
	engine := &Engine{
		chunkStore:  chunk.NewStore(backupRoot),
		manifest:    manifest.NewManager(backupRoot),
		broadcaster: progress.NewBroadcaster(),
		backupRoot:  backupRoot,
	}
	
	// Add console handler by default
	engine.broadcaster.AddHandler(progress.ConsoleHandler)
	
	return engine
}

// AddProgressHandler adds a custom progress handler
func (e *Engine) AddProgressHandler(handler progress.Handler) {
	e.broadcaster.AddHandler(handler)
}

// Init initializes the backup engine
func (e *Engine) Init() error {
	e.broadcaster.EmitInfo("Initializing backup engine...")
	
	if err := e.chunkStore.Init(); err != nil {
		return fmt.Errorf("failed to initialize chunk store: %w", err)
	}
	
	if err := e.manifest.Init(); err != nil {
		return fmt.Errorf("failed to initialize manifest manager: %w", err)
	}
	
	e.broadcaster.EmitInfo("Backup engine initialized successfully")
	return nil
}

// Scan scans a directory and reports what would be backed up
func (e *Engine) Scan(sourcePath string) error {
	e.broadcaster.EmitEvent(progress.EventScanStart, "Starting scan", 0, 0, 0)
	
	if err := e.Init(); err != nil {
		e.broadcaster.EmitError(err)
		return err
	}
	
	// First pass: count files
	var totalFiles int64
	var totalSize int64
	
	err := filepath.Walk(sourcePath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() {
			totalFiles++
			totalSize += info.Size()
		}
		return nil
	})
	
	if err != nil {
		e.broadcaster.EmitError(err)
		return err
	}
	
	e.broadcaster.EmitInfo(fmt.Sprintf("Found %d files, %d bytes total", totalFiles, totalSize))
	
	// Second pass: analyze files
	tracker := progress.NewTracker(e.broadcaster, progress.EventScanProgress, totalFiles)
	var processedFiles int64
	
	err = filepath.Walk(sourcePath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		
		if !info.IsDir() {
			relPath, _ := filepath.Rel(sourcePath, path)
			tracker.Update(processedFiles, fmt.Sprintf("Scanning: %s", relPath))
			processedFiles++
		}
		
		return nil
	})
	
	if err != nil {
		e.broadcaster.EmitError(err)
		return err
	}
	
	tracker.Complete("Scan completed")
	e.broadcaster.EmitEvent(progress.EventScanComplete, fmt.Sprintf("Scanned %d files", totalFiles), 1.0, totalFiles, totalFiles)
	
	return nil
}

// Plan creates a backup plan without executing it
func (e *Engine) Plan(sourcePath string) error {
	e.broadcaster.EmitEvent(progress.EventPlanStart, "Starting backup plan", 0, 0, 0)
	
	if err := e.Init(); err != nil {
		e.broadcaster.EmitError(err)
		return err
	}
	
	// Create snapshot
	snapshot := e.manifest.CreateSnapshot(sourcePath)
	
	// Count files for progress tracking
	var totalFiles int64
	err := filepath.Walk(sourcePath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() {
			totalFiles++
		}
		return nil
	})
	
	if err != nil {
		e.broadcaster.EmitError(err)
		return err
	}
	
	tracker := progress.NewTracker(e.broadcaster, progress.EventPlanProgress, totalFiles)
	var processedFiles int64
	var newChunks int64
	var existingChunks int64
	
	err = filepath.Walk(sourcePath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		
		if !info.IsDir() {
			relPath, _ := filepath.Rel(sourcePath, path)
			tracker.Update(processedFiles, fmt.Sprintf("Planning: %s", relPath))
			
			// Analyze file chunks
			chunks, err := e.chunkStore.ChunkFile(path)
			if err != nil {
				return fmt.Errorf("failed to chunk file %s: %w", path, err)
			}
			
			// Count new vs existing chunks
			for _, chunk := range chunks {
				if e.chunkStore.Exists(chunk.Hash) {
					existingChunks++
				} else {
					newChunks++
				}
			}
			
			// Calculate file hash
			fileHash := e.chunkStore.CalculateFileHash(chunks)
			
			// Add to snapshot
			snapshot.AddFile(relPath, info, chunks, fileHash)
			processedFiles++
		}
		
		return nil
	})
	
	if err != nil {
		e.broadcaster.EmitError(err)
		return err
	}
	
	tracker.Complete("Plan completed")
	
	snapshot.UniqueChunks = newChunks
	snapshot.Metadata["existing_chunks"] = existingChunks
	snapshot.Metadata["new_chunks"] = newChunks
	snapshot.Metadata["deduplication_ratio"] = float64(existingChunks) / float64(existingChunks + newChunks)
	
	e.broadcaster.EmitInfo(fmt.Sprintf("Plan complete: %d files, %d new chunks, %d existing chunks (%.1f%% deduplication)",
		totalFiles, newChunks, existingChunks, 
		float64(existingChunks)/float64(existingChunks+newChunks)*100))
	
	e.broadcaster.EmitEvent(progress.EventPlanComplete, "Backup plan created", 1.0, totalFiles, totalFiles)
	
	return nil
}

// Run executes a backup
func (e *Engine) Run(sourcePath string) error {
	e.broadcaster.EmitEvent(progress.EventBackupStart, "Starting backup", 0, 0, 0)
	
	if err := e.Init(); err != nil {
		e.broadcaster.EmitError(err)
		return err
	}
	
	// Create snapshot
	snapshot := e.manifest.CreateSnapshot(sourcePath)
	
	// Count files for progress tracking
	var totalFiles int64
	var totalSize int64
	err := filepath.Walk(sourcePath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() {
			totalFiles++
			totalSize += info.Size()
		}
		return nil
	})
	
	if err != nil {
		e.broadcaster.EmitError(err)
		return err
	}
	
	tracker := progress.NewTracker(e.broadcaster, progress.EventBackupProgress, totalSize)
	var processedSize int64
	var uniqueChunks int64
	
	err = filepath.Walk(sourcePath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		
		if !info.IsDir() {
			relPath, _ := filepath.Rel(sourcePath, path)
			tracker.Update(processedSize, fmt.Sprintf("Backing up: %s", relPath))
			
			// Store file chunks
			chunks, err := e.chunkStore.ChunkFile(path)
			if err != nil {
				return fmt.Errorf("failed to backup file %s: %w", path, err)
			}
			
			// Count unique chunks stored
			for _, chunk := range chunks {
				if !e.chunkStore.Exists(chunk.Hash) {
					uniqueChunks++
				}
			}
			
			// Calculate file hash
			fileHash := e.chunkStore.CalculateFileHash(chunks)
			
			// Add to snapshot
			snapshot.AddFile(relPath, info, chunks, fileHash)
			processedSize += info.Size()
		}
		
		return nil
	})
	
	if err != nil {
		e.broadcaster.EmitError(err)
		return err
	}
	
	// Save snapshot manifest
	snapshot.UniqueChunks = uniqueChunks
	if err := e.manifest.Save(snapshot); err != nil {
		e.broadcaster.EmitError(err)
		return fmt.Errorf("failed to save snapshot: %w", err)
	}
	
	tracker.Complete("Backup completed")
	
	e.broadcaster.EmitInfo(fmt.Sprintf("Backup complete: %d files, %d bytes, %d unique chunks, snapshot ID: %s",
		totalFiles, totalSize, uniqueChunks, snapshot.ID))
	
	e.broadcaster.EmitEvent(progress.EventBackupComplete, 
		fmt.Sprintf("Backup completed - Snapshot: %s", snapshot.ID), 1.0, totalSize, totalSize)
	
	return nil
}

// ListSnapshots returns all available snapshots
func (e *Engine) ListSnapshots() ([]*manifest.Snapshot, error) {
	if err := e.manifest.Init(); err != nil {
		return nil, err
	}
	return e.manifest.List()
}

// GetSnapshot retrieves a specific snapshot
func (e *Engine) GetSnapshot(snapshotID string) (*manifest.Snapshot, error) {
	if err := e.manifest.Init(); err != nil {
		return nil, err
	}
	return e.manifest.Load(snapshotID)
}

// RestoreFile restores a single file from a snapshot
func (e *Engine) RestoreFile(snapshotID, filePath, targetPath string) error {
	snapshot, err := e.GetSnapshot(snapshotID)
	if err != nil {
		return err
	}
	
	fileEntry, exists := snapshot.Files[filePath]
	if !exists {
		return fmt.Errorf("file not found in snapshot: %s", filePath)
	}
	
	// Create target directory if needed
	targetDir := filepath.Dir(targetPath)
	if err := os.MkdirAll(targetDir, 0755); err != nil {
		return fmt.Errorf("failed to create target directory: %w", err)
	}
	
	// Restore file from chunks
	targetFile, err := os.Create(targetPath)
	if err != nil {
		return fmt.Errorf("failed to create target file: %w", err)
	}
	defer targetFile.Close()
	
	for _, chunk := range fileEntry.Chunks {
		data, err := e.chunkStore.Get(chunk.Hash)
		if err != nil {
			return fmt.Errorf("failed to get chunk %s: %w", chunk.Hash, err)
		}
		
		if _, err := targetFile.Write(data); err != nil {
			return fmt.Errorf("failed to write chunk data: %w", err)
		}
	}
	
	// Restore file permissions and timestamp
	if err := os.Chmod(targetPath, fileEntry.Permissions); err != nil {
		return fmt.Errorf("failed to restore permissions: %w", err)
	}
	
	if err := os.Chtimes(targetPath, fileEntry.ModTime, fileEntry.ModTime); err != nil {
		return fmt.Errorf("failed to restore timestamp: %w", err)
	}
	
	return nil
}