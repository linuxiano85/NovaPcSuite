//! Nova CLI - Command Line Interface for NovaPcSuite utilities
//! 
//! Provides command-line access to various utilities including the Italian Codice Fiscale
//! generator and validator.

use anyhow::Result;
use chrono::NaiveDate;
use clap::{Args, Parser, Subcommand};

#[cfg(feature = "cf")]
mod util;

#[cfg(feature = "cf")]
use util::codice_fiscale::{self, CfInput, ComuneIndex, Sex};

/// Nova CLI - Command Line Interface for NovaPcSuite utilities
#[derive(Parser)]
#[command(name = "nova-cli")]
#[command(about = "NovaPcSuite command line utilities")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[cfg(feature = "cf")]
    /// Italian Codice Fiscale utilities
    Cf {
        #[command(subcommand)]
        action: CfCommands,
    },
}

#[cfg(feature = "cf")]
#[derive(Subcommand)]
enum CfCommands {
    /// Generate a Codice Fiscale
    Generate(GenerateArgs),
    /// Validate a Codice Fiscale
    Validate(ValidateArgs),
    /// Decode information from a Codice Fiscale
    Decode(DecodeArgs),
}

#[cfg(feature = "cf")]
#[derive(Args)]
struct GenerateArgs {
    /// Surname (cognome)
    #[arg(long)]
    surname: String,
    
    /// Given name (nome)  
    #[arg(long)]
    name: String,
    
    /// Date of birth (YYYY-MM-DD)
    #[arg(long)]
    birth_date: String,
    
    /// Sex (M/F)
    #[arg(long)]
    sex: char,
    
    /// Birthplace code (4 characters, optional)
    #[arg(long)]
    birthplace_code: Option<String>,
    
    /// Comune name (used if birthplace_code not provided)
    #[arg(long)]
    comune: Option<String>,
}

#[cfg(feature = "cf")]
#[derive(Args)]
struct ValidateArgs {
    /// Codice Fiscale to validate
    codice_fiscale: String,
}

#[cfg(feature = "cf")]
#[derive(Args)]
struct DecodeArgs {
    /// Codice Fiscale to decode
    codice_fiscale: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        #[cfg(feature = "cf")]
        Commands::Cf { action } => handle_cf_command(action).await,
    }
}

#[cfg(feature = "cf")]
async fn handle_cf_command(action: CfCommands) -> Result<()> {
    match action {
        CfCommands::Generate(args) => {
            let birth_date = NaiveDate::parse_from_str(&args.birth_date, "%Y-%m-%d")
                .map_err(|e| anyhow::anyhow!("Invalid date format: {}. Use YYYY-MM-DD", e))?;
            
            let sex = Sex::from_char(args.sex)
                .map_err(|e| anyhow::anyhow!("Invalid sex: {}. Use M or F", e))?;
            
            let input = CfInput {
                surname: args.surname,
                name: args.name,
                birth_date,
                sex,
                birthplace_code: args.birthplace_code,
                comune_name: args.comune,
            };
            
            let comune_index = ComuneIndex::new();
            let cf = codice_fiscale::generate(&input, &comune_index)?;
            
            println!("Generated Codice Fiscale: {}", cf);
            Ok(())
        }
        
        CfCommands::Validate(args) => {
            match codice_fiscale::validate(&args.codice_fiscale) {
                Ok(is_valid) => {
                    if is_valid {
                        println!("✓ Codice Fiscale '{}' is VALID", args.codice_fiscale);
                    } else {
                        println!("✗ Codice Fiscale '{}' is INVALID (wrong control character)", args.codice_fiscale);
                    }
                    Ok(())
                }
                Err(e) => {
                    println!("✗ Codice Fiscale '{}' is INVALID: {}", args.codice_fiscale, e);
                    Ok(())
                }
            }
        }
        
        CfCommands::Decode(args) => {
            match codice_fiscale::decode(&args.codice_fiscale) {
                Ok(decoded) => {
                    println!("Decoded information from '{}':", args.codice_fiscale);
                    println!("  Birth Year: {}", decoded.birth_year);
                    println!("  Birth Month: {}", decoded.birth_month);
                    println!("  Birth Day: {}", decoded.birth_day);
                    println!("  Sex: {}", decoded.sex.to_char());
                    println!("  Birthplace Code: {}", decoded.birthplace_code);
                    Ok(())
                }
                Err(e) => {
                    println!("Failed to decode '{}': {}", args.codice_fiscale, e);
                    Ok(())
                }
            }
        }
    }
}