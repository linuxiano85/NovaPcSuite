//! Image deduplication using perceptual hashing.
//! 
//! This module implements simplified perceptual hashing (pHash) for detecting
//! similar images that may have been resized, compressed, or slightly modified.

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::{ClusterType, SimilarityCluster};

/// Image deduplicator using perceptual hashing
#[derive(Debug)]
pub struct ImageDeduplicator {
    similarity_threshold: f64,
}

impl ImageDeduplicator {
    /// Create a new image deduplicator
    pub fn new() -> Self {
        Self {
            similarity_threshold: 0.85, // 85% similarity threshold
        }
    }

    /// Set similarity threshold (0.0 to 1.0)
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Analyze an image file and compute its perceptual hash
    pub fn analyze(&self, image_path: &Path) -> PerceptualHash {
        match self.compute_phash(image_path) {
            Ok(hash) => hash,
            Err(e) => {
                eprintln!("Error analyzing image {:?}: {}", image_path, e);
                PerceptualHash::default()
            }
        }
    }

    /// Compute perceptual hash for an image
    fn compute_phash(&self, image_path: &Path) -> anyhow::Result<PerceptualHash> {
        let img = image::open(image_path)
            .with_context(|| format!("Failed to open image: {:?}", image_path))?;

        // Simplified pHash implementation:
        // 1. Resize to 32x32 grayscale
        // 2. Compute DCT (simplified version)
        // 3. Extract low-frequency components
        // 4. Create binary hash

        let resized = img.resize_exact(32, 32, image::imageops::FilterType::Lanczos3);
        let gray = resized.to_luma8();

        // Convert to f64 matrix for DCT computation
        let mut matrix = vec![vec![0.0f64; 32]; 32];
        for (x, y, pixel) in gray.enumerate_pixels() {
            matrix[y as usize][x as usize] = pixel[0] as f64;
        }

        // Apply simplified 2D DCT (just the low-frequency 8x8 corner)
        let dct_matrix = self.simple_dct_2d(&matrix, 8, 8);

        // Compute median of DCT coefficients (excluding DC component)
        let mut coeffs = Vec::new();
        for i in 0..8 {
            for j in 0..8 {
                if i != 0 || j != 0 {
                    // Skip DC component
                    coeffs.push(dct_matrix[i][j]);
                }
            }
        }
        coeffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = coeffs[coeffs.len() / 2];

        // Create binary hash based on median threshold
        let mut hash_bits = Vec::new();
        for i in 0..8 {
            for j in 0..8 {
                if i != 0 || j != 0 {
                    hash_bits.push(dct_matrix[i][j] > median);
                }
            }
        }

        Ok(PerceptualHash { 
            bits: hash_bits,
            image_path: image_path.to_path_buf(),
        })
    }

    /// Simplified 2D DCT implementation (not optimized, for demonstration)
    fn simple_dct_2d(&self, matrix: &[Vec<f64>], width: usize, height: usize) -> Vec<Vec<f64>> {
        let mut result = vec![vec![0.0; width]; height];

        for u in 0..height {
            for v in 0..width {
                let mut sum = 0.0;
                for x in 0..height {
                    for y in 0..width {
                        let cos_u = ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / (2.0 * height as f64)).cos();
                        let cos_v = ((2 * y + 1) as f64 * v as f64 * std::f64::consts::PI / (2.0 * width as f64)).cos();
                        sum += matrix[x][y] * cos_u * cos_v;
                    }
                }

                let alpha_u = if u == 0 { 1.0 / (height as f64).sqrt() } else { (2.0 / height as f64).sqrt() };
                let alpha_v = if v == 0 { 1.0 / (width as f64).sqrt() } else { (2.0 / width as f64).sqrt() };

                result[u][v] = alpha_u * alpha_v * sum;
            }
        }

        result
    }

    /// Find clusters of similar images
    pub fn find_similar_images(&self, images: &[(PathBuf, PerceptualHash)]) -> Vec<SimilarityCluster> {
        let mut clusters = Vec::new();
        let mut processed = vec![false; images.len()];

        for i in 0..images.len() {
            if processed[i] {
                continue;
            }

            let mut cluster_files = vec![images[i].0.clone()];
            processed[i] = true;

            for j in (i + 1)..images.len() {
                if processed[j] {
                    continue;
                }

                let similarity = images[i].1.similarity(&images[j].1);
                if similarity >= self.similarity_threshold {
                    cluster_files.push(images[j].0.clone());
                    processed[j] = true;
                }
            }

            // Only create cluster if it has more than one file
            if cluster_files.len() > 1 {
                clusters.push(SimilarityCluster {
                    cluster_type: ClusterType::Image,
                    files: cluster_files,
                    similarity_score: self.similarity_threshold,
                });
            }
        }

        clusters
    }
}

impl Default for ImageDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Perceptual hash for an image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptualHash {
    pub bits: Vec<bool>,
    pub image_path: PathBuf,
}

impl PerceptualHash {
    /// Calculate similarity between two perceptual hashes (0.0 to 1.0)
    pub fn similarity(&self, other: &Self) -> f64 {
        if self.bits.len() != other.bits.len() {
            return 0.0;
        }

        let matching_bits = self.bits
            .iter()
            .zip(&other.bits)
            .filter(|(a, b)| a == b)
            .count();

        matching_bits as f64 / self.bits.len() as f64
    }

    /// Calculate Hamming distance between hashes
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        if self.bits.len() != other.bits.len() {
            return u32::MAX;
        }

        self.bits
            .iter()
            .zip(&other.bits)
            .filter(|(a, b)| a != b)
            .count() as u32
    }
}

impl Default for PerceptualHash {
    fn default() -> Self {
        Self {
            bits: vec![false; 63], // 8x8 - 1 (excluding DC component)
            image_path: PathBuf::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perceptual_hash_similarity() {
        let hash1 = PerceptualHash {
            bits: vec![true, false, true, false],
            image_path: PathBuf::from("test1.jpg"),
        };

        let hash2 = PerceptualHash {
            bits: vec![true, false, true, false],
            image_path: PathBuf::from("test2.jpg"),
        };

        let hash3 = PerceptualHash {
            bits: vec![false, true, false, true],
            image_path: PathBuf::from("test3.jpg"),
        };

        assert_eq!(hash1.similarity(&hash2), 1.0);
        assert_eq!(hash1.similarity(&hash3), 0.0);
        assert_eq!(hash1.hamming_distance(&hash2), 0);
        assert_eq!(hash1.hamming_distance(&hash3), 4);
    }

    #[test]
    fn test_image_deduplicator_creation() {
        let dedup = ImageDeduplicator::new();
        assert_eq!(dedup.similarity_threshold, 0.85);

        let dedup_custom = ImageDeduplicator::new().with_threshold(0.9);
        assert_eq!(dedup_custom.similarity_threshold, 0.9);
    }

    #[test]
    fn test_similarity_clustering() {
        let dedup = ImageDeduplicator::new().with_threshold(0.8);

        let images = vec![
            (PathBuf::from("img1.jpg"), PerceptualHash {
                bits: vec![true; 63],
                image_path: PathBuf::from("img1.jpg"),
            }),
            (PathBuf::from("img2.jpg"), PerceptualHash {
                bits: vec![true; 63],
                image_path: PathBuf::from("img2.jpg"),
            }),
            (PathBuf::from("img3.jpg"), PerceptualHash {
                bits: vec![false; 63],
                image_path: PathBuf::from("img3.jpg"),
            }),
        ];

        let clusters = dedup.find_similar_images(&images);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].files.len(), 2);
    }
}