package chunk

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

func TestStore_Init(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(tmpDir)
	
	err := store.Init()
	if err != nil {
		t.Fatalf("Init failed: %v", err)
	}
	
	// Check that chunk directories were created
	chunksDir := filepath.Join(tmpDir, "chunks")
	if _, err := os.Stat(chunksDir); os.IsNotExist(err) {
		t.Fatal("chunks directory was not created")
	}
	
	// Check that subdirectories 00-ff were created
	for i := 0; i < 256; i++ {
		subdir := filepath.Join(chunksDir, fmt.Sprintf("%02x", i))
		if _, err := os.Stat(subdir); os.IsNotExist(err) {
			t.Fatalf("chunk subdirectory %02x was not created", i)
		}
	}
}

func TestStore_StoreAndGet(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(tmpDir)
	
	err := store.Init()
	if err != nil {
		t.Fatalf("Init failed: %v", err)
	}
	
	testData := []byte("Hello, World!")
	
	// Store the chunk
	chunk, err := store.Store(testData)
	if err != nil {
		t.Fatalf("Store failed: %v", err)
	}
	
	if chunk == nil {
		t.Fatal("Store returned nil chunk")
	}
	
	if chunk.Size != int64(len(testData)) {
		t.Fatalf("Expected chunk size %d, got %d", len(testData), chunk.Size)
	}
	
	if len(chunk.Hash) != 64 {
		t.Fatalf("Expected hash length 64, got %d", len(chunk.Hash))
	}
	
	// Retrieve the chunk
	retrievedData, err := store.Get(chunk.Hash)
	if err != nil {
		t.Fatalf("Get failed: %v", err)
	}
	
	if !bytes.Equal(testData, retrievedData) {
		t.Fatalf("Retrieved data doesn't match original. Expected %s, got %s", testData, retrievedData)
	}
}

func TestStore_Deduplication(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(tmpDir)
	
	err := store.Init()
	if err != nil {
		t.Fatalf("Init failed: %v", err)
	}
	
	testData := []byte("Duplicate content")
	
	// Store the same data twice
	chunk1, err := store.Store(testData)
	if err != nil {
		t.Fatalf("First store failed: %v", err)
	}
	
	chunk2, err := store.Store(testData)
	if err != nil {
		t.Fatalf("Second store failed: %v", err)
	}
	
	// Should have the same hash (deduplication)
	if chunk1.Hash != chunk2.Hash {
		t.Fatalf("Expected same hash for duplicate content. Got %s and %s", chunk1.Hash, chunk2.Hash)
	}
	
	// Should point to the same file
	if chunk1.Path != chunk2.Path {
		t.Fatalf("Expected same path for duplicate content. Got %s and %s", chunk1.Path, chunk2.Path)
	}
}

func TestStore_ChunkFile(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(tmpDir)
	
	err := store.Init()
	if err != nil {
		t.Fatalf("Init failed: %v", err)
	}
	
	// Create a test file
	testFile := filepath.Join(tmpDir, "test.txt")
	testContent := "This is test content for chunking"
	err = os.WriteFile(testFile, []byte(testContent), 0644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}
	
	// Chunk the file
	chunks, err := store.ChunkFile(testFile)
	if err != nil {
		t.Fatalf("ChunkFile failed: %v", err)
	}
	
	if len(chunks) == 0 {
		t.Fatal("Expected at least one chunk")
	}
	
	// Verify chunks contain the original content
	var reconstructed bytes.Buffer
	for _, chunk := range chunks {
		data, err := store.Get(chunk.Hash)
		if err != nil {
			t.Fatalf("Failed to get chunk %s: %v", chunk.Hash, err)
		}
		reconstructed.Write(data)
	}
	
	if reconstructed.String() != testContent {
		t.Fatalf("Reconstructed content doesn't match original. Expected %s, got %s", testContent, reconstructed.String())
	}
}

func TestStore_CalculateFileHash(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(tmpDir)
	
	err := store.Init()
	if err != nil {
		t.Fatalf("Init failed: %v", err)
	}
	
	// Test with single chunk
	chunk1, err := store.Store([]byte("single chunk"))
	if err != nil {
		t.Fatalf("Store failed: %v", err)
	}
	
	fileHash1 := store.CalculateFileHash([]*Chunk{chunk1})
	if fileHash1 != chunk1.Hash {
		t.Fatalf("Single chunk file hash should equal chunk hash. Expected %s, got %s", chunk1.Hash, fileHash1)
	}
	
	// Test with multiple chunks
	chunk2, err := store.Store([]byte("second chunk"))
	if err != nil {
		t.Fatalf("Store failed: %v", err)
	}
	
	fileHash2 := store.CalculateFileHash([]*Chunk{chunk1, chunk2})
	if fileHash2 == "" {
		t.Fatal("Multi-chunk file hash should not be empty")
	}
	
	if fileHash2 == chunk1.Hash || fileHash2 == chunk2.Hash {
		t.Fatal("Multi-chunk file hash should be different from individual chunk hashes")
	}
}