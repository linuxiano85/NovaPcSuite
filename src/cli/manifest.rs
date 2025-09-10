//! Manifest command implementation for managing backup manifests.

use clap::Args;
use std::path::PathBuf;
use crate::Result;
use tokio::fs;

/// Arguments for the manifest command
#[derive(Args)]
pub struct ManifestArgs {
    /// Backup output directory containing manifests
    #[arg(short, long)]
    pub backup_dir: PathBuf,

    /// List all available manifests
    #[arg(short, long)]
    pub list: bool,

    /// Show detailed manifest for specific ID
    #[arg(long)]
    pub show: Option<String>,

    /// Verify manifest integrity
    #[arg(long)]
    pub verify: Option<String>,

    /// Output format (json, summary)
    #[arg(long, default_value = "summary")]
    pub format: String,
}

/// Run the manifest command
pub async fn run(args: ManifestArgs) -> Result<()> {
    let manifests_dir = args.backup_dir.join("manifests");

    if !manifests_dir.exists() {
        println!("No manifests directory found at: {}", manifests_dir.display());
        return Ok(());
    }

    if args.list {
        list_manifests(&manifests_dir).await?;
        return Ok(());
    }

    if let Some(manifest_id) = args.show {
        show_manifest(&manifests_dir, &manifest_id, &args.format).await?;
        return Ok(());
    }

    if let Some(manifest_id) = args.verify {
        verify_manifest(&manifests_dir, &args.backup_dir, &manifest_id).await?;
        return Ok(());
    }

    // Default: show latest manifest
    show_latest_manifest(&manifests_dir, &args.format).await?;

    Ok(())
}

async fn list_manifests(manifests_dir: &PathBuf) -> Result<()> {
    println!("Available backup manifests:\n");

    let mut entries = fs::read_dir(manifests_dir).await?;
    let mut manifests = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                if filename.starts_with("manifest-") {
                    // Try to read the manifest to get metadata
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(manifest) = serde_json::from_str::<crate::backup::Manifest>(&content) {
                            manifests.push(manifest);
                        }
                    }
                }
            }
        }
    }

    if manifests.is_empty() {
        println!("No backup manifests found.");
        return Ok(());
    }

    // Sort by creation date (most recent first)
    manifests.sort_by(|a, b| b.created.cmp(&a.created));

    println!("{:<38} {:<20} {:<15} {:<10} {:<15}", "Manifest ID", "Created", "Label", "Files", "Size");
    println!("{}", "-".repeat(100));

    for manifest in manifests {
        println!(
            "{:<38} {:<20} {:<15} {:<10} {:<15}",
            manifest.id(),
            manifest.created.format("%Y-%m-%d %H:%M:%S"),
            manifest.label,
            manifest.files.len(),
            format_bytes(manifest.total_size)
        );
    }

    Ok(())
}

async fn show_manifest(manifests_dir: &PathBuf, manifest_id: &str, format: &str) -> Result<()> {
    let manifest_path = manifests_dir.join(format!("manifest-{}.json", manifest_id));

    if !manifest_path.exists() {
        println!("Manifest not found for ID: {}", manifest_id);
        return Ok(());
    }

    let content = fs::read_to_string(&manifest_path).await?;
    let manifest: crate::backup::Manifest = serde_json::from_str(&content)?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        }
        "summary" | _ => {
            print_manifest_summary(&manifest);
        }
    }

    Ok(())
}

async fn show_latest_manifest(manifests_dir: &PathBuf, format: &str) -> Result<()> {
    let mut entries = fs::read_dir(manifests_dir).await?;
    let mut latest_manifest = None;
    let mut latest_time = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                if filename.starts_with("manifest-") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(manifest) = serde_json::from_str::<crate::backup::Manifest>(&content) {
                            if latest_time.is_none() || manifest.created > latest_time.unwrap() {
                                latest_time = Some(manifest.created);
                                latest_manifest = Some(manifest);
                            }
                        }
                    }
                }
            }
        }
    }

    match latest_manifest {
        Some(manifest) => {
            match format {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&manifest)?);
                }
                "summary" | _ => {
                    print_manifest_summary(&manifest);
                }
            }
        }
        None => {
            println!("No backup manifests found.");
        }
    }

    Ok(())
}

async fn verify_manifest(manifests_dir: &PathBuf, backup_dir: &PathBuf, manifest_id: &str) -> Result<()> {
    let manifest_path = manifests_dir.join(format!("manifest-{}.json", manifest_id));

    if !manifest_path.exists() {
        println!("Manifest not found for ID: {}", manifest_id);
        return Ok(());
    }

    let content = fs::read_to_string(&manifest_path).await?;
    let manifest: crate::backup::Manifest = serde_json::from_str(&content)?;

    println!("Verifying manifest: {}", manifest_id);
    println!("Checking chunk integrity...\n");

    let chunks_dir = backup_dir.join("chunks");
    let mut missing_chunks = Vec::new();
    let mut total_chunks = 0;

    for file_entry in &manifest.files {
        for chunk in &file_entry.chunks {
            total_chunks += 1;
            let chunk_path = chunks_dir.join(&chunk.id);
            
            if !chunk_path.exists() {
                missing_chunks.push((file_entry.path.clone(), chunk.id.clone()));
            } else {
                // Verify chunk hash
                if let Ok(chunk_data) = fs::read(&chunk_path).await {
                    let actual_hash = blake3::hash(&chunk_data);
                    if actual_hash.as_bytes() != chunk.hash.as_slice() {
                        println!("WARNING: Hash mismatch for chunk {} in file {}", 
                            chunk.id, file_entry.path.display());
                    }
                }
            }
        }
    }

    if missing_chunks.is_empty() {
        println!("✓ Verification completed successfully");
        println!("  All {} chunks are present and valid", total_chunks);
    } else {
        println!("✗ Verification failed");
        println!("  {} missing chunks out of {}", missing_chunks.len(), total_chunks);
        println!("\nMissing chunks:");
        for (file_path, chunk_id) in missing_chunks.iter().take(10) {
            println!("  {} (chunk: {})", file_path.display(), chunk_id);
        }
        if missing_chunks.len() > 10 {
            println!("  ... and {} more", missing_chunks.len() - 10);
        }
    }

    Ok(())
}

fn print_manifest_summary(manifest: &crate::backup::Manifest) {
    println!("Backup Manifest Summary");
    println!("======================\n");
    println!("Manifest ID:     {}", manifest.id());
    println!("Label:           {}", manifest.label);
    println!("Created:         {}", manifest.created.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Source Path:     {}", manifest.source_path.display());
    println!("Total Files:     {}", manifest.files.len());
    println!("Total Size:      {}", format_bytes(manifest.total_size));
    println!("Total Chunks:    {}", manifest.chunk_count);

    if !manifest.files.is_empty() {
        println!("\nSample Files:");
        for file in manifest.files.iter().take(5) {
            println!("  {} ({} chunks, {})", 
                file.path.display(), 
                file.chunks.len(),
                format_bytes(file.size)
            );
        }
        if manifest.files.len() > 5 {
            println!("  ... and {} more files", manifest.files.len() - 5);
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}