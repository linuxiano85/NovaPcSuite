//! Backup report generation for JSON and HTML formats.
//! 
//! This module provides reporting capabilities for completed backup snapshots,
//! generating both machine-readable JSON and human-readable HTML reports.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

use super::Manifest;

/// Backup report generator
#[derive(Debug)]
pub struct ReportGenerator {
    output_dir: std::path::PathBuf,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new(output_dir: &Path) -> Self {
        Self {
            output_dir: output_dir.to_path_buf(),
        }
    }

    /// Generate a comprehensive backup report
    pub async fn generate_report(&self, manifest: &Manifest) -> Result<BackupReport> {
        let report = BackupReport::from_manifest(manifest);

        // Write JSON report
        self.write_json_report(&report).await?;

        // Write HTML report
        self.write_html_report(&report).await?;

        Ok(report)
    }

    /// Write JSON format report
    async fn write_json_report(&self, report: &BackupReport) -> Result<()> {
        let reports_dir = self.output_dir.join("reports");
        fs::create_dir_all(&reports_dir).await?;

        let filename = format!("report-{}.json", report.manifest_id);
        let json_path = reports_dir.join(filename);

        let json_content = serde_json::to_string_pretty(report)?;
        fs::write(json_path, json_content).await?;

        Ok(())
    }

    /// Write HTML format report
    async fn write_html_report(&self, report: &BackupReport) -> Result<()> {
        let reports_dir = self.output_dir.join("reports");
        fs::create_dir_all(&reports_dir).await?;

        let filename = format!("report-{}.html", report.manifest_id);
        let html_path = reports_dir.join(filename);

        let html_content = self.generate_html_content(report)?;
        fs::write(html_path, html_content).await?;

        Ok(())
    }

