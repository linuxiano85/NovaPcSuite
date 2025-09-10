//! Nova PC Suite CLI application

use clap::{Parser, Subcommand, Args};
use nova_pc_suite::{
    backup::{BackupEngine, BackupConfig, ConsoleProgress},
    restore::{RestoreEngine, RestoreConfig, ConflictPolicy, load_path_mappings},
    scheduling::{Scheduler, Schedule, SchedulePattern, BackupCommand, SystemdConfig},
    Error, Result,
};

#[cfg(feature = "recovery")]
use nova_pc_suite::recovery::RecoveryEngine;

use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, error, Level};
use tracing_subscriber::{fmt, EnvFilter};
use uuid::Uuid;

/// Nova PC Suite - Comprehensive backup and restore system
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable quiet mode (reduce output)
    #[arg(long, global = true)]
    quiet: bool,

    /// Log format: text or json
    #[arg(long, default_value = "text", global = true)]
    log_format: LogFormat,

    /// Backup root directory
    #[arg(long, short = 'r', global = true, env = "NOVA_BACKUP_ROOT")]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum LogFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
enum Commands {
    /// Backup operations
    Backup(BackupCommands),
    /// Restore operations
    Restore(RestoreCommands),
    /// Schedule management
    Schedule(ScheduleCommands),
    /// Data recovery operations
    #[cfg(feature = "recovery")]
    Recover(RecoveryCommands),
}

#[derive(Args)]
struct BackupCommands {
    #[command(subcommand)]
    action: BackupActions,
}

#[derive(Subcommand)]
enum BackupActions {
    /// Create a backup snapshot
    Run {
        /// Source directory to backup
        #[arg(long, short)]
        source: PathBuf,

        /// Snapshot name
        #[arg(long, short)]
        name: String,

        /// Chunk size in bytes
        #[arg(long, default_value = "1048576")]
        chunk_size: usize,

        /// Follow symbolic links
        #[arg(long)]
        follow_symlinks: bool,

        /// Exclude patterns (glob-style)
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,

        /// Maximum file size to backup (bytes)
        #[arg(long)]
        max_file_size: Option<u64>,
    },
    /// List available snapshots
    List {
        /// Output format
        #[arg(long, default_value = "table")]
        format: OutputFormat,
    },
    /// Show snapshot details
    Show {
        /// Snapshot ID
        snapshot_id: String,

        /// Output format
        #[arg(long, default_value = "table")]
        format: OutputFormat,
    },
}

#[derive(Args)]
struct RestoreCommands {
    #[command(subcommand)]
    action: RestoreActions,
}

