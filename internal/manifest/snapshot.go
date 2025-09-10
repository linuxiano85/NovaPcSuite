package manifest

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/google/uuid"
	"github.com/linuxiano85/NovaPcSuite/internal/chunk"
)

// ManifestVersion represents the manifest format version
const ManifestVersion = "2.0"

// FileEntry represents a file in the backup
type FileEntry struct {
	Path         string         `json:"path"`
	Size         int64          `json:"size"`
	ModTime      time.Time      `json:"mod_time"`
	Chunks       []*chunk.Chunk `json:"chunks"`
	FileHash     string         `json:"file_hash"`
	Permissions  os.FileMode    `json:"permissions"`
	IsDir        bool           `json:"is_dir"`
}

// Snapshot represents a backup snapshot
type Snapshot struct {
	ID            string                 `json:"id"`
	Version       string                 `json:"version"`
	Timestamp     time.Time              `json:"timestamp"`
	SourcePath    string                 `json:"source_path"`
	Files         map[string]*FileEntry  `json:"files"`
	TotalSize     int64                  `json:"total_size"`
	TotalFiles    int64                  `json:"total_files"`
	UniqueChunks  int64                  `json:"unique_chunks"`
	Metadata      map[string]interface{} `json:"metadata"`
}

// Manager handles snapshot manifests
type Manager struct {
	rootPath string
}

// NewManager creates a new manifest manager
func NewManager(rootPath string) *Manager {
	return &Manager{
		rootPath: rootPath,
	}
}

// Init initializes the manifest storage
func (m *Manager) Init() error {
	manifestsDir := filepath.Join(m.rootPath, "manifests")
	return os.MkdirAll(manifestsDir, 0755)
}

// CreateSnapshot creates a new snapshot
func (m *Manager) CreateSnapshot(sourcePath string) *Snapshot {
	return &Snapshot{
		ID:           uuid.New().String(),
		Version:      ManifestVersion,
		Timestamp:    time.Now(),
		SourcePath:   sourcePath,
		Files:        make(map[string]*FileEntry),
		Metadata:     make(map[string]interface{}),
	}
}

// AddFile adds a file entry to the snapshot
func (s *Snapshot) AddFile(path string, info os.FileInfo, chunks []*chunk.Chunk, fileHash string) {
	entry := &FileEntry{
		Path:        path,
		Size:        info.Size(),
		ModTime:     info.ModTime(),
		Chunks:      chunks,
		FileHash:    fileHash,
		Permissions: info.Mode(),
		IsDir:       info.IsDir(),
	}
	
	s.Files[path] = entry
	s.TotalSize += info.Size()
	s.TotalFiles++
}

// Save saves the snapshot manifest to disk
func (m *Manager) Save(snapshot *Snapshot) error {
	manifestPath := filepath.Join(m.rootPath, "manifests", snapshot.ID+".json")
	
	data, err := json.MarshalIndent(snapshot, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal snapshot: %w", err)
	}
	
	if err := os.WriteFile(manifestPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write manifest: %w", err)
	}
	
	// Also save as latest.json for easy access
	latestPath := filepath.Join(m.rootPath, "manifests", "latest.json")
	if err := os.WriteFile(latestPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write latest manifest: %w", err)
	}
	
	return nil
}

// Load loads a snapshot by ID
func (m *Manager) Load(snapshotID string) (*Snapshot, error) {
	manifestPath := filepath.Join(m.rootPath, "manifests", snapshotID+".json")
	
	data, err := os.ReadFile(manifestPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read manifest: %w", err)
	}
	
	var snapshot Snapshot
	if err := json.Unmarshal(data, &snapshot); err != nil {
		return nil, fmt.Errorf("failed to unmarshal snapshot: %w", err)
	}
	
	return &snapshot, nil
}

// LoadLatest loads the latest snapshot
func (m *Manager) LoadLatest() (*Snapshot, error) {
	latestPath := filepath.Join(m.rootPath, "manifests", "latest.json")
	
	data, err := os.ReadFile(latestPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read latest manifest: %w", err)
	}
	
	var snapshot Snapshot
	if err := json.Unmarshal(data, &snapshot); err != nil {
		return nil, fmt.Errorf("failed to unmarshal latest snapshot: %w", err)
	}
	
	return &snapshot, nil
}

// List returns all available snapshots
func (m *Manager) List() ([]*Snapshot, error) {
	manifestsDir := filepath.Join(m.rootPath, "manifests")
	
	entries, err := os.ReadDir(manifestsDir)
	if err != nil {
		return nil, fmt.Errorf("failed to read manifests directory: %w", err)
	}
	
	var snapshots []*Snapshot
	for _, entry := range entries {
		if entry.IsDir() || entry.Name() == "latest.json" {
			continue
		}
		
		if filepath.Ext(entry.Name()) == ".json" {
			snapshotID := entry.Name()[:len(entry.Name())-5] // Remove .json
			snapshot, err := m.Load(snapshotID)
			if err != nil {
				continue // Skip corrupted manifests
			}
			snapshots = append(snapshots, snapshot)
		}
	}
	
	return snapshots, nil
}