//! Scan command implementation for file analysis.

use clap::Args;
use std::path::PathBuf;
use crate::dedupe::DedupeEngine;
use crate::Result;
use walkdir::WalkDir;

/// Arguments for the scan command
#[derive(Args)]
pub struct ScanArgs {
    /// Directory to scan
    #[arg(short, long)]
    pub path: PathBuf,

    /// Output analysis to JSON file
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Show similarity clusters
    #[arg(long)]
    pub find_similar: bool,

    /// Include hidden files
    #[arg(long)]
    pub include_hidden: bool,
}

/// Run the scan command
pub async fn run(args: ScanArgs) -> Result<()> {
    println!("Scanning directory: {}", args.path.display());

    let dedupe_engine = DedupeEngine::new();
    let mut entries = Vec::new();
    let mut file_count = 0;

    // Walk through directory
    for entry in WalkDir::new(&args.path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path().to_path_buf();

            // Skip hidden files unless requested
            if !args.include_hidden {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with('.') {
                        continue;
                    }
                }
            }

            let result = dedupe_engine.analyze_file(&path);
            entries.push(crate::dedupe::DedupeEntry { path, result });
            file_count += 1;

            if file_count % 100 == 0 {
                println!("Processed {} files...", file_count);
            }
        }
    }

    println!("Scan completed: {} files analyzed", file_count);

    // Find similar files if requested
    if args.find_similar {
        println!("Analyzing for similar files...");
        let clusters = dedupe_engine.find_similar(&entries);
        
        if clusters.is_empty() {
            println!("No similar file clusters found.");
        } else {
            println!("Found {} similarity clusters:", clusters.len());
            for (i, cluster) in clusters.iter().enumerate() {
                println!("  Cluster {} ({:?}): {} files", i + 1, cluster.cluster_type, cluster.files.len());
                for file in &cluster.files {
                    println!("    - {}", file.display());
                }
            }
        }
    }

    // Output to JSON if requested
    if let Some(output_path) = args.output {
        println!("Writing analysis to: {}", output_path.display());
        let json_output = serde_json::to_string_pretty(&entries)?;
        tokio::fs::write(output_path, json_output).await?;
        println!("Analysis saved successfully.");
    }

    Ok(())
}