mod model_download;
mod model_list;
mod python_env;

use clap::{Parser, Subcommand};
use model_download::ModelDownloader;
use model_list::ModelInventory;
use python_env::PythonEnvironment;
use std::env;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "verify-cli")]
#[command(about = "Para-Speak Environment Verifier")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    List,
}

fn get_project_root() -> PathBuf {
    env::current_dir().expect("Failed to get current directory")
}

fn print_header() {
    println!();
    println!("╔══════════════════════════════════════════════╗");
    println!("║       Para-Speak Environment Verifier        ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();
    println!("This tool will:");
    println!("  1. ✅ Set up Python virtual environment");
    println!("  2. ✅ Install all Python dependencies");
    println!("  3. ✅ Download required ML models");
    println!("  4. ✅ Configure environment variables");
    println!();
}

fn print_success() {
    println!();
    println!("╔══════════════════════════════════════════════╗");
    println!("║    🎉 Environment Setup Complete! 🎉         ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();
    println!("Your Para-Speak environment is ready!");
    println!();
    println!("You can now run:");
    println!("  ./para-speak");
    println!();
}

fn run_verification() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = get_project_root();

    println!("🔍 Project root: {}", project_root.display());
    println!();

    println!("═══════════════════════════════════════════════");
    println!("Step 1: Python Environment Setup");
    println!("═══════════════════════════════════════════════");
    println!();

    let _python_env = match PythonEnvironment::detect_and_setup(&project_root) {
        Ok(env) => env,
        Err(e) => {
            eprintln!("❌ Failed to set up Python environment: {}", e);
            eprintln!();
            eprintln!("Please ensure Python 3.10+ is installed and try again.");
            eprintln!("On macOS, you can install Python with:");
            eprintln!("  brew install python@3.13");
            return Err(Box::new(e));
        }
    };

    println!();
    println!("═══════════════════════════════════════════════");
    println!("Step 2: Model Download");
    println!("═══════════════════════════════════════════════");
    println!();

    let downloader = ModelDownloader::new()?;
    downloader.download_if_needed()?;

    Ok(())
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::List) => {
            let inventory = ModelInventory::new();
            if let Err(e) = inventory.list_models() {
                eprintln!("❌ Failed to list models: {}", e);
                process::exit(1);
            }
        }
        None => {
            print_header();

            match run_verification() {
                Ok(()) => {
                    print_success();
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!();
                    eprintln!("❌ Verification failed: {}", e);
                    eprintln!();
                    eprintln!("Please fix the issues above and try again.");
                    eprintln!("If you continue to have problems, please report an issue at:");
                    eprintln!("  https://github.com/your-repo/para-speak/issues");
                    process::exit(1);
                }
            }
        }
    }
}
