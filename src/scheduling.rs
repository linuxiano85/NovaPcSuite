//! Scheduling functionality for automated backups

use crate::{Error, Result};
use chrono::{DateTime, Utc, TimeZone, Local, NaiveTime, Datelike};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn, debug};

/// Schedule configuration for automated backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// Unique identifier for this schedule
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Whether this schedule is enabled
    pub enabled: bool,
    /// Schedule pattern (cron-like or simple)
    pub pattern: SchedulePattern,
    /// Backup command to execute
    pub command: BackupCommand,
    /// When this schedule was created
    pub created_at: DateTime<Utc>,
    /// Last time this schedule was executed
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled execution time
    pub next_run: Option<DateTime<Utc>>,
}

/// Pattern for scheduling backups
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SchedulePattern {
    /// Simple daily schedule at a specific time
    Daily { time: NaiveTime },
    /// Weekly schedule on specific days
    Weekly { 
        days: Vec<Weekday>,
        time: NaiveTime,
    },
    /// Full cron expression
    Cron { expression: String },
    /// One-time execution at a specific datetime
    Once { datetime: DateTime<Utc> },
}

/// Days of the week
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weekday {
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
    Sunday = 0,
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Weekday::Monday => write!(f, "Monday"),
            Weekday::Tuesday => write!(f, "Tuesday"),
            Weekday::Wednesday => write!(f, "Wednesday"),
            Weekday::Thursday => write!(f, "Thursday"),
            Weekday::Friday => write!(f, "Friday"),
            Weekday::Saturday => write!(f, "Saturday"),
            Weekday::Sunday => write!(f, "Sunday"),
        }
    }
}

/// Backup command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCommand {
    /// Source directory to backup
    pub source_path: PathBuf,
    /// Backup root directory
    pub backup_root: PathBuf,
    /// Snapshot name (can include date placeholders)
    pub snapshot_name: String,
    /// Additional CLI arguments
    pub extra_args: Vec<String>,
}

/// systemd service configuration
#[derive(Debug, Clone)]
pub struct SystemdConfig {
    /// Service name
    pub service_name: String,
    /// Timer name
    pub timer_name: String,
    /// User mode (--user) or system mode
    pub user_mode: bool,
    /// Working directory
    pub working_directory: Option<PathBuf>,
    /// Environment variables
    pub environment: Vec<(String, String)>,
}

impl Default for SystemdConfig {
    fn default() -> Self {
        Self {
            service_name: "nova-backup".to_string(),
            timer_name: "nova-backup".to_string(),
            user_mode: true,
            working_directory: None,
            environment: Vec::new(),
        }
    }
}

/// Scheduler for managing backup schedules
pub struct Scheduler {
    schedules_path: PathBuf,
    nova_cli_path: PathBuf,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new<P: AsRef<Path>>(config_root: P, nova_cli_path: P) -> Result<Self> {
        let schedules_path = config_root.as_ref().join("schedules");
        fs::create_dir_all(&schedules_path)?;
        
        Ok(Self {
            schedules_path,
            nova_cli_path: nova_cli_path.as_ref().to_path_buf(),
        })
    }

    /// Add a new schedule
    pub fn add_schedule(&self, mut schedule: Schedule) -> Result<()> {
        // Calculate next run time
        schedule.next_run = self.calculate_next_run(&schedule.pattern)?;
        
        let schedule_file = self.schedules_path.join(format!("{}.json", schedule.id));
        let content = serde_json::to_string_pretty(&schedule)?;
        fs::write(schedule_file, content)?;
        
        info!("Added schedule '{}' ({})", schedule.name, schedule.id);
        Ok(())
    }

    /// Remove a schedule
    pub fn remove_schedule(&self, schedule_id: &str) -> Result<()> {
        let schedule_file = self.schedules_path.join(format!("{}.json", schedule_id));
        if schedule_file.exists() {
            fs::remove_file(schedule_file)?;
            info!("Removed schedule '{}'", schedule_id);
        }
        Ok(())
    }

