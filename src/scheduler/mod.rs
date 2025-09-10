//! Backup scheduler for automated backup operations.
//! 
//! This module provides functionality to generate systemd service and timer units
//! for automated backup scheduling on Linux systems.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Backup scheduler for managing automated backups
#[derive(Debug)]
pub struct BackupScheduler {
    output_dir: PathBuf,
}

impl BackupScheduler {
    /// Create a new backup scheduler
    pub fn new(output_dir: &Path) -> Self {
        Self {
            output_dir: output_dir.to_path_buf(),
        }
    }

    /// Generate systemd service and timer units for backup scheduling
    pub async fn generate_systemd_units(&self, schedule: &BackupSchedule) -> Result<ScheduleOutput> {
        let systemd_dir = self.output_dir.join("systemd");
        fs::create_dir_all(&systemd_dir).await?;

        let service_content = self.generate_service_unit(schedule)?;
        let timer_content = self.generate_timer_unit(schedule)?;

        let service_path = systemd_dir.join(format!("{}.service", schedule.name));
        let timer_path = systemd_dir.join(format!("{}.timer", schedule.name));

        fs::write(&service_path, service_content).await?;
        fs::write(&timer_path, timer_content).await?;

        println!("Generated systemd units:");
        println!("  Service: {}", service_path.display());
        println!("  Timer: {}", timer_path.display());
        println!();
        println!("To install and enable:");
        println!("  sudo cp {} /etc/systemd/system/", service_path.display());
        println!("  sudo cp {} /etc/systemd/system/", timer_path.display());
        println!("  sudo systemctl daemon-reload");
        println!("  sudo systemctl enable {}.timer", schedule.name);
        println!("  sudo systemctl start {}.timer", schedule.name);

        Ok(ScheduleOutput {
            service_path: service_path.clone(),
            timer_path: timer_path.clone(),
            install_commands: vec![
                format!("sudo cp {} /etc/systemd/system/", service_path.display()),
                format!("sudo cp {} /etc/systemd/system/", timer_path.display()),
                "sudo systemctl daemon-reload".to_string(),
                format!("sudo systemctl enable {}.timer", schedule.name),
                format!("sudo systemctl start {}.timer", schedule.name),
            ],
        })
    }

    /// Generate systemd service unit content
    fn generate_service_unit(&self, schedule: &BackupSchedule) -> Result<String> {
        let service_content = format!(
            r#"[Unit]
Description=NovaPcSuite Backup - {}
After=network.target

[Service]
Type=oneshot
User={}
Group={}
ExecStart={} backup --source {} --output {} --label "{}"{}
Environment=HOME={}
WorkingDirectory={}

# Resource limits
MemoryMax={}
CPUQuota={}%

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=nova-pc-suite-{}

[Install]
WantedBy=multi-user.target
"#,
            schedule.description,
            schedule.user,
            schedule.group,
            schedule.executable_path.display(),
            schedule.source_path.display(),
            schedule.output_path.display(),
            schedule.label,
            if schedule.generate_report { " --generate-report" } else { "" },
            schedule.home_directory.display(),
            schedule.working_directory.display(),
            schedule.memory_limit,
            schedule.cpu_quota,
            schedule.name
        );

        Ok(service_content)
    }

    /// Generate systemd timer unit content
    fn generate_timer_unit(&self, schedule: &BackupSchedule) -> Result<String> {
        let timer_content = format!(
            r#"[Unit]
Description=Timer for NovaPcSuite Backup - {}
Requires={}.service

[Timer]
OnCalendar={}
Persistent=true
RandomizedDelaySec={}

[Install]
WantedBy=timers.target
"#,
            schedule.description,
            schedule.name,
            schedule.cron_expression,
            schedule.randomized_delay_sec
        );

        Ok(timer_content)
    }

    /// Validate a backup schedule configuration
    pub fn validate_schedule(&self, schedule: &BackupSchedule) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Check if source path exists
        if !schedule.source_path.exists() {
            warnings.push(format!("Source path does not exist: {}", schedule.source_path.display()));
        }

        // Check if executable exists
        if !schedule.executable_path.exists() {
            warnings.push(format!("Executable not found: {}", schedule.executable_path.display()));
        }

        // Validate cron expression (basic validation)
        if !self.is_valid_systemd_calendar(&schedule.cron_expression) {
            warnings.push(format!("Invalid systemd calendar expression: {}", schedule.cron_expression));
        }

        // Check memory limit
        if schedule.memory_limit.ends_with('G') {
            if let Ok(gb) = schedule.memory_limit.trim_end_matches('G').parse::<u32>() {
                if gb > 16 {
                    warnings.push("Memory limit exceeds 16GB, consider reducing for system stability".to_string());
                }
            }
        }

        // Check CPU quota
        if schedule.cpu_quota > 100 {
            warnings.push("CPU quota exceeds 100%, this will be clamped by systemd".to_string());
        }

        Ok(warnings)
    }

    /// Basic validation for systemd calendar expressions
    fn is_valid_systemd_calendar(&self, expression: &str) -> bool {
        // This is a simplified validation - a real implementation would be more thorough
        !expression.is_empty() && (
            expression.contains("daily") ||
            expression.contains("weekly") ||
            expression.contains("monthly") ||
            expression.contains("hourly") ||
            expression.contains(':') // Time specification
        )
    }

    /// List existing scheduled backups
    pub async fn list_schedules(&self) -> Result<Vec<String>> {
        let systemd_dir = self.output_dir.join("systemd");
        
        if !systemd_dir.exists() {
            return Ok(Vec::new());
        }

        let mut schedules = Vec::new();
        let mut entries = fs::read_dir(&systemd_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("timer") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    schedules.push(name.to_string());
                }
            }
        }

        Ok(schedules)
    }
}

