use crate::{NovaError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaConfig {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub backup: BackupConfig,
    pub adb: AdbConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub default_output_dir: Option<String>,
    pub incremental: bool,
    pub verify_hashes: bool,
    pub preserve_timestamps: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdbConfig {
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
}

impl Default for NovaConfig {
    fn default() -> Self {
        Self {
            include: vec![
                "/sdcard/DCIM".to_string(),
                "/sdcard/Pictures".to_string(),
                "/sdcard/Movies".to_string(),
                "/sdcard/Music".to_string(),
                "/sdcard/Documents".to_string(),
                "/sdcard/Download".to_string(),
                "/sdcard/WhatsApp/Media".to_string(),
                "/sdcard/Telegram".to_string(),
                "/sdcard/Recordings".to_string(),
                "/sdcard/MIUI/sound_recorder".to_string(),
            ],
            exclude: vec![
                "**/.thumbdata*".to_string(),
                "**/.thumbnails/*".to_string(),
                "**/cache/*".to_string(),
                "**/tmp/*".to_string(),
            ],
            backup: BackupConfig {
                default_output_dir: None,
                incremental: false,
                verify_hashes: true,
                preserve_timestamps: true,
            },
            adb: AdbConfig {
                timeout_seconds: 30,
                retry_attempts: 3,
            },
        }
    }
}

impl NovaConfig {
    /// Load config from file or create default if not exists
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            debug!("Loading config from {}", config_path.display());
            let content = fs::read_to_string(&config_path)
                .map_err(|e| NovaError::Config(format!("Failed to read config file: {}", e)))?;
            
            serde_yaml::from_str(&content)
                .map_err(|e| NovaError::Config(format!("Failed to parse config file: {}", e)))
        } else {
            debug!("Config file not found, creating default");
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| NovaError::Config(format!("Failed to create config directory: {}", e)))?;
        }

        let content = serde_yaml::to_string(self)
            .map_err(|e| NovaError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(&config_path, content)
            .map_err(|e| NovaError::Config(format!("Failed to write config file: {}", e)))?;

        debug!("Config saved to {}", config_path.display());
        Ok(())
    }

    /// Get the config file path
    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| NovaError::Config("Could not determine config directory".to_string()))?;
        
        Ok(config_dir.join("novapcsuite").join("config.yaml"))
    }

    /// Get effective include directories (config + defaults)
    pub fn get_include_dirs(&self) -> Vec<String> {
        if self.include.is_empty() {
            Self::default().include
        } else {
            self.include.clone()
        }
    }

    /// Get effective exclude patterns (config + defaults) 
    pub fn get_exclude_patterns(&self) -> Vec<String> {
        let mut patterns = self.exclude.clone();
        
        // Always add some basic exclusions
        patterns.extend_from_slice(&[
            "**/.git/*".to_string(),
            "**/.svn/*".to_string(),
            "**/node_modules/*".to_string(),
        ]);

        patterns
    }

    /// Check if a path should be excluded
    pub fn should_exclude(&self, path: &str) -> bool {
        let patterns = self.get_exclude_patterns();
        
        for pattern in &patterns {
            if self.matches_pattern(path, pattern) {
                return true;
            }
        }

        false
    }

    /// Simple pattern matching (supports * and **)
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        // Simple implementation - in a real app you'd want proper glob matching
        if pattern.contains("**") {
            // Recursive match
            let base = pattern.replace("**", "");
            path.contains(&base.trim_matches('/'))
        } else if pattern.contains('*') {
            // Single level wildcard - simplified
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                path.starts_with(parts[0]) && path.ends_with(parts[1])
            } else {
                false
            }
        } else {
            // Exact match
            path == pattern
        }
    }

    /// Get default output directory for backups
    pub fn get_default_output_dir(&self) -> Result<PathBuf> {
        if let Some(ref dir) = self.backup.default_output_dir {
            Ok(PathBuf::from(dir))
        } else {
            // Use user's Documents/NovaPcSuite/backups as default
            let docs_dir = dirs::document_dir()
                .ok_or_else(|| NovaError::Config("Could not determine documents directory".to_string()))?;
            
            Ok(docs_dir.join("NovaPcSuite").join("backups"))
        }
    }

    /// Validate config settings
    pub fn validate(&self) -> Result<()> {
        // Check that include directories are valid paths
        for dir in &self.include {
            if !dir.starts_with('/') {
                warn!("Include directory should be absolute path: {}", dir);
            }
        }

        // Validate backup settings
        if self.adb.timeout_seconds == 0 {
            return Err(NovaError::Config("ADB timeout must be greater than 0".to_string()));
        }

        if self.adb.retry_attempts > 10 {
            warn!("High retry attempts configured: {}", self.adb.retry_attempts);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NovaConfig::default();
        assert!(!config.include.is_empty());
        assert!(config.backup.verify_hashes);
        assert_eq!(config.adb.timeout_seconds, 30);
    }

    #[test]
    fn test_pattern_matching() {
        let config = NovaConfig::default();
        
        // Test exact match
        assert!(config.matches_pattern("/path/cache/file", "**/cache/*"));
        
        // Test wildcard
        assert!(config.matches_pattern("file.thumbdata", "*.thumbdata*"));
        
        // Test non-match
        assert!(!config.matches_pattern("/path/important/file", "**/cache/*"));
    }

    #[test]
    fn test_exclusion() {
        let mut config = NovaConfig::default();
        config.exclude.push("**/test/*".to_string());
        
        assert!(config.should_exclude("/sdcard/test/file.txt"));
        assert!(!config.should_exclude("/sdcard/important/file.txt"));
    }
}