    /// List all schedules
    pub fn list_schedules(&self) -> Result<Vec<Schedule>> {
        let mut schedules = Vec::new();
        
        for entry in fs::read_dir(&self.schedules_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "json") {
                match self.load_schedule_from_file(&path) {
                    Ok(schedule) => schedules.push(schedule),
                    Err(e) => warn!("Failed to load schedule from {}: {}", path.display(), e),
                }
            }
        }
        
        // Sort by next run time
        schedules.sort_by(|a, b| a.next_run.cmp(&b.next_run));
        
        Ok(schedules)
    }

    /// Get a specific schedule by ID
    pub fn get_schedule(&self, schedule_id: &str) -> Result<Option<Schedule>> {
        let schedule_file = self.schedules_path.join(format!("{}.json", schedule_id));
        
        if !schedule_file.exists() {
            return Ok(None);
        }
        
        self.load_schedule_from_file(&schedule_file).map(Some)
    }

    /// Enable or disable a schedule
    pub fn set_schedule_enabled(&self, schedule_id: &str, enabled: bool) -> Result<()> {
        if let Some(mut schedule) = self.get_schedule(schedule_id)? {
            schedule.enabled = enabled;
            
            if enabled {
                schedule.next_run = self.calculate_next_run(&schedule.pattern)?;
            } else {
                schedule.next_run = None;
            }
            
            self.add_schedule(schedule)?;
            info!("Schedule '{}' {}", schedule_id, if enabled { "enabled" } else { "disabled" });
        }
        
        Ok(())
    }

    /// Generate systemd service and timer files for a schedule
    pub fn generate_systemd_units(
        &self,
        schedule: &Schedule,
        config: &SystemdConfig,
    ) -> Result<(String, String)> {
        let service_content = self.generate_systemd_service(schedule, config)?;
        let timer_content = self.generate_systemd_timer(schedule, config)?;
        
        Ok((service_content, timer_content))
    }

    /// Install systemd units for a schedule
    pub fn install_systemd_schedule(
        &self,
        schedule: &Schedule,
        config: &SystemdConfig,
    ) -> Result<()> {
        let (service_content, timer_content) = self.generate_systemd_units(schedule, config)?;
        
        // Determine systemd directory
        let systemd_dir = if config.user_mode {
            dirs::home_dir()
                .ok_or_else(|| Error::Configuration {
                    reason: "Could not determine home directory".to_string(),
                })?
                .join(".config/systemd/user")
        } else {
            PathBuf::from("/etc/systemd/system")
        };
        
        fs::create_dir_all(&systemd_dir)?;
        
        // Write service file
        let service_file = systemd_dir.join(format!("{}.service", config.service_name));
        fs::write(&service_file, service_content)?;
        
        // Write timer file
        let timer_file = systemd_dir.join(format!("{}.timer", config.timer_name));
        fs::write(&timer_file, timer_content)?;
        
        // Reload systemd and enable timer
        self.systemctl_reload(config.user_mode)?;
        self.systemctl_enable(&config.timer_name, config.user_mode)?;
        
        info!("Installed systemd schedule for '{}'", schedule.name);
        Ok(())
    }

    /// Uninstall systemd units for a schedule
    pub fn uninstall_systemd_schedule(&self, config: &SystemdConfig) -> Result<()> {
        // Stop and disable timer
        let _ = self.systemctl_stop(&config.timer_name, config.user_mode);
        let _ = self.systemctl_disable(&config.timer_name, config.user_mode);
        
        // Determine systemd directory
        let systemd_dir = if config.user_mode {
            dirs::home_dir()
                .ok_or_else(|| Error::Configuration {
                    reason: "Could not determine home directory".to_string(),
                })?
                .join(".config/systemd/user")
        } else {
            PathBuf::from("/etc/systemd/system")
        };
        
        // Remove files
        let service_file = systemd_dir.join(format!("{}.service", config.service_name));
        let timer_file = systemd_dir.join(format!("{}.timer", config.timer_name));
        
        if service_file.exists() {
            fs::remove_file(service_file)?;
        }
        
        if timer_file.exists() {
            fs::remove_file(timer_file)?;
        }
        
        self.systemctl_reload(config.user_mode)?;
        
        info!("Uninstalled systemd schedule");
        Ok(())
    }

    /// Parse a schedule pattern from a string
    pub fn parse_schedule_pattern(pattern: &str) -> Result<SchedulePattern> {
        if pattern.starts_with("daily@") {
            let time_str = &pattern[6..];
            let time = NaiveTime::parse_from_str(time_str, "%H:%M")
                .map_err(|_| Error::Scheduling {
                    reason: format!("Invalid daily time format: {}", time_str),
                })?;
            
            Ok(SchedulePattern::Daily { time })
        } else if pattern.starts_with("weekly@") {
            // Format: weekly@Mon,Wed,Fri@14:30
            let parts: Vec<&str> = pattern[7..].split('@').collect();
            if parts.len() != 2 {
                return Err(Error::Scheduling {
                    reason: "Weekly format should be 'weekly@days@time'".to_string(),
                });
            }
            
            let days: Result<Vec<Weekday>> = parts[0]
                .split(',')
                .map(|day| Self::parse_weekday(day.trim()))
                .collect();
            
            let time = NaiveTime::parse_from_str(parts[1], "%H:%M")
                .map_err(|_| Error::Scheduling {
                    reason: format!("Invalid weekly time format: {}", parts[1]),
                })?;
            
            Ok(SchedulePattern::Weekly { days: days?, time })
        } else if pattern.starts_with("cron:") {
            Ok(SchedulePattern::Cron {
                expression: pattern[5..].to_string(),
            })
        } else {
            Err(Error::Scheduling {
                reason: format!("Unknown schedule pattern: {}", pattern),
            })
        }
    }

    /// Calculate the next run time for a schedule pattern
    fn calculate_next_run(&self, pattern: &SchedulePattern) -> Result<Option<DateTime<Utc>>> {
        let now = Utc::now();
        
        match pattern {
            SchedulePattern::Daily { time } => {
                let today = now.date_naive();
                let mut next_datetime = today.and_time(*time);
                
                // If the time has already passed today, schedule for tomorrow
                if Local.from_local_datetime(&next_datetime).unwrap().with_timezone(&Utc) <= now {
                    next_datetime = (today + chrono::Duration::days(1)).and_time(*time);
                }
                
                Ok(Some(Local.from_local_datetime(&next_datetime).unwrap().with_timezone(&Utc)))
            }
            SchedulePattern::Weekly { days, time } => {
                // Find the next occurrence of any of the specified days
                let today = now.date_naive();
                let current_weekday = today.weekday().num_days_from_sunday() as i32;
                
                let mut min_days_ahead = 8; // More than a week
                
                for weekday in days {
                    let target_day = *weekday as i32;
                    let days_ahead = if target_day >= current_weekday {
                        target_day - current_weekday
                    } else {
                        7 + target_day - current_weekday
                    };
                    
                    // Check if we can schedule today
                    if days_ahead == 0 {
                        let today_at_time = today.and_time(*time);
                        if Local.from_local_datetime(&today_at_time).unwrap().with_timezone(&Utc) > now {
                            return Ok(Some(Local.from_local_datetime(&today_at_time).unwrap().with_timezone(&Utc)));
                        }
                    }
                    
                    min_days_ahead = min_days_ahead.min(if days_ahead == 0 { 7 } else { days_ahead });
                }
                
                let next_date = today + chrono::Duration::days(min_days_ahead as i64);
                let next_datetime = next_date.and_time(*time);
                
                Ok(Some(Local.from_local_datetime(&next_datetime).unwrap().with_timezone(&Utc)))
            }
            SchedulePattern::Once { datetime } => {
                if *datetime > now {
                    Ok(Some(*datetime))
                } else {
                    Ok(None) // One-time schedule in the past
                }
            }
            SchedulePattern::Cron { expression: _ } => {
                // TODO: Implement proper cron parsing
                warn!("Cron expressions not yet fully implemented");
                Ok(None)
            }
        }
    }

    /// Parse a weekday from string
    fn parse_weekday(day: &str) -> Result<Weekday> {
        match day.to_lowercase().as_str() {
            "mon" | "monday" => Ok(Weekday::Monday),
            "tue" | "tuesday" => Ok(Weekday::Tuesday),
            "wed" | "wednesday" => Ok(Weekday::Wednesday),
            "thu" | "thursday" => Ok(Weekday::Thursday),
            "fri" | "friday" => Ok(Weekday::Friday),
            "sat" | "saturday" => Ok(Weekday::Saturday),
            "sun" | "sunday" => Ok(Weekday::Sunday),
            _ => Err(Error::Scheduling {
                reason: format!("Invalid weekday: {}", day),
            }),
        }
    }

    /// Load a schedule from a file
    fn load_schedule_from_file(&self, path: &Path) -> Result<Schedule> {
        let content = fs::read_to_string(path)?;
        let schedule: Schedule = serde_json::from_str(&content)?;
        Ok(schedule)
    }

    /// Generate systemd service file content
    fn generate_systemd_service(
        &self,
        schedule: &Schedule,
        config: &SystemdConfig,
    ) -> Result<String> {
        let mut service = String::new();
        
        service.push_str("[Unit]\n");
        service.push_str(&format!("Description=Nova PC Suite Backup - {}\n", schedule.name));
        service.push_str("Wants=network-online.target\n");
        service.push_str("After=network-online.target\n\n");
        
        service.push_str("[Service]\n");
        service.push_str("Type=oneshot\n");
        
        if let Some(working_dir) = &config.working_directory {
            service.push_str(&format!("WorkingDirectory={}\n", working_dir.display()));
        }
        
        // Add environment variables
        for (key, value) in &config.environment {
            service.push_str(&format!("Environment={}={}\n", key, value));
        }
        
        // Build the command
        let mut cmd_args = vec![
            "backup".to_string(),
            "run".to_string(),
            "--source".to_string(),
            schedule.command.source_path.display().to_string(),
            "--root".to_string(),
            schedule.command.backup_root.display().to_string(),
            "--name".to_string(),
            schedule.command.snapshot_name.clone(),
        ];
        
        cmd_args.extend(schedule.command.extra_args.clone());
        
        service.push_str(&format!(
            "ExecStart={} {}\n",
            self.nova_cli_path.display(),
            cmd_args.join(" ")
        ));
        
        service.push_str("StandardOutput=journal\n");
        service.push_str("StandardError=journal\n");
        
        Ok(service)
    }

    /// Generate systemd timer file content
    fn generate_systemd_timer(
        &self,
        schedule: &Schedule,
        config: &SystemdConfig,
    ) -> Result<String> {
        let mut timer = String::new();
        
        timer.push_str("[Unit]\n");
        timer.push_str(&format!("Description=Timer for Nova PC Suite Backup - {}\n", schedule.name));
        timer.push_str(&format!("Requires={}.service\n\n", config.service_name));
        
        timer.push_str("[Timer]\n");
        
        match &schedule.pattern {
            SchedulePattern::Daily { time } => {
                timer.push_str(&format!("OnCalendar=daily\n"));
                timer.push_str(&format!("AccuracySec=1min\n"));
            }
            SchedulePattern::Weekly { days, time } => {
                let day_names: Vec<String> = days.iter().map(|d| format!("{}", d)).collect();
                timer.push_str(&format!(
                    "OnCalendar={} *-*-* {}:00\n",
                    day_names.join(","),
                    time.format("%H:%M")
                ));
            }
            SchedulePattern::Cron { expression } => {
                // Convert cron to systemd calendar format (simplified)
                timer.push_str(&format!("# Cron: {}\n", expression));
                timer.push_str("OnCalendar=daily\n"); // Fallback
            }
            SchedulePattern::Once { datetime } => {
                timer.push_str(&format!(
                    "OnCalendar={}\n",
                    datetime.format("%Y-%m-%d %H:%M:%S")
                ));
            }
        }
        
        timer.push_str("Persistent=true\n\n");
        
        timer.push_str("[Install]\n");
        timer.push_str("WantedBy=timers.target\n");
        
        Ok(timer)
    }

    /// Execute systemctl command
    fn systemctl_cmd(&self, args: &[&str], user_mode: bool) -> Result<()> {
        let mut cmd = Command::new("systemctl");
        
        if user_mode {
            cmd.arg("--user");
        }
        
        cmd.args(args);
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            return Err(Error::Scheduling {
                reason: format!(
                    "systemctl command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }
        
        Ok(())
    }

    /// Reload systemd daemon
    fn systemctl_reload(&self, user_mode: bool) -> Result<()> {
        self.systemctl_cmd(&["daemon-reload"], user_mode)
    }

    /// Enable a systemd unit
    fn systemctl_enable(&self, unit_name: &str, user_mode: bool) -> Result<()> {
        self.systemctl_cmd(&["enable", &format!("{}.timer", unit_name)], user_mode)?;
        self.systemctl_cmd(&["start", &format!("{}.timer", unit_name)], user_mode)
    }

    /// Disable a systemd unit
    fn systemctl_disable(&self, unit_name: &str, user_mode: bool) -> Result<()> {
        self.systemctl_cmd(&["disable", &format!("{}.timer", unit_name)], user_mode)
    }

    /// Stop a systemd unit
    fn systemctl_stop(&self, unit_name: &str, user_mode: bool) -> Result<()> {
        self.systemctl_cmd(&["stop", &format!("{}.timer", unit_name)], user_mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use chrono::Timelike;

    #[test]
    fn test_parse_daily_schedule() -> Result<()> {
        let pattern = Scheduler::parse_schedule_pattern("daily@14:30")?;
        
        match pattern {
            SchedulePattern::Daily { time } => {
                assert_eq!(time.hour(), 14);
                assert_eq!(time.minute(), 30);
            }
            _ => panic!("Expected daily pattern"),
        }
        
        Ok(())
    }

    #[test]
    fn test_parse_weekly_schedule() -> Result<()> {
        let pattern = Scheduler::parse_schedule_pattern("weekly@Mon,Wed,Fri@09:00")?;
        
        match pattern {
            SchedulePattern::Weekly { days, time } => {
                assert_eq!(days.len(), 3);
                assert!(days.contains(&Weekday::Monday));
                assert!(days.contains(&Weekday::Wednesday));
                assert!(days.contains(&Weekday::Friday));
                assert_eq!(time.hour(), 9);
                assert_eq!(time.minute(), 0);
            }
            _ => panic!("Expected weekly pattern"),
        }
        
        Ok(())
    }

    #[test]
    fn test_parse_cron_schedule() -> Result<()> {
        let pattern = Scheduler::parse_schedule_pattern("cron:0 2 * * *")?;
        
        match pattern {
            SchedulePattern::Cron { expression } => {
                assert_eq!(expression, "0 2 * * *");
            }
            _ => panic!("Expected cron pattern"),
        }
        
        Ok(())
    }

    #[test]
    fn test_weekday_parsing() -> Result<()> {
        assert_eq!(Scheduler::parse_weekday("mon")?, Weekday::Monday);
        assert_eq!(Scheduler::parse_weekday("Monday")?, Weekday::Monday);
        assert_eq!(Scheduler::parse_weekday("fri")?, Weekday::Friday);
        assert_eq!(Scheduler::parse_weekday("Sunday")?, Weekday::Sunday);
        
        Ok(())
    }

    #[test]
    fn test_scheduler_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let nova_cli_path = temp_dir.path().join("nova-cli");
        
        let _scheduler = Scheduler::new(temp_dir.path(), &nova_cli_path)?;
        
        Ok(())
    }

    #[test]
    fn test_systemd_config_default() {
        let config = SystemdConfig::default();
        assert_eq!(config.service_name, "nova-backup");
        assert_eq!(config.timer_name, "nova-backup");
        assert!(config.user_mode);
    }
}