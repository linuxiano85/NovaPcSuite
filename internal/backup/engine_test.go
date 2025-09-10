package backup

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/linuxiano85/NovaPcSuite/internal/progress"
)

func TestEngine_BasicOperations(t *testing.T) {
	tmpDir := t.TempDir()
	backupDir := filepath.Join(tmpDir, "backups")
	testDataDir := filepath.Join(tmpDir, "test_data")
	
	// Create test data
	err := os.MkdirAll(testDataDir, 0755)
	if err != nil {
		t.Fatalf("Failed to create test data dir: %v", err)
	}
	
	testFile1 := filepath.Join(testDataDir, "file1.txt")
	testFile2 := filepath.Join(testDataDir, "file2.txt")
	
	err = os.WriteFile(testFile1, []byte("Hello, World!"), 0644)
	if err != nil {
		t.Fatalf("Failed to create test file 1: %v", err)
	}
	
	err = os.WriteFile(testFile2, []byte("This is a test file"), 0644)
	if err != nil {
		t.Fatalf("Failed to create test file 2: %v", err)
	}
	
	// Create engine
	engine := NewEngine(backupDir)
	
	// Test event collection
	var events []*progress.Event
	engine.AddProgressHandler(func(event *progress.Event) {
		events = append(events, event)
	})
	
	// Test scan
	err = engine.Scan(testDataDir)
	if err != nil {
		t.Fatalf("Scan failed: %v", err)
	}
	
	// Test plan
	err = engine.Plan(testDataDir)
	if err != nil {
		t.Fatalf("Plan failed: %v", err)
	}
	
	// Test backup run
	err = engine.Run(testDataDir)
	if err != nil {
		t.Fatalf("Backup run failed: %v", err)
	}
	
	// Verify backup structure was created
	chunksDir := filepath.Join(backupDir, "chunks")
	if _, err := os.Stat(chunksDir); os.IsNotExist(err) {
		t.Fatal("chunks directory was not created")
	}
	
	manifestsDir := filepath.Join(backupDir, "manifests")
	if _, err := os.Stat(manifestsDir); os.IsNotExist(err) {
		t.Fatal("manifests directory was not created")
	}
	
	latestManifest := filepath.Join(manifestsDir, "latest.json")
	if _, err := os.Stat(latestManifest); os.IsNotExist(err) {
		t.Fatal("latest manifest was not created")
	}
	
	// Test snapshot listing
	snapshots, err := engine.ListSnapshots()
	if err != nil {
		t.Fatalf("ListSnapshots failed: %v", err)
	}
	
	if len(snapshots) == 0 {
		t.Fatal("Expected at least one snapshot")
	}
	
	snapshot := snapshots[0]
	if len(snapshot.Files) != 2 {
		t.Fatalf("Expected 2 files in snapshot, got %d", len(snapshot.Files))
	}
	
	// Verify events were emitted
	if len(events) == 0 {
		t.Fatal("Expected progress events to be emitted")
	}
	
	// Check for specific event types
	hasBackupStart := false
	hasBackupComplete := false
	for _, event := range events {
		if event.Type == progress.EventBackupStart {
			hasBackupStart = true
		}
		if event.Type == progress.EventBackupComplete {
			hasBackupComplete = true
		}
	}
	
	if !hasBackupStart {
		t.Fatal("Expected backup start event")
	}
	
	if !hasBackupComplete {
		t.Fatal("Expected backup complete event")
	}
}

func TestEngine_Deduplication(t *testing.T) {
	tmpDir := t.TempDir()
	backupDir := filepath.Join(tmpDir, "backups")
	testDataDir := filepath.Join(tmpDir, "test_data")
	
	// Create test data with duplicate content
	err := os.MkdirAll(testDataDir, 0755)
	if err != nil {
		t.Fatalf("Failed to create test data dir: %v", err)
	}
	
	duplicateContent := []byte("This content is duplicated")
	
	err = os.WriteFile(filepath.Join(testDataDir, "file1.txt"), duplicateContent, 0644)
	if err != nil {
		t.Fatalf("Failed to create test file 1: %v", err)
	}
	
	err = os.WriteFile(filepath.Join(testDataDir, "file2.txt"), duplicateContent, 0644)
	if err != nil {
		t.Fatalf("Failed to create test file 2: %v", err)
	}
	
	// Create engine and run backup
	engine := NewEngine(backupDir)
	
	err = engine.Run(testDataDir)
	if err != nil {
		t.Fatalf("Backup run failed: %v", err)
	}
	
	// Get the snapshot
	snapshots, err := engine.ListSnapshots()
	if err != nil {
		t.Fatalf("ListSnapshots failed: %v", err)
	}
	
	if len(snapshots) == 0 {
		t.Fatal("Expected at least one snapshot")
	}
	
	snapshot := snapshots[0]
	
	// Both files should have the same chunk hash (deduplication)
	file1 := snapshot.Files["file1.txt"]
	file2 := snapshot.Files["file2.txt"]
	
	if file1 == nil || file2 == nil {
		t.Fatal("Both files should exist in snapshot")
	}
	
	if len(file1.Chunks) == 0 || len(file2.Chunks) == 0 {
		t.Fatal("Both files should have chunks")
	}
	
	if file1.Chunks[0].Hash != file2.Chunks[0].Hash {
		t.Fatal("Duplicate files should have the same chunk hash (deduplication failed)")
	}
}

func TestEngine_RestoreFile(t *testing.T) {
	tmpDir := t.TempDir()
	backupDir := filepath.Join(tmpDir, "backups")
	testDataDir := filepath.Join(tmpDir, "test_data")
	restoreDir := filepath.Join(tmpDir, "restore")
	
	// Create test data
	err := os.MkdirAll(testDataDir, 0755)
	if err != nil {
		t.Fatalf("Failed to create test data dir: %v", err)
	}
	
	originalContent := []byte("This is the original file content")
	originalFile := filepath.Join(testDataDir, "test.txt")
	
	err = os.WriteFile(originalFile, originalContent, 0644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}
	
	// Create engine and run backup
	engine := NewEngine(backupDir)
	
	err = engine.Run(testDataDir)
	if err != nil {
		t.Fatalf("Backup run failed: %v", err)
	}
	
	// Get snapshot ID
	snapshots, err := engine.ListSnapshots()
	if err != nil {
		t.Fatalf("ListSnapshots failed: %v", err)
	}
	
	if len(snapshots) == 0 {
		t.Fatal("Expected at least one snapshot")
	}
	
	snapshotID := snapshots[0].ID
	
	// Restore the file
	restoredFile := filepath.Join(restoreDir, "restored.txt")
	err = engine.RestoreFile(snapshotID, "test.txt", restoredFile)
	if err != nil {
		t.Fatalf("RestoreFile failed: %v", err)
	}
	
	// Verify restored content
	restoredContent, err := os.ReadFile(restoredFile)
	if err != nil {
		t.Fatalf("Failed to read restored file: %v", err)
	}
	
	if string(restoredContent) != string(originalContent) {
		t.Fatalf("Restored content doesn't match original. Expected %s, got %s", originalContent, restoredContent)
	}
}