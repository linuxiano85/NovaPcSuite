// Copyright 2025 linuxiano85
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub backup: BackupConfig,
    pub ui: UiConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub default_backup_dir: PathBuf,
    pub compression_enabled: bool,
    pub verify_checksums: bool,
    pub max_parallel_operations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub remember_window_size: bool,
    pub auto_scan_on_device_connect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_enabled: bool,
    pub console_enabled: bool,
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));

        Self {
            backup: BackupConfig {
                default_backup_dir: home_dir.join("NovaBackups"),
                compression_enabled: true,
                verify_checksums: true,
                max_parallel_operations: 4,
            },
            ui: UiConfig {
                theme: "dark".to_string(),
                remember_window_size: true,
                auto_scan_on_device_connect: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file_enabled: true,
                console_enabled: true,
            },
        }
    }
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config = toml::from_str(&content)
                .map_err(|e| crate::Error::Config(format!("Failed to parse config: {}", e)))?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::Error::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(&config_path, content)?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("", "", "novapcsuite").ok_or_else(|| {
            crate::Error::Config("Could not determine config directory".to_string())
        })?;

        Ok(project_dirs.config_dir().join("config.toml"))
    }
}