/// Backup schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    /// Schedule name (used for service/timer names)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Source directory to backup
    pub source_path: PathBuf,
    /// Output directory for backups
    pub output_path: PathBuf,
    /// Backup label
    pub label: String,
    /// Systemd calendar expression (e.g., "daily", "weekly", "*-*-* 02:00:00")
    pub cron_expression: String,
    /// Path to nova-pc-suite executable
    pub executable_path: PathBuf,
    /// User to run backup as
    pub user: String,
    /// Group to run backup as
    pub group: String,
    /// Home directory for the user
    pub home_directory: PathBuf,
    /// Working directory for the backup
    pub working_directory: PathBuf,
    /// Memory limit (e.g., "2G", "512M")
    pub memory_limit: String,
    /// CPU quota percentage (0-100+)
    pub cpu_quota: u32,
    /// Maximum randomized delay in seconds
    pub randomized_delay_sec: u32,
    /// Whether to generate HTML reports
    pub generate_report: bool,
    /// Created timestamp
    pub created: DateTime<Utc>,
}

impl BackupSchedule {
    /// Create a new backup schedule with sensible defaults
    pub fn new(name: &str, source_path: &Path, output_path: &Path) -> Self {
        Self {
            name: name.to_string(),
            description: format!("Automated backup: {}", name),
            source_path: source_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
            label: format!("scheduled-{}", name),
            cron_expression: "daily".to_string(),
            executable_path: PathBuf::from("/usr/local/bin/nova-pc-suite"),
            user: "backup".to_string(),
            group: "backup".to_string(),
            home_directory: PathBuf::from("/var/lib/nova-pc-suite"),
            working_directory: PathBuf::from("/var/lib/nova-pc-suite"),
            memory_limit: "2G".to_string(),
            cpu_quota: 50,
            randomized_delay_sec: 300, // 5 minutes
            generate_report: true,
            created: Utc::now(),
        }
    }

    /// Set custom cron expression
    pub fn with_schedule(mut self, cron_expression: &str) -> Self {
        self.cron_expression = cron_expression.to_string();
        self
    }

    /// Set custom user and group
    pub fn with_user(mut self, user: &str, group: &str) -> Self {
        self.user = user.to_string();
        self.group = group.to_string();
        self
    }

    /// Set resource limits
    pub fn with_limits(mut self, memory_limit: &str, cpu_quota: u32) -> Self {
        self.memory_limit = memory_limit.to_string();
        self.cpu_quota = cpu_quota;
        self
    }
}

/// Output from schedule generation
#[derive(Debug)]
pub struct ScheduleOutput {
    pub service_path: PathBuf,
    pub timer_path: PathBuf,
    pub install_commands: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_schedule_creation() {
        let temp_dir = TempDir::new().unwrap();
        let scheduler = BackupScheduler::new(temp_dir.path());

        let source_path = temp_dir.path().join("source");
        let output_path = temp_dir.path().join("output");

        let schedule = BackupSchedule::new("test-backup", &source_path, &output_path)
            .with_schedule("daily")
            .with_user("testuser", "testgroup")
            .with_limits("1G", 25);

        assert_eq!(schedule.name, "test-backup");
        assert_eq!(schedule.cron_expression, "daily");
        assert_eq!(schedule.user, "testuser");
        assert_eq!(schedule.memory_limit, "1G");
        assert_eq!(schedule.cpu_quota, 25);
    }

    #[tokio::test]
    async fn test_systemd_unit_generation() {
        let temp_dir = TempDir::new().unwrap();
        let scheduler = BackupScheduler::new(temp_dir.path());

        let source_path = temp_dir.path().join("source");
        let output_path = temp_dir.path().join("output");

        let schedule = BackupSchedule::new("test-backup", &source_path, &output_path);

        let service_content = scheduler.generate_service_unit(&schedule).unwrap();
        let timer_content = scheduler.generate_timer_unit(&schedule).unwrap();

        assert!(service_content.contains("Description=NovaPcSuite Backup - Automated backup: test-backup"));
        assert!(service_content.contains("User=backup"));
        assert!(service_content.contains("MemoryMax=2G"));
        assert!(service_content.contains("CPUQuota=50%"));

        assert!(timer_content.contains("OnCalendar=daily"));
        assert!(timer_content.contains("Persistent=true"));
        assert!(timer_content.contains("RandomizedDelaySec=300"));
    }

    #[test]
    fn test_calendar_validation() {
        let temp_dir = TempDir::new().unwrap();
        let _scheduler = BackupScheduler::new(temp_dir.path());

        assert!(_scheduler.is_valid_systemd_calendar("daily"));
        assert!(_scheduler.is_valid_systemd_calendar("weekly"));
        assert!(_scheduler.is_valid_systemd_calendar("*-*-* 02:00:00"));
        assert!(_scheduler.is_valid_systemd_calendar("Mon *-*-* 10:00:00"));
        
        assert!(!_scheduler.is_valid_systemd_calendar(""));
        assert!(!_scheduler.is_valid_systemd_calendar("invalid"));
    }

    #[tokio::test]
    async fn test_schedule_validation() {
        let temp_dir = TempDir::new().unwrap();
        let _scheduler = BackupScheduler::new(temp_dir.path());

        // Create source directory
        let source_path = temp_dir.path().join("source");
        tokio::fs::create_dir_all(&source_path).await.unwrap();

        let output_path = temp_dir.path().join("output");

        let schedule = BackupSchedule::new("test-backup", &source_path, &output_path);
        let warnings = _scheduler.validate_schedule(&schedule).unwrap();

        // Should have warning about missing executable, but not about source path
        assert!(warnings.iter().any(|w| w.contains("Executable not found")));
        assert!(!warnings.iter().any(|w| w.contains("Source path does not exist")));
    }
}