    /// Generate HTML content for the report
    fn generate_html_content(&self, report: &BackupReport) -> Result<String> {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>NovaPcSuite Backup Report - {}</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            padding: 30px;
        }}
        .header {{
            border-bottom: 2px solid #007acc;
            padding-bottom: 20px;
            margin-bottom: 30px;
        }}
        .title {{
            color: #007acc;
            margin: 0;
            font-size: 28px;
        }}
        .subtitle {{
            color: #666;
            margin: 5px 0 0 0;
            font-size: 16px;
        }}
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        .stat-card {{
            background: #f8f9fa;
            border: 1px solid #e9ecef;
            border-radius: 6px;
            padding: 20px;
            text-align: center;
        }}
        .stat-value {{
            font-size: 24px;
            font-weight: bold;
            color: #007acc;
            margin-bottom: 5px;
        }}
        .stat-label {{
            color: #666;
            font-size: 14px;
        }}
        .section {{
            margin-bottom: 30px;
        }}
        .section-title {{
            color: #333;
            border-bottom: 1px solid #ddd;
            padding-bottom: 10px;
            margin-bottom: 20px;
            font-size: 20px;
        }}
        .file-list {{
            background: #f8f9fa;
            border-radius: 6px;
            padding: 20px;
            max-height: 400px;
            overflow-y: auto;
        }}
        .file-item {{
            display: flex;
            justify-content: space-between;
            padding: 8px 0;
            border-bottom: 1px solid #e9ecef;
        }}
        .file-item:last-child {{
            border-bottom: none;
        }}
        .file-path {{
            color: #333;
            font-family: monospace;
        }}
        .file-size {{
            color: #666;
            font-size: 14px;
        }}
        .efficiency-bar {{
            background: #e9ecef;
            border-radius: 10px;
            height: 20px;
            overflow: hidden;
            margin: 10px 0;
        }}
        .efficiency-fill {{
            background: linear-gradient(90deg, #28a745, #20c997);
            height: 100%;
            transition: width 0.3s ease;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1 class="title">NovaPcSuite Backup Report</h1>
            <p class="subtitle">Backup completed on {}</p>
        </div>

        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Files Backed Up</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Size</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Chunks Created</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{:.1}%</div>
                <div class="stat-label">Compression Ratio</div>
            </div>
        </div>

        <div class="section">
            <h2 class="section-title">Storage Efficiency</h2>
            <div class="efficiency-bar">
                <div class="efficiency-fill" style="width: {:.1}%"></div>
            </div>
            <p>Deduplication and compression achieved {:.1}% storage efficiency</p>
        </div>

        <div class="section">
            <h2 class="section-title">Backup Details</h2>
            <p><strong>Backup ID:</strong> {}</p>
            <p><strong>Label:</strong> {}</p>
            <p><strong>Source Path:</strong> <code>{}</code></p>
            <p><strong>Duration:</strong> N/A (tracking not implemented yet)</p>
        </div>

        <div class="section">
            <h2 class="section-title">Files Processed ({})</h2>
            <div class="file-list">
                {}
            </div>
        </div>

        <div class="section">
            <h2 class="section-title">Technical Details</h2>
            <p><strong>Chunking Algorithm:</strong> Adaptive (2 MiB default)</p>
            <p><strong>Hash Algorithm:</strong> BLAKE3</p>
            <p><strong>Merkle Tree:</strong> Enabled for integrity verification</p>
            <p><strong>Content Addressing:</strong> Enabled for deduplication</p>
        </div>
    </div>
</body>
</html>"#,
            report.label,
            report.completed_at.format("%Y-%m-%d %H:%M:%S UTC"),
            report.total_files,
            Self::format_bytes(report.total_size),
            report.total_chunks,
            report.compression_ratio * 100.0,
            report.storage_efficiency * 100.0,
            report.storage_efficiency * 100.0,
            report.manifest_id,
            report.label,
            report.source_path.display(),
            report.total_files,
            self.generate_file_list_html(&report.files)
        );

        Ok(html)
    }

    /// Generate HTML for file list
    fn generate_file_list_html(&self, files: &[FileInfo]) -> String {
        files
            .iter()
            .map(|file| {
                format!(
                    r#"<div class="file-item">
                        <span class="file-path">{}</span>
                        <span class="file-size">{}</span>
                    </div>"#,
                    file.path.display(),
                    Self::format_bytes(file.size)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format bytes in human-readable format
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
}

/// Comprehensive backup report
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupReport {
    pub manifest_id: String,
    pub label: String,
    pub completed_at: DateTime<Utc>,
    pub source_path: std::path::PathBuf,
    pub total_files: usize,
    pub total_size: u64,
    pub total_chunks: usize,
    pub compression_ratio: f64,
    pub storage_efficiency: f64,
    pub files: Vec<FileInfo>,
}

impl BackupReport {
    /// Create a report from a backup manifest
    pub fn from_manifest(manifest: &Manifest) -> Self {
        let total_chunk_size: u64 = manifest
            .files
            .iter()
            .flat_map(|f| &f.chunks)
            .map(|c| c.size)
            .sum();

        let compression_ratio = if manifest.total_size > 0 {
            total_chunk_size as f64 / manifest.total_size as f64
        } else {
            1.0
        };

        let storage_efficiency = if total_chunk_size > 0 {
            (manifest.total_size as f64 - total_chunk_size as f64) / manifest.total_size as f64
        } else {
            0.0
        };

        let files = manifest
            .files
            .iter()
            .map(|f| FileInfo {
                path: f.path.clone(),
                size: f.size,
                chunks: f.chunks.len(),
            })
            .collect();

        Self {
            manifest_id: manifest.id.to_string(),
            label: manifest.label.clone(),
            completed_at: manifest.created,
            source_path: manifest.source_path.clone(),
            total_files: manifest.files.len(),
            total_size: manifest.total_size,
            total_chunks: manifest.chunk_count,
            compression_ratio,
            storage_efficiency: storage_efficiency.max(0.0),
            files,
        }
    }
}

/// Information about a backed up file
#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: std::path::PathBuf,
    pub size: u64,
    pub chunks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_report_generation() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ReportGenerator::new(temp_dir.path());

        // Create a dummy manifest
        let manifest = Manifest {
            id: Uuid::new_v4(),
            created: Utc::now(),
            label: "test-backup".to_string(),
            source_path: temp_dir.path().to_path_buf(),
            files: vec![],
            chunk_count: 5,
            total_size: 1024,
        };

        let report = generator.generate_report(&manifest).await.unwrap();

        assert_eq!(report.label, "test-backup");
        assert_eq!(report.total_chunks, 5);
        assert_eq!(report.total_size, 1024);

        // Check that files were created
        let json_path = temp_dir.path().join("reports").join(format!("report-{}.json", manifest.id));
        let html_path = temp_dir.path().join("reports").join(format!("report-{}.html", manifest.id));

        assert!(json_path.exists());
        assert!(html_path.exists());
    }
}