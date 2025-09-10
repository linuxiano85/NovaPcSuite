use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration management for plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    plugin_configs: HashMap<String, serde_json::Value>,
    config_dir: PathBuf,
}

impl PluginConfig {
    pub fn new() -> Self {
        Self {
            plugin_configs: HashMap::new(),
            config_dir: Self::default_config_dir(),
        }
    }

    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self {
            plugin_configs: HashMap::new(),
            config_dir,
        }
    }

    /// Get configuration for a specific plugin
    pub fn get_plugin_config(&self, plugin_id: &str) -> Option<&serde_json::Value> {
        self.plugin_configs.get(plugin_id)
    }

    /// Set configuration for a specific plugin
    pub fn set_plugin_config(&mut self, plugin_id: String, config: serde_json::Value) {
        self.plugin_configs.insert(plugin_id, config);
    }

    /// Remove configuration for a specific plugin
    pub fn remove_plugin_config(&mut self, plugin_id: &str) -> Option<serde_json::Value> {
        self.plugin_configs.remove(plugin_id)
    }

    /// Load configuration from disk
    pub async fn load(&mut self) -> anyhow::Result<()> {
        let config_file = self.config_dir.join("plugins.json");
        
        if !config_file.exists() {
            // Create default config directory
            tokio::fs::create_dir_all(&self.config_dir).await?;
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&config_file).await?;
        let loaded_config: HashMap<String, serde_json::Value> = serde_json::from_str(&content)?;
        self.plugin_configs = loaded_config;
        
        tracing::info!("Loaded plugin configurations from {:?}", config_file);
        Ok(())
    }

    /// Save configuration to disk
    pub async fn save(&self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.config_dir).await?;
        
        let config_file = self.config_dir.join("plugins.json");
        let content = serde_json::to_string_pretty(&self.plugin_configs)?;
        tokio::fs::write(&config_file, content).await?;
        
        tracing::info!("Saved plugin configurations to {:?}", config_file);
        Ok(())
    }

    /// Get list of plugins with configuration
    pub fn configured_plugins(&self) -> Vec<&String> {
        self.plugin_configs.keys().collect()
    }

    /// Clear all plugin configurations
    pub fn clear(&mut self) {
        self.plugin_configs.clear();
    }

    fn default_config_dir() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("nova-pc-suite").join("plugins")
        } else {
            PathBuf::from(".config").join("nova-pc-suite").join("plugins")
        }
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin-specific configuration schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigSchema {
    pub title: String,
    pub description: Option<String>,
    pub properties: HashMap<String, ConfigProperty>,
    pub required: Vec<String>,
}

/// Configuration property definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProperty {
    #[serde(rename = "type")]
    pub property_type: ConfigPropertyType,
    pub title: String,
    pub description: Option<String>,
    pub default: Option<serde_json::Value>,
    pub enum_values: Option<Vec<serde_json::Value>>,
}

/// Types of configuration properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigPropertyType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_plugin_config_operations() {
        let mut config = PluginConfig::new();
        
        // Set plugin config
        let plugin_config = serde_json::json!({
            "enabled": true,
            "settings": {
                "timeout": 30,
                "retries": 3
            }
        });
        
        config.set_plugin_config("test-plugin".to_string(), plugin_config.clone());
        
        // Get plugin config
        let retrieved = config.get_plugin_config("test-plugin").unwrap();
        assert_eq!(retrieved, &plugin_config);
        
        // Check configured plugins
        let configured = config.configured_plugins();
        assert_eq!(configured.len(), 1);
        assert_eq!(configured[0], "test-plugin");
    }

    #[tokio::test]
    async fn test_config_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_path_buf();
        
        // Create and save config
        let mut config = PluginConfig::with_config_dir(config_dir.clone());
        config.set_plugin_config(
            "test-plugin".to_string(),
            serde_json::json!({"test": "value"}),
        );
        config.save().await.unwrap();
        
        // Load config in new instance
        let mut new_config = PluginConfig::with_config_dir(config_dir);
        new_config.load().await.unwrap();
        
        let retrieved = new_config.get_plugin_config("test-plugin").unwrap();
        assert_eq!(retrieved["test"], "value");
    }
}