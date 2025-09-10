package chunk

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"github.com/zeebo/blake3"
)

const (
	ChunkSize = 64 * 1024 // 64KB chunks
)

// Chunk represents a content-addressed chunk of data
type Chunk struct {
	Hash string `json:"hash"`
	Size int64  `json:"size"`
	Path string `json:"path"`
}

// Store manages content-addressed chunks
type Store struct {
	rootPath string
}

// NewStore creates a new chunk store
func NewStore(rootPath string) *Store {
	return &Store{
		rootPath: rootPath,
	}
}

// Init initializes the chunk store directory structure
func (s *Store) Init() error {
	chunksDir := filepath.Join(s.rootPath, "chunks")
	
	// Create base chunks directory
	if err := os.MkdirAll(chunksDir, 0755); err != nil {
		return fmt.Errorf("failed to create chunks directory: %w", err)
	}
	
	// Create subdirectories aa-ff for first two hex chars
	for i := 0; i < 256; i++ {
		subdir := fmt.Sprintf("%02x", i)
		subdirPath := filepath.Join(chunksDir, subdir)
		if err := os.MkdirAll(subdirPath, 0755); err != nil {
			return fmt.Errorf("failed to create chunk subdir %s: %w", subdir, err)
		}
	}
	
	return nil
}

// Store stores data as a chunk and returns its hash
func (s *Store) Store(data []byte) (*Chunk, error) {
	// Calculate BLAKE3 hash
	hasher := blake3.New()
	hasher.Write(data)
	hash := hex.EncodeToString(hasher.Sum(nil))
	
	// Create chunk path: chunks/aa/aabbcc...
	subdir := hash[:2]
	chunkPath := filepath.Join(s.rootPath, "chunks", subdir, hash)
	
	// Check if chunk already exists (deduplication)
	if _, err := os.Stat(chunkPath); err == nil {
		return &Chunk{
			Hash: hash,
			Size: int64(len(data)),
			Path: chunkPath,
		}, nil
	}
	
	// Write chunk to disk
	if err := os.WriteFile(chunkPath, data, 0644); err != nil {
		return nil, fmt.Errorf("failed to write chunk %s: %w", hash, err)
	}
	
	return &Chunk{
		Hash: hash,
		Size: int64(len(data)),
		Path: chunkPath,
	}, nil
}

// Get retrieves a chunk by its hash
func (s *Store) Get(hash string) ([]byte, error) {
	if len(hash) < 2 {
		return nil, fmt.Errorf("invalid hash length")
	}
	
	subdir := hash[:2]
	chunkPath := filepath.Join(s.rootPath, "chunks", subdir, hash)
	
	data, err := os.ReadFile(chunkPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read chunk %s: %w", hash, err)
	}
	
	return data, nil
}

// Exists checks if a chunk exists
func (s *Store) Exists(hash string) bool {
	if len(hash) < 2 {
		return false
	}
	
	subdir := hash[:2]
	chunkPath := filepath.Join(s.rootPath, "chunks", subdir, hash)
	
	_, err := os.Stat(chunkPath)
	return err == nil
}

// ChunkReader splits an io.Reader into chunks
func (s *Store) ChunkReader(reader io.Reader) ([]*Chunk, error) {
	var chunks []*Chunk
	buffer := make([]byte, ChunkSize)
	
	for {
		n, err := reader.Read(buffer)
		if n > 0 {
			chunkData := make([]byte, n)
			copy(chunkData, buffer[:n])
			
			chunk, chunkErr := s.Store(chunkData)
			if chunkErr != nil {
				return nil, fmt.Errorf("failed to store chunk: %w", chunkErr)
			}
			chunks = append(chunks, chunk)
		}
		
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, fmt.Errorf("error reading data: %w", err)
		}
	}
	
	return chunks, nil
}

// ChunkFile splits a file into chunks
func (s *Store) ChunkFile(filePath string) ([]*Chunk, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return nil, fmt.Errorf("failed to open file %s: %w", filePath, err)
	}
	defer file.Close()
	
	return s.ChunkReader(file)
}

// CalculateFileHash calculates Merkle-style hash for a file based on its chunks
func (s *Store) CalculateFileHash(chunks []*Chunk) string {
	if len(chunks) == 0 {
		return ""
	}
	
	if len(chunks) == 1 {
		return chunks[0].Hash
	}
	
	// Build Merkle tree
	hashes := make([]string, len(chunks))
	for i, chunk := range chunks {
		hashes[i] = chunk.Hash
	}
	
	for len(hashes) > 1 {
		nextLevel := make([]string, 0, (len(hashes)+1)/2)
		
		for i := 0; i < len(hashes); i += 2 {
			hasher := sha256.New()
			hasher.Write([]byte(hashes[i]))
			
			if i+1 < len(hashes) {
				hasher.Write([]byte(hashes[i+1]))
			} else {
				// Odd number, duplicate last hash
				hasher.Write([]byte(hashes[i]))
			}
			
			nextLevel = append(nextLevel, hex.EncodeToString(hasher.Sum(nil)))
		}
		
		hashes = nextLevel
	}
	
	return hashes[0]
}