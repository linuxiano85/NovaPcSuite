use serde::{Deserialize, Serialize};
use semver::Version;
use std::collections::HashMap;

/// Plugin descriptor parsed from nova_plugin.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub api_version: u32,
    pub authors: Vec<String>,
    pub description: String,
    pub categories: Vec<PluginCategory>,
    pub capabilities: super::PluginCapabilities,
    pub dependencies: HashMap<String, String>,
    pub entry_point: Option<String>,
}

impl PluginDescriptor {
    /// Validate that this plugin descriptor is compatible with the current API
    pub fn validate_compatibility(&self) -> anyhow::Result<()> {
        if self.api_version != super::CURRENT_API_VERSION {
            anyhow::bail!(
                "Plugin {} requires API version {}, but current version is {}",
                self.id,
                self.api_version,
                super::CURRENT_API_VERSION
            );
        }
        
        if self.id.is_empty() {
            anyhow::bail!("Plugin ID cannot be empty");
        }
        
        if self.name.is_empty() {
            anyhow::bail!("Plugin name cannot be empty");
        }
        
        Ok(())
    }
}

/// Categories of plugins
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginCategory {
    #[serde(rename = "backup")]
    Backup,
    #[serde(rename = "ui")]
    UI,
    #[serde(rename = "analysis")]
    Analysis,
    #[serde(rename = "transport")]
    Transport,
    #[serde(rename = "crypto")]
    Crypto,
    #[serde(rename = "integration")]
    Integration,
}

/// Parse plugin descriptor from TOML content
pub fn parse_plugin_descriptor(toml_content: &str) -> anyhow::Result<PluginDescriptor> {
    let descriptor: PluginDescriptor = toml::from_str(toml_content)?;
    descriptor.validate_compatibility()?;
    Ok(descriptor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginCapabilities;

    #[test]
    fn test_parse_valid_descriptor() {
        let toml_content = r#"
id = "example-plugin"
name = "Example Plugin"
version = "1.0.0"
api_version = 1
authors = ["Test Author"]
description = "A test plugin"
categories = ["backup", "ui"]

[capabilities]
file_system_access = true
network_access = false
system_info_access = true
backup_events = true
ui_panels = true
config_ui = false

[dependencies]
some-dep = "1.0"
"#;

        let descriptor = parse_plugin_descriptor(toml_content).unwrap();
        assert_eq!(descriptor.id, "example-plugin");
        assert_eq!(descriptor.name, "Example Plugin");
        assert_eq!(descriptor.api_version, 1);
        assert!(descriptor.capabilities.file_system_access);
        assert!(!descriptor.capabilities.network_access);
    }

    #[test]
    fn test_invalid_api_version() {
        let toml_content = r#"
id = "example-plugin"
name = "Example Plugin"
version = "1.0.0"
api_version = 999
authors = ["Test Author"]
description = "A test plugin"
categories = ["backup"]

[capabilities]
file_system_access = false
network_access = false
system_info_access = false
backup_events = false
ui_panels = false
config_ui = false

[dependencies]
"#;

        let result = parse_plugin_descriptor(toml_content);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // The error should be about API version compatibility
        assert!(error_msg.contains("requires API version 999") || error_msg.contains("current version is 1"));
    }
}