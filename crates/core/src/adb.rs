use crate::{NovaError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub serial: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub package: String,
    pub version_name: Option<String>,
    pub version_code: Option<String>,
    pub source_path: String,
}

pub struct AdbWrapper;

impl AdbWrapper {
    pub fn new() -> Self {
        Self
    }

    /// List connected devices
    pub fn list_devices(&self) -> Result<Vec<Device>> {
        debug!("Listing ADB devices");
        
        let output = Command::new("adb")
            .args(["devices", "-l"])
            .output()
            .map_err(|e| NovaError::Adb(format!("Failed to execute adb devices: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NovaError::Adb(format!("adb devices failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();

        for line in stdout.lines().skip(1) { // Skip "List of devices attached"
            if line.trim().is_empty() {
                continue;
            }
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                devices.push(Device {
                    serial: parts[0].to_string(),
                    state: parts[1].to_string(),
                });
            }
        }

        debug!("Found {} devices", devices.len());
        Ok(devices)
    }

    /// Execute shell command on device
    pub fn shell(&self, serial: &str, command: &str) -> Result<String> {
        debug!("Executing shell command on {}: {}", serial, command);
        
        let output = Command::new("adb")
            .args(["-s", serial, "shell", command])
            .output()
            .map_err(|e| NovaError::Adb(format!("Failed to execute adb shell: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Shell command failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Pull file from device
    pub fn pull(&self, serial: &str, remote_path: &str, local_path: &str) -> Result<()> {
        debug!("Pulling {} to {}", remote_path, local_path);
        
        let output = Command::new("adb")
            .args(["-s", serial, "pull", remote_path, local_path])
            .output()
            .map_err(|e| NovaError::Adb(format!("Failed to execute adb pull: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NovaError::Adb(format!("adb pull failed: {}", stderr)));
        }

        Ok(())
    }

    /// Push file to device
    pub fn push(&self, serial: &str, local_path: &str, remote_path: &str) -> Result<()> {
        debug!("Pushing {} to {}", local_path, remote_path);
        
        let output = Command::new("adb")
            .args(["-s", serial, "push", local_path, remote_path])
            .output()
            .map_err(|e| NovaError::Adb(format!("Failed to execute adb push: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NovaError::Adb(format!("adb push failed: {}", stderr)));
        }

        Ok(())
    }

    /// List installed packages
    pub fn list_packages(&self, serial: &str, user_only: bool) -> Result<Vec<String>> {
        debug!("Listing packages on device {}", serial);
        
        let mut args = vec!["-s", serial, "shell", "pm", "list", "packages"];
        if user_only {
            args.push("-3"); // Only user packages
        }

        let output = Command::new("adb")
            .args(&args)
            .output()
            .map_err(|e| NovaError::Adb(format!("Failed to execute pm list packages: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NovaError::Adb(format!("pm list packages failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                line.strip_prefix("package:")
            })
            .map(|s| s.to_string())
            .collect();

        debug!("Found {} packages", packages.len());
        Ok(packages)
    }

    /// Get package path
    pub fn get_package_path(&self, serial: &str, package: &str) -> Result<String> {
        debug!("Getting path for package {}", package);
        
        let output = Command::new("adb")
            .args(["-s", serial, "shell", "pm", "path", package])
            .output()
            .map_err(|e| NovaError::Adb(format!("Failed to execute pm path: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NovaError::Adb(format!("pm path failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().next() {
            if let Some(path) = line.strip_prefix("package:") {
                return Ok(path.to_string());
            }
        }

        Err(NovaError::Adb(format!("Could not find path for package {}", package)))
    }

    /// Get device properties
    pub fn getprop(&self, serial: &str, property: Option<&str>) -> Result<String> {
        let mut args = vec!["-s", serial, "shell", "getprop"];
        if let Some(prop) = property {
            args.push(prop);
        }

        debug!("Getting device properties");
        
        let output = Command::new("adb")
            .args(&args)
            .output()
            .map_err(|e| NovaError::Adb(format!("Failed to execute getprop: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NovaError::Adb(format!("getprop failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}