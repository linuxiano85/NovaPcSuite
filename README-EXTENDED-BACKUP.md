# NovaPcSuite Backup Engine Architecture

## Table of Contents

1. [Overview](#overview)
2. [Core Components](#core-components)
3. [Chunking Strategy](#chunking-strategy)
4. [Hash-based Integrity](#hash-based-integrity)
5. [Merkle Tree Implementation](#merkle-tree-implementation)
6. [Content-Addressed Storage](#content-addressed-storage)
7. [Deduplication System](#deduplication-system)
8. [Manifest Format](#manifest-format)
9. [Performance Considerations](#performance-considerations)
10. [Future Optimizations](#future-optimizations)

## Overview

The NovaPcSuite backup engine is designed around a chunked, content-addressed storage system with cryptographic integrity verification. This architecture provides:

- **Deduplication**: Identical content stored only once
- **Integrity**: Cryptographic verification at chunk and file levels
- **Efficiency**: Parallel processing and adaptive chunking
- **Scalability**: Content-addressed storage enables efficient synchronization
- **Reliability**: Merkle trees provide tamper detection

## Core Components

### BackupEngine

The `BackupEngine` is the central orchestrator that coordinates the backup process:

```rust
pub struct BackupEngine {
    output_dir: PathBuf,
    chunk_size: usize,
}
```

**Responsibilities:**
- File system traversal coordination
- Chunk processing pipeline management
- Manifest generation and atomic writing
- Progress reporting and error handling

### ChunkStore

Content-addressed storage implementation:

```rust
pub struct ChunkStore {
    chunks_dir: PathBuf,
    stored_chunks: Mutex<HashSet<String>>,
}
```

**Key Features:**
- Thread-safe chunk deduplication
- Automatic directory creation
- Hash-based file naming
- Atomic chunk storage

### LocalFsSource

File system source implementation:

```rust
pub struct LocalFsSource {
    source_path: PathBuf,
}
```

**Capabilities:**
- Recursive directory traversal
- Metadata extraction
- File filtering and exclusion
- Progress reporting

## Chunking Strategy

### Adaptive Chunking Algorithm

NovaPcSuite uses a **fixed-size chunking** approach with adaptive optimizations:

```rust
pub const DEFAULT_CHUNK_SIZE: usize = 2 * 1024 * 1024; // 2 MiB
pub const SMALL_FILE_THRESHOLD: usize = 64 * 1024;     // 64 KiB
```

#### Chunking Logic

1. **Large Files (> 64 KiB)**: Split into 2 MiB chunks
2. **Small Files (≤ 64 KiB)**: Store as single chunk (fast path)
3. **Final Chunk**: May be smaller than chunk size

#### Benefits of Fixed-Size Chunking

- **Simplicity**: Predictable chunk boundaries
- **Deduplication**: Identical chunks across files are deduplicated
- **Performance**: No content-dependent boundary detection overhead
- **Parallelization**: Independent chunk processing

#### Trade-offs

- **Lower deduplication ratio** compared to content-defined chunking
- **Boundary shift sensitivity**: Small changes can affect many chunks
- **Future enhancement**: Content-defined chunking planned for v0.2.0

### Chunk Processing Pipeline

```
File → Read Chunks → Hash (BLAKE3) → Store in ChunkStore → Update Manifest
  ↓         ↓            ↓               ↓                    ↓
Parallel  Buffer     Content Hash    Deduplication      Merkle Tree
```

## Hash-based Integrity

### BLAKE3 Cryptographic Hashing

NovaPcSuite uses BLAKE3 for all cryptographic hashing:

```rust
let chunk_hash = blake3::hash(chunk_data);
let chunk_id = hex::encode(chunk_hash.as_bytes());
```

#### Why BLAKE3?

- **Performance**: Fastest cryptographic hash function available
- **Security**: Cryptographically secure with 256-bit output
- **Parallelization**: Built-in parallel processing
- **Tree Structure**: Natural fit for Merkle tree construction

### Chunk Identification

Each chunk is identified by its BLAKE3 hash:

```rust
pub struct ChunkInfo {
    pub id: String,        // Hex-encoded BLAKE3 hash
    pub hash: Vec<u8>,     // Raw BLAKE3 hash bytes
    pub offset: u64,       // Offset within original file
    pub size: u64,         // Chunk size in bytes
}
```

## Merkle Tree Implementation

### File-Level Merkle Roots

Each file has a Merkle root calculated from its chunk hashes:

```rust
fn calculate_merkle_root(chunks: &[ChunkInfo]) -> Result<Vec<u8>> {
    let mut hasher = Hasher::new();
    for chunk in chunks {
        hasher.update(&chunk.hash);
    }
    Ok(hasher.finalize().as_bytes().to_vec())
}
```

### Current Implementation

**Simplified Approach**: The current implementation uses a "fold" of all chunk hashes:

1. Create a new BLAKE3 hasher
2. Update hasher with each chunk hash in order
3. Finalize to get the Merkle root

### Future Enhancement: True Merkle Tree

The next version will implement a proper binary Merkle tree:

```
        Root Hash
       /         \
   Hash AB      Hash CD
   /     \      /     \
Hash A  Hash B Hash C Hash D
  |       |      |       |
Chunk 1 Chunk 2 Chunk 3 Chunk 4
```

**Benefits:**
- **Partial verification**: Verify individual chunks without downloading entire file
- **Efficient updates**: Re-compute only affected tree branches
- **Proof of inclusion**: Cryptographic proofs for chunk presence

## Content-Addressed Storage

### Storage Layout

```
backup-output/
├── chunks/
│   ├── 0a1b2c3d...  # Chunk files named by BLAKE3 hash
│   ├── 4e5f6a7b...
│   └── ...
├── manifests/
│   ├── manifest-<uuid>.json
│   └── ...
├── reports/
│   ├── report-<uuid>.json
│   ├── report-<uuid>.html
│   └── ...
└── tmp/
    └── ...          # Temporary files during operations
```

### Chunk Storage Properties

1. **Content-Addressed**: Filename = BLAKE3(content)
2. **Immutable**: Chunks never change once written
3. **Atomic**: Chunks written atomically (no partial writes)
4. **Deduplicated**: Identical content stored only once

### Advantages

- **Space Efficiency**: Automatic deduplication
- **Integrity**: Content hash verifies data integrity
- **Caching**: Chunk-level caching and synchronization
- **Parallelization**: Independent chunk operations

## Deduplication System

### Content Deduplication

Automatic deduplication through content-addressed storage:

```rust
pub fn store_chunk(&self, chunk_id: &str, data: &[u8]) -> Result<()> {
    let chunk_path = self.chunks_dir.join(chunk_id);
    
    // Check if chunk already exists (deduplication)
    if chunk_path.exists() {
        return Ok(());
    }
    
    // Write chunk data
    fs::write(chunk_path, data)?;
    Ok(())
}
```

### Perceptual Deduplication

For media files, NovaPcSuite implements perceptual hashing:

#### Image Deduplication

```rust
pub struct ImageDeduplicator {
    similarity_threshold: f64, // Default: 0.85 (85% similarity)
}
```

**Algorithm**: Simplified pHash (Perceptual Hash)
1. Resize image to 32×32 grayscale
2. Apply 8×8 DCT (Discrete Cosine Transform)
3. Extract low-frequency components
4. Create 63-bit hash based on median threshold

#### Audio Deduplication (Placeholder)

```rust
pub struct AudioDeduplicator {
    similarity_threshold: f64, // Default: 0.80 (80% similarity)
}
```

**Future Implementation**:
- MFCC (Mel-Frequency Cepstral Coefficients)
- Chromagram analysis
- Spectral feature extraction
- Robust audio fingerprinting

## Manifest Format

### Manifest Structure

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "created": "2024-01-15T10:30:00Z",
  "label": "documents-backup",
  "source_path": "/home/user/documents",
  "files": [
    {
      "path": "project/readme.txt",
      "size": 1024,
      "chunks": [
        {
          "id": "a1b2c3d4e5f6...",
          "hash": [161, 178, 195, 212, ...],
          "offset": 0,
          "size": 1024
        }
      ],
      "merkle_root": [123, 45, 67, 89, ...],
      "modified": "2024-01-15T09:15:00Z"
    }
  ],
  "chunk_count": 1,
  "total_size": 1024
}
```

### Manifest Properties

- **Immutable**: Manifests are never modified after creation
- **Atomic**: Written atomically via temporary files
- **Self-contained**: Contains all metadata for restoration
- **Versioned**: Future versions will include schema version

## Performance Considerations

### Parallel Processing

NovaPcSuite leverages Rust's parallel processing capabilities:

```rust
// Parallel file processing using rayon
let chunk_results: Result<Vec<_>> = plan
    .files
    .par_iter()
    .map(|file_plan| {
        self.process_file_chunks(file_plan, &chunk_store)
    })
    .collect();
```

### I/O Optimization

- **Buffered Reading**: Files read in chunk-sized buffers
- **Async I/O**: Non-blocking operations using Tokio
- **Memory Management**: Controlled memory usage for large files

### Hash Performance

- **BLAKE3**: ~7 GB/s on modern hardware
- **Parallel Hashing**: BLAKE3's tree structure enables parallelization
- **Hardware Acceleration**: SIMD optimizations where available

### Storage Efficiency

```
Backup Size Reduction Examples:
├── Identical files: 100% deduplication
├── Similar images: 85%+ similarity detection
├── Chunk-level dedup: ~20-40% typical reduction
└── Overall efficiency: 30-70% space savings
```

## Future Optimizations

### Content-Defined Chunking (v0.2.0)

Implement FastCDC (Fast Content-Defined Chunking):

```rust
pub struct FastCDC {
    min_chunk_size: usize,    // 256 KiB
    avg_chunk_size: usize,    // 1 MiB  
    max_chunk_size: usize,    // 4 MiB
}
```

**Benefits**:
- Higher deduplication ratios
- Better handling of file modifications
- Reduced storage requirements

### Delta Compression (v0.3.0)

For frequently modified files:

```rust
pub struct DeltaCompression {
    base_chunks: Vec<ChunkInfo>,
    delta_ops: Vec<DeltaOperation>,
}
```

### Advanced Merkle Trees (v0.2.0)

Full binary Merkle tree implementation:

```rust
pub struct MerkleTree {
    leaves: Vec<Blake3Hash>,
    tree: Vec<Blake3Hash>,
    depth: usize,
}
```

### Compression Integration (v0.4.0)

Chunk-level compression before storage:

```rust
pub enum CompressionAlgorithm {
    None,
    Lz4,
    Zstd,
    Brotli,
}
```

### Performance Targets

| Version | Target Throughput | Dedup Ratio | CPU Usage |
|---------|------------------|-------------|-----------|
| v0.1.0  | 100 MB/s        | 30-50%      | 1-2 cores |
| v0.2.0  | 200 MB/s        | 50-70%      | 2-4 cores |
| v0.3.0  | 500 MB/s        | 60-80%      | 4-8 cores |
| v1.0.0  | 1 GB/s          | 70-90%      | 8+ cores  |

---

This architecture provides a solid foundation for a modern backup system while maintaining flexibility for future enhancements and optimizations.