#[derive(Subcommand)]
enum RestoreActions {
    /// Restore a snapshot
    Run {
        /// Snapshot ID to restore
        snapshot_id: String,

        /// Target directory for restore
        #[arg(long, short)]
        target: PathBuf,

        /// Dry run mode (plan only, no actual restore)
        #[arg(long)]
        dry_run: bool,

        /// Conflict resolution policy
        #[arg(long, default_value = "skip")]
        on_conflict: ConflictPolicyArg,

        /// Path mapping file (TOML format)
        #[arg(long)]
        map: Option<PathBuf>,

        /// Skip integrity verification
        #[arg(long)]
        skip_verify: bool,

        /// Don't preserve file permissions
        #[arg(long)]
        no_permissions: bool,

        /// Output format for dry run
        #[arg(long, default_value = "table")]
        format: OutputFormat,
    },
    /// Create a restore plan without executing it
    Plan {
        /// Snapshot ID to plan restore for
        snapshot_id: String,

        /// Target directory for restore
        #[arg(long, short)]
        target: PathBuf,

        /// Conflict resolution policy
        #[arg(long, default_value = "skip")]
        on_conflict: ConflictPolicyArg,

        /// Path mapping file (TOML format)
        #[arg(long)]
        map: Option<PathBuf>,

        /// Output format
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
}

#[derive(Args)]
struct ScheduleCommands {
    #[command(subcommand)]
    action: ScheduleActions,
}

#[derive(Subcommand)]
enum ScheduleActions {
    /// Add a new backup schedule
    Add {
        /// Schedule name
        #[arg(long, short)]
        name: String,

        /// Schedule pattern (e.g., daily@14:30, weekly@Mon,Wed,Fri@09:00)
        #[arg(long, short)]
        pattern: String,

        /// Source directory to backup
        #[arg(long, short)]
        source: PathBuf,

        /// Backup root directory
        #[arg(long)]
        backup_root: Option<PathBuf>,

        /// Snapshot name template
        #[arg(long, default_value = "auto-{date}")]
        snapshot_name: String,

        /// Install systemd units
        #[arg(long)]
        install: bool,

        /// Additional CLI arguments
        #[arg(long, action = clap::ArgAction::Append)]
        extra_args: Vec<String>,
    },
    /// List all schedules
    List {
        /// Output format
        #[arg(long, default_value = "table")]
        format: OutputFormat,
    },
    /// Show schedule details
    Show {
        /// Schedule ID
        schedule_id: String,

        /// Output format
        #[arg(long, default_value = "table")]
        format: OutputFormat,
    },
    /// Enable or disable a schedule
    Toggle {
        /// Schedule ID
        schedule_id: String,

        /// Enable the schedule
        #[arg(long)]
        enable: bool,

        /// Disable the schedule
        #[arg(long)]
        disable: bool,
    },
    /// Remove a schedule
    Remove {
        /// Schedule ID
        schedule_id: String,

        /// Force removal without confirmation
        #[arg(long)]
        force: bool,
    },
    /// Install systemd units for a schedule
    Install {
        /// Schedule ID
        schedule_id: String,

        /// Use system-wide installation instead of user
        #[arg(long)]
        system: bool,
    },
    /// Uninstall systemd units for a schedule
    Uninstall {
        /// Schedule ID
        schedule_id: String,

        /// Use system-wide uninstallation instead of user
        #[arg(long)]
        system: bool,
    },
}

#[cfg(feature = "recovery")]
#[derive(Args)]
struct RecoveryCommands {
    #[command(subcommand)]
    action: RecoveryActions,
}

#[cfg(feature = "recovery")]
#[derive(Subcommand)]
enum RecoveryActions {
    /// Detect orphaned chunks
    OrphanChunks {
        /// Output format
        #[arg(long, default_value = "json")]
        format: OutputFormat,

        /// Clean up orphaned chunks
        #[arg(long)]
        cleanup: bool,

        /// Force cleanup without confirmation
        #[arg(long)]
        force: bool,
    },
    /// Salvage corrupted snapshots
    Salvage {
        /// Output format
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    /// Validate snapshot integrity
    Validate {
        /// Snapshot ID to validate
        snapshot_id: String,

        /// Output format
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum OutputFormat {
    Table,
    Json,
    Yaml,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ConflictPolicyArg {
    Skip,
    Overwrite,
    Rename,
}

impl From<ConflictPolicyArg> for ConflictPolicy {
    fn from(arg: ConflictPolicyArg) -> Self {
        match arg {
            ConflictPolicyArg::Skip => ConflictPolicy::Skip,
            ConflictPolicyArg::Overwrite => ConflictPolicy::Overwrite,
            ConflictPolicyArg::Rename => ConflictPolicy::Rename,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(&cli)?;

    // Get backup root directory
    let root = get_backup_root(&cli)?;

    // Execute command
    match cli.command {
        Commands::Backup(backup_cmd) => handle_backup_commands(backup_cmd, &root),
        Commands::Restore(restore_cmd) => handle_restore_commands(restore_cmd, &root),
        Commands::Schedule(schedule_cmd) => handle_schedule_commands(schedule_cmd, &root),
        #[cfg(feature = "recovery")]
        Commands::Recover(recovery_cmd) => handle_recovery_commands(recovery_cmd, &root),
    }
}

fn init_logging(cli: &Cli) -> Result<()> {
    let level = if cli.quiet { Level::WARN } else { Level::INFO };
    
    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    match cli.log_format {
        LogFormat::Text => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .init();
        }
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .json()
                .init();
        }
    }

    Ok(())
}

fn get_backup_root(cli: &Cli) -> Result<PathBuf> {
    cli.root.clone()
        .or_else(|| std::env::var("NOVA_BACKUP_ROOT").ok().map(PathBuf::from))
        .or_else(|| dirs::home_dir().map(|home| home.join(".nova-backup")))
        .ok_or_else(|| Error::Configuration {
            reason: "No backup root directory specified. Use --root, NOVA_BACKUP_ROOT env var, or default ~/.nova-backup".to_string(),
        })
}

fn handle_backup_commands(cmd: BackupCommands, root: &PathBuf) -> Result<()> {
    match cmd.action {
        BackupActions::Run {
            source,
            name,
            chunk_size,
            follow_symlinks,
            exclude,
            max_file_size,
        } => {
            let mut config = BackupConfig::default();
            config.chunk_size = chunk_size;
            config.follow_symlinks = follow_symlinks;
            if !exclude.is_empty() {
                config.exclude_patterns = exclude;
            }
            config.max_file_size = max_file_size;

            let engine = BackupEngine::new(root, config)?;
            let snapshot = engine.create_snapshot(&source, name)?;

            info!("Backup completed successfully");
            println!("Snapshot ID: {}", snapshot.id);
            println!("Files: {}", snapshot.files.len());
            println!("Chunks: {}", snapshot.chunk_stats.total_chunks);
            println!("Total size: {} bytes", snapshot.chunk_stats.total_bytes);
        }
        BackupActions::List { format } => {
            let config = BackupConfig::default();
            let engine = BackupEngine::new(root, config)?;
            let snapshots = engine.list_snapshots()?;

            match format {
                OutputFormat::Json => {
                    let snapshot_details: Vec<_> = snapshots
                        .iter()
                        .filter_map(|id| engine.get_snapshot(id).ok())
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&snapshot_details)?);
                }
                OutputFormat::Table | OutputFormat::Yaml => {
                    println!("{:<36} {:<20} {:<20} {:<10}", "ID", "Name", "Created", "Files");
                    println!("{:-<86}", "");
                    
                    for id in snapshots {
                        if let Ok(snapshot) = engine.get_snapshot(&id) {
                            println!(
                                "{:<36} {:<20} {:<20} {:<10}",
                                id,
                                snapshot.name,
                                snapshot.created.format("%Y-%m-%d %H:%M:%S"),
                                snapshot.files.len()
                            );
                        }
                    }
                }
            }
        }
        BackupActions::Show { snapshot_id, format } => {
            let id = Uuid::parse_str(&snapshot_id).map_err(|_| Error::Configuration {
                reason: "Invalid snapshot ID format".to_string(),
            })?;

            let config = BackupConfig::default();
            let engine = BackupEngine::new(root, config)?;
            let snapshot = engine.get_snapshot(&id)?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&snapshot)?);
                }
                OutputFormat::Table | OutputFormat::Yaml => {
                    println!("Snapshot Details:");
                    println!("  ID: {}", snapshot.id);
                    println!("  Name: {}", snapshot.name);
                    println!("  Created: {}", snapshot.created);
                    println!("  Source: {}", snapshot.source_root.display());
                    println!("  Files: {}", snapshot.files.len());
                    println!("  Chunks: {}", snapshot.chunk_stats.total_chunks);
                    println!("  Total Size: {} bytes", snapshot.chunk_stats.total_bytes);
                    
                    if !snapshot.files.is_empty() {
                        println!("\nFiles:");
                        for file in &snapshot.files[..10.min(snapshot.files.len())] {
                            println!("  {} ({} bytes)", file.path.display(), file.size);
                        }
                        
                        if snapshot.files.len() > 10 {
                            println!("  ... and {} more files", snapshot.files.len() - 10);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_restore_commands(cmd: RestoreCommands, root: &PathBuf) -> Result<()> {
    let engine = RestoreEngine::new(root)?;

    match cmd.action {
        RestoreActions::Run {
            snapshot_id,
            target,
            dry_run,
            on_conflict,
            map,
            skip_verify,
            no_permissions,
            format,
        } => {
            let id = Uuid::parse_str(&snapshot_id).map_err(|_| Error::Configuration {
                reason: "Invalid snapshot ID format".to_string(),
            })?;

            let mut config = RestoreConfig::default();
            config.dry_run = dry_run;
            config.conflict_policy = on_conflict.into();
            config.verify_integrity = !skip_verify;
            config.preserve_permissions = !no_permissions;

            // Load path mappings if provided
            if let Some(map_file) = map {
                config.path_mappings = load_path_mappings(map_file)?;
            }

            if dry_run {
                let plan = engine.create_plan(&id, &target, &config)?;
                
                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&plan)?);
                    }
                    OutputFormat::Table | OutputFormat::Yaml => {
                        println!("Restore Plan:");
                        println!("  Snapshot: {}", plan.snapshot_id);
                        println!("  Target: {}", plan.target_root.display());
                        println!("  Total files: {}", plan.summary.total_files);
                        println!("  Files to restore: {}", plan.summary.files_to_restore);
                        println!("  Files to skip: {}", plan.summary.files_skipped);
                        println!("  Files with conflicts: {}", plan.summary.files_with_conflicts);
                        println!("  Missing chunks: {}", plan.summary.files_with_missing_chunks);
                        println!("  Total bytes: {}", plan.summary.total_bytes);
                    }
                }
            } else {
                let result = engine.restore_snapshot(&id, &target, config)?;
                
                info!("Restore completed");
                println!("Files restored: {}", result.files_restored);
                println!("Files skipped: {}", result.files_skipped);
                println!("Files failed: {}", result.files_failed);
                println!("Bytes written: {}", result.bytes_written);
                println!("Duration: {:?}", result.duration);
                
                if !result.errors.is_empty() {
                    println!("\nErrors:");
                    for (path, error) in &result.errors {
                        println!("  {}: {}", path.display(), error);
                    }
                }
            }
        }
        RestoreActions::Plan {
            snapshot_id,
            target,
            on_conflict,
            map,
            format,
        } => {
            let id = Uuid::parse_str(&snapshot_id).map_err(|_| Error::Configuration {
                reason: "Invalid snapshot ID format".to_string(),
            })?;

            let mut config = RestoreConfig::default();
            config.conflict_policy = on_conflict.into();

            if let Some(map_file) = map {
                config.path_mappings = load_path_mappings(map_file)?;
            }

            let plan = engine.create_plan(&id, &target, &config)?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                }
                OutputFormat::Table | OutputFormat::Yaml => {
                    println!("Restore Plan Summary:");
                    println!("  Total files: {}", plan.summary.total_files);
                    println!("  Files to restore: {}", plan.summary.files_to_restore);
                    println!("  Files to skip: {}", plan.summary.files_skipped);
                    println!("  Total bytes: {}", plan.summary.total_bytes);
                }
            }
        }
    }

