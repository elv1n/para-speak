use std::fs;
use std::path::Path;
use ml_utils::{get_models_dir, REQUIRED_FILES};

pub struct ModelInventory;

impl ModelInventory {
    pub fn new() -> Self {
        Self
    }
    
    pub fn list_models(&self) -> Result<(), Box<dyn std::error::Error>> {
        let models_dir = get_models_dir();
        let hub_dir = models_dir.join("hub");
        
        if !hub_dir.exists() {
            println!("No models directory found at: {}", hub_dir.display());
            return Ok(());
        }
        
        println!("╔══════════════════════════════════════════════╗");
        println!("║            Downloaded Models                 ║");
        println!("╚══════════════════════════════════════════════╝");
        println!();
        
        let mut model_entries = Vec::new();
        
        for entry in fs::read_dir(&hub_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() && path.file_name().unwrap().to_str().unwrap().starts_with("models--") {
                if let Some(model_info) = self.get_model_info(&path)? {
                    model_entries.push(model_info);
                }
            }
        }
        
        if model_entries.is_empty() {
            println!("No downloaded models found.");
            println!();
            println!("To download models, run:");
            println!("  cargo run -p verify-cli");
            return Ok(());
        }
        
        model_entries.sort_by(|a, b| a.0.cmp(&b.0));
        
        for (model_name, size_bytes) in model_entries {
            let size_display = self.format_size(size_bytes);
            println!("📦 {} - {}", model_name, size_display);
        }
        
        println!();
        
        Ok(())
    }
    
    fn get_model_info(&self, model_path: &Path) -> Result<Option<(String, u64)>, Box<dyn std::error::Error>> {
        let model_name = self.extract_model_name(model_path)?;
        let snapshots_dir = model_path.join("snapshots");
        
        if !snapshots_dir.exists() {
            return Ok(None);
        }
        
        let mut total_size = 0u64;
        let mut has_complete_model = false;
        
        for entry in fs::read_dir(&snapshots_dir)? {
            let entry = entry?;
            let snapshot_path = entry.path();
            
            if snapshot_path.is_dir() {
                let all_files_exist = REQUIRED_FILES.iter().all(|file| {
                    snapshot_path.join(file).exists()
                });
                
                if all_files_exist {
                    has_complete_model = true;
                    total_size = Self::calculate_directory_size(&snapshot_path)?;
                    break;
                }
            }
        }
        
        if has_complete_model {
            Ok(Some((model_name, total_size)))
        } else {
            Ok(None)
        }
    }
    
    fn extract_model_name(&self, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let dir_name = path.file_name()
            .ok_or("Invalid path")?
            .to_str()
            .ok_or("Invalid UTF-8 in path")?;
        
        if let Some(model_name) = dir_name.strip_prefix("models--") {
            Ok(model_name.replace("--", "/"))
        } else {
            Err("Invalid model directory name".into())
        }
    }
    
    fn calculate_directory_size(dir_path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
        let mut total_size = 0u64;
        
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                total_size += entry.metadata()?.len();
            } else if path.is_dir() {
                total_size += Self::calculate_directory_size(&path)?;
            }
        }
        
        Ok(total_size)
    }
    
    fn format_size(&self, bytes: u64) -> String {
        const GB: u64 = 1_073_741_824;
        const MB: u64 = 1_048_576;
        const KB: u64 = 1_024;
        
        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.0} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

impl Default for ModelInventory {
    fn default() -> Self {
        Self::new()
    }
}