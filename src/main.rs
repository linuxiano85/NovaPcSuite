//! NovaPcSuite - Advanced PC backup and maintenance suite
//! 
//! Main binary entry point for the command-line interface.

use clap::Parser;
use nova_pc_suite::cli::{Cli, Commands};
use nova_pc_suite::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Backup(args) => {
            nova_pc_suite::cli::backup::run(args).await
        }
        Commands::Scan(args) => {
            nova_pc_suite::cli::scan::run(args).await
        }
        Commands::Report(args) => {
            nova_pc_suite::cli::report::run(args).await
        }
        Commands::Manifest(args) => {
            nova_pc_suite::cli::manifest::run(args).await
        }
        Commands::Devices(args) => {
            nova_pc_suite::cli::devices::run(args).await
        }
    }
}