    Ok(())
}

fn handle_schedule_commands(cmd: ScheduleCommands, root: &PathBuf) -> Result<()> {
    let nova_cli_path = std::env::current_exe()?;
    let scheduler = Scheduler::new(root.join("config"), nova_cli_path)?;

    match cmd.action {
        ScheduleActions::Add {
            name,
            pattern,
            source,
            backup_root,
            snapshot_name,
            install,
            extra_args,
        } => {
            let pattern = Scheduler::parse_schedule_pattern(&pattern)?;
            let backup_root = backup_root.unwrap_or_else(|| root.clone());

            let schedule = Schedule {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.clone(),
                enabled: true,
                pattern,
                command: BackupCommand {
                    source_path: source,
                    backup_root,
                    snapshot_name,
                    extra_args,
                },
                created_at: chrono::Utc::now(),
                last_run: None,
                next_run: None,
            };

            scheduler.add_schedule(schedule.clone())?;
            
            if install {
                let config = SystemdConfig::default();
                scheduler.install_systemd_schedule(&schedule, &config)?;
                info!("Schedule '{}' added and installed", name);
            } else {
                info!("Schedule '{}' added", name);
            }
            
            println!("Schedule ID: {}", schedule.id);
        }
        ScheduleActions::List { format } => {
            let schedules = scheduler.list_schedules()?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&schedules)?);
                }
                OutputFormat::Table | OutputFormat::Yaml => {
                    println!("{:<36} {:<20} {:<10} {:<20}", "ID", "Name", "Enabled", "Next Run");
                    println!("{:-<86}", "");
                    
                    for schedule in schedules {
                        let next_run = schedule.next_run
                            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "Never".to_string());
                        
                        println!(
                            "{:<36} {:<20} {:<10} {:<20}",
                            schedule.id,
                            schedule.name,
                            schedule.enabled,
                            next_run
                        );
                    }
                }
            }
        }
        ScheduleActions::Show { schedule_id, format } => {
            if let Some(schedule) = scheduler.get_schedule(&schedule_id)? {
                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&schedule)?);
                    }
                    OutputFormat::Table | OutputFormat::Yaml => {
                        println!("Schedule Details:");
                        println!("  ID: {}", schedule.id);
                        println!("  Name: {}", schedule.name);
                        println!("  Enabled: {}", schedule.enabled);
                        println!("  Pattern: {:?}", schedule.pattern);
                        println!("  Source: {}", schedule.command.source_path.display());
                        println!("  Backup Root: {}", schedule.command.backup_root.display());
                        println!("  Snapshot Name: {}", schedule.command.snapshot_name);
                        
                        if let Some(next_run) = schedule.next_run {
                            println!("  Next Run: {}", next_run);
                        }
                        
                        if let Some(last_run) = schedule.last_run {
                            println!("  Last Run: {}", last_run);
                        }
                    }
                }
            } else {
                return Err(Error::Configuration {
                    reason: format!("Schedule '{}' not found", schedule_id),
                });
            }
        }
        ScheduleActions::Toggle { schedule_id, enable, disable } => {
            if enable && disable {
                return Err(Error::Configuration {
                    reason: "Cannot both enable and disable".to_string(),
                });
            }
            
            let enabled = enable || !disable;
            scheduler.set_schedule_enabled(&schedule_id, enabled)?;
            
            println!("Schedule '{}' {}", schedule_id, if enabled { "enabled" } else { "disabled" });
        }
        ScheduleActions::Remove { schedule_id, force } => {
            if !force {
                println!("Are you sure you want to remove schedule '{}'? (y/N)", schedule_id);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("Cancelled");
                    return Ok(());
                }
            }
            
            scheduler.remove_schedule(&schedule_id)?;
            println!("Schedule '{}' removed", schedule_id);
        }
        ScheduleActions::Install { schedule_id, system } => {
            if let Some(schedule) = scheduler.get_schedule(&schedule_id)? {
                let mut config = SystemdConfig::default();
                config.user_mode = !system;
                
                scheduler.install_systemd_schedule(&schedule, &config)?;
                println!("systemd units installed for schedule '{}'", schedule_id);
            } else {
                return Err(Error::Configuration {
                    reason: format!("Schedule '{}' not found", schedule_id),
                });
            }
        }
        ScheduleActions::Uninstall { schedule_id, system } => {
            let mut config = SystemdConfig::default();
            config.user_mode = !system;
            
            scheduler.uninstall_systemd_schedule(&config)?;
            println!("systemd units uninstalled for schedule '{}'", schedule_id);
        }
    }

    Ok(())
}

#[cfg(feature = "recovery")]
fn handle_recovery_commands(cmd: RecoveryCommands, root: &PathBuf) -> Result<()> {
    let engine = RecoveryEngine::new(root)?;

    match cmd.action {
        RecoveryActions::OrphanChunks { format, cleanup, force } => {
            let report = engine.detect_orphan_chunks()?;

            if cleanup {
                if !force {
                    println!("This will permanently delete {} orphaned chunks ({} bytes).", 
                        report.total_orphans, report.total_size);
                    println!("Are you sure? (y/N)");
                    
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    
                    if !input.trim().to_lowercase().starts_with('y') {
                        println!("Cancelled");
                        return Ok(());
                    }
                }
                
                let cleanup_result = engine.cleanup_orphans(&report, true)?;
                
                println!("Cleanup completed:");
                println!("  Chunks removed: {}", cleanup_result.chunks_removed);
                println!("  Bytes freed: {}", cleanup_result.bytes_freed);
                
                if !cleanup_result.errors.is_empty() {
                    println!("Errors:");
                    for error in &cleanup_result.errors {
                        println!("  {}", error);
                    }
                }
            } else {
                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                    OutputFormat::Table | OutputFormat::Yaml => {
                        println!("Orphan Chunks Report:");
                        println!("  Generated: {}", report.generated_at);
                        println!("  Total orphans: {}", report.total_orphans);
                        println!("  Total size: {} bytes", report.total_size);
                        
                        println!("\nSize distribution:");
                        for (category, count) in &report.size_distribution {
                            println!("  {}: {}", category, count);
                        }
                        
                        if !report.orphans.is_empty() {
                            println!("\nLargest orphans:");
                            for orphan in &report.orphans[..5.min(report.orphans.len())] {
                                println!("  {} ({} bytes)", orphan.hash, orphan.size);
                            }
                        }
                    }
                }
            }
        }
        RecoveryActions::Salvage { format } => {
            let result = engine.salvage_snapshots()?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                OutputFormat::Table | OutputFormat::Yaml => {
                    println!("Snapshot Salvage Results:");
                    println!("  Manifests processed: {}", result.manifests_processed);
                    println!("  Corrupted manifests: {}", result.corrupted_manifests);
                    println!("  Files recovered: {}", result.files_recovered);
                    println!("  Chunks referenced: {}", result.chunks_referenced);
                    
                    if !result.errors.is_empty() {
                        println!("\nErrors:");
                        for error in &result.errors {
                            println!("  {}", error);
                        }
                    }
                    
                    println!("\nRecovered snapshots:");
                    for snapshot in &result.rebuilt_index {
                        let status = if snapshot.corrupted { "CORRUPTED" } else { "OK" };
                        println!("  {} - {} files ({})", 
                            snapshot.name.as_deref().unwrap_or("Unknown"),
                            snapshot.file_count, 
                            status
                        );
                    }
                }
            }
        }
        RecoveryActions::Validate { snapshot_id, format } => {
            let id = Uuid::parse_str(&snapshot_id).map_err(|_| Error::Configuration {
                reason: "Invalid snapshot ID format".to_string(),
            })?;

            let result = engine.validate_snapshot(&id)?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                OutputFormat::Table | OutputFormat::Yaml => {
                    println!("Validation Results:");
                    println!("  Snapshot ID: {}", result.snapshot_id);
                    println!("  Total files: {}", result.total_files);
                    println!("  Valid files: {}", result.valid_files);
                    println!("  Corrupted files: {}", result.corrupted_files);
                    println!("  Missing chunks: {}", result.missing_chunks);
                    
                    if !result.integrity_errors.is_empty() {
                        println!("\nIntegrity errors:");
                        for error in &result.integrity_errors {
                            println!("  {}: {:?} - {}", 
                                error.file_path.display(), 
                                error.error_type, 
                                error.details
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(not(feature = "recovery"))]
fn handle_recovery_commands(_cmd: RecoveryCommands, _root: &PathBuf) -> Result<()> {
    Err(Error::FeatureNotAvailable {
        feature: "recovery".to_string(),
    })
}