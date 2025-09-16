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

#[derive(Subcommand, Debug)]
enum Commands {
    List,
    Download,
    Clean,
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
    println!("  1. ✅ Check system dependencies");
    println!("  2. ✅ Set up Python virtual environment");  
    println!("  3. ✅ Verify model requirements match");
    println!("  4. ✅ Install Python dependencies");
    println!("  5. ✅ Download required ML models");
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

fn check_system_dependencies() {
    println!("═══════════════════════════════════════════════");
    println!("System Dependencies Check");
    println!("═══════════════════════════════════════════════");
    println!();

    let ffmpeg_check = process::Command::new("which").arg("ffmpeg").output();

    match ffmpeg_check {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("✅ ffmpeg found at: {}", path);
        }
        _ => {
            println!("⚠️  ffmpeg not found in PATH");
            println!("   For now, this is not required.");
            println!("   On macOS, install with: brew install ffmpeg");
        }
    }
    println!();
}

fn run_verification() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = get_project_root();

    println!("🔍 Project root: {}", project_root.display());
    println!();

    check_system_dependencies();

    println!("═══════════════════════════════════════════════");
    println!("Step 1: Python Environment Setup");
    println!("═══════════════════════════════════════════════");
    println!();

    let python_env = match PythonEnvironment::detect_and_setup(&project_root) {
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
    println!("Step 2: Model Requirements Check");
    println!("═══════════════════════════════════════════════");
    println!();

    let model_name = std::env::var("PARA_MODEL")
        .unwrap_or_else(|_| ml_utils::get_default_model());
    println!("🔍 Configured model: {}", model_name);

    if let Err(e) = python_env.ensure_correct_profile(&project_root, &model_name) {
        eprintln!("❌ Failed to set up model requirements: {}", e);
        return Err(Box::new(e));
    }

    println!();
    println!("═══════════════════════════════════════════════");
    println!("Step 3: Model Download");
    println!("═══════════════════════════════════════════════");
    println!();

    let downloader = ModelDownloader::new(Some(model_name))?;
    downloader.download_if_needed()?;

    Ok(())
}

fn main() {
    let _ = dotenv::dotenv();
    let _ = dotenv::from_filename(".env.local");

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::List) => {
            let inventory = ModelInventory::new();
            if let Err(e) = inventory.list_models() {
                eprintln!("❌ Failed to list models: {}", e);
                process::exit(1);
            }
        }
        Some(Commands::Download) => {
            println!("═══════════════════════════════════════════════");
            println!("Model Download");
            println!("═══════════════════════════════════════════════");
            println!();

            let downloader = match ModelDownloader::new(None) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("❌ Failed to initialize downloader: {}", e);
                    process::exit(1);
                }
            };

            if let Err(e) = downloader.download_if_needed() {
                eprintln!("❌ Failed to download model: {}", e);
                process::exit(1);
            }
        }
        Some(Commands::Clean) => {
            println!("═══════════════════════════════════════════════");
            println!("Clean Installation");
            println!("═══════════════════════════════════════════════");
            println!();

            let project_root = get_project_root();
            let venv_path = project_root.join("python").join("venv");

            if venv_path.exists() {
                println!("🗑️  Removing existing virtual environment...");
                if let Err(e) = std::fs::remove_dir_all(&venv_path) {
                    eprintln!("❌ Failed to remove virtual environment: {}", e);
                    process::exit(1);
                }
                println!("✅ Virtual environment removed");
            }

            println!("\nRunning fresh setup...\n");
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
