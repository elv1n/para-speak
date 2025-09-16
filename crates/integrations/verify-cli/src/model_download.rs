use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;
use std::io::{Read, Write};
use reqwest::blocking::Client;
use thiserror::Error;
use ml_utils::{get_models_dir, get_model_cache_path, get_model_info, get_default_model, verify_model_files};

#[derive(Error, Debug)]
pub enum ModelDownloadError {
    #[error("Failed to download file {0}: {1}")]
    DownloadFailed(String, String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Unknown error: {0}")]
    Unknown(String),
}


pub struct ModelDownloader {
    client: Client,
    model_name: String,
}

impl ModelDownloader {
    pub fn new(model_name: Option<String>) -> Result<Self, ModelDownloadError> {
        let model_name = model_name.unwrap_or_else(|| {
            std::env::var("PARA_MODEL")
                .unwrap_or_else(|_| get_default_model())
        });

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        let models_dir = get_models_dir();
        fs::create_dir_all(&models_dir)?;

        Ok(Self {
            client,
            model_name,
        })
    }
    
    pub fn download_if_needed(&self) -> Result<(), ModelDownloadError> {
        println!("🔍 Checking for model: {}", self.model_name);

        match verify_model_files(&self.model_name) {
            Ok(()) => {
                println!("✅ Model already downloaded and verified");
                return Ok(());
            }
            Err(err) => {
                if err.contains("Model directory not found") || err.contains("Required file missing") {
                    println!("📥 Model not found locally, starting download...");
                } else if err.contains("File size mismatch") {
                    println!("⚠️  Model files are incomplete:");
                    println!("   {}", err);
                    println!("📥 Re-downloading to fix incomplete files...");

                    self.remove_incomplete_files()?;
                } else {
                    println!("⚠️  Model verification failed: {}", err);
                    println!("📥 Re-downloading model...");
                }
            }
        }

        self.download_model()?;

        println!("🔍 Verifying downloaded files...");
        verify_model_files(&self.model_name)
            .map_err(|e| ModelDownloadError::Unknown(format!("Model verification failed after download: {}", e)))?;

        Ok(())
    }

    fn remove_incomplete_files(&self) -> Result<(), ModelDownloadError> {
        let cache_path = get_model_cache_path(&self.model_name);
        let snapshot_path = cache_path.join("snapshots").join("main");

        if snapshot_path.exists() {
            println!("🗑️  Removing incomplete files...");
            fs::remove_dir_all(&snapshot_path)?;
        }

        Ok(())
    }

    fn download_model(&self) -> Result<(), ModelDownloadError> {
        println!("   Calculating total download size...");
        println!();

        let model_info = get_model_info(&self.model_name)
            .map_err(ModelDownloadError::Unknown)?;

        let mut total_size = 0u64;
        let mut file_info = Vec::new();

        for file in model_info.required_files {
            let url = self.construct_download_url(file);
            match self.get_file_size(&url) {
                Ok(size) => {
                    file_info.push((file.to_string(), url, size));
                    total_size += size;
                }
                Err(e) => {
                    eprintln!("⚠️  Could not get size for {}: {}", file, e);
                    file_info.push((file.to_string(), url, 0));
                }
            }
        }
        
        if total_size > 0 {
            let total_size_gb = total_size as f64 / 1_073_741_824.0;
            println!("📦 Total download size: {:.2} GB", total_size_gb);
        }
        println!();
        
        let m = MultiProgress::new();
        
        let overall_pb = if total_size > 0 {
            let pb = m.add(ProgressBar::new(total_size));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:50.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
                    .unwrap()
                    .progress_chars("█▉▊▋▌▍▎▏  ")
            );
            pb.set_message("Overall progress");
            Some(pb)
        } else {
            None
        };
        
        let snapshot_path = self.ensure_cache_dirs()?;
        
        let mut overall_downloaded = 0u64;
        
        for (file_name, url, expected_size) in file_info {
            let dest_path = snapshot_path.join(&file_name);
            
            if dest_path.exists() {
                println!("⏭️  {} already exists, skipping", file_name);
                if let Some(ref pb) = overall_pb {
                    overall_downloaded += expected_size;
                    pb.set_position(overall_downloaded);
                }
                continue;
            }
            
            let file_pb = if expected_size > 0 {
                let pb = m.add(ProgressBar::new(expected_size));
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("  {msg:30} [{bar:30.yellow/white}] {bytes}/{total_bytes} ({bytes_per_sec})")
                        .unwrap()
                        .progress_chars("═╾─")
                );
                pb.set_message(file_name.clone());
                Some(pb)
            } else {
                println!("  Downloading {} (size unknown)...", file_name);
                None
            };
            
            match self.download_file_with_progress(&url, &dest_path, 
                                                  file_pb.as_ref().unwrap_or(&ProgressBar::hidden()), 
                                                  &file_name) {
                Ok(_) => {
                    if let Some(pb) = file_pb {
                        pb.finish_with_message(format!("✓ {}", file_name));
                        if let Some(ref overall) = overall_pb {
                            overall_downloaded += expected_size;
                            overall.set_position(overall_downloaded);
                        }
                    } else {
                        println!("  ✓ {} downloaded", file_name);
                    }
                }
                Err(e) => {
                    if let Some(pb) = file_pb {
                        pb.finish_with_message(format!("✗ {} - Failed", file_name));
                    }
                    eprintln!("❌ Error downloading {}: {}", file_name, e);
                    
                    let _ = fs::remove_file(&dest_path);
                    
                    return Err(ModelDownloadError::DownloadFailed(file_name, e.to_string()));
                }
            }
        }
        
        if let Some(pb) = overall_pb {
            pb.finish_with_message("Download complete!");
        }
        
        println!();
        println!("✅ Successfully downloaded model {}", self.model_name);
        println!("   Model location: {:?}", snapshot_path);
        
        Ok(())
    }
    
    fn construct_download_url(&self, filename: &str) -> String {
        format!("https://huggingface.co/{}/resolve/main/{}", self.model_name, filename)
    }
    
    fn get_file_size(&self, url: &str) -> Result<u64, ModelDownloadError> {
        let response = self.client.head(url).send()?;
        
        if !response.status().is_success() {
            return Err(ModelDownloadError::Unknown(format!("Failed to get file info: {}", response.status())));
        }
        
        response.headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse::<u64>().ok())
            .ok_or_else(|| ModelDownloadError::Unknown("Failed to get content length".to_string()))
    }
    
    fn ensure_cache_dirs(&self) -> Result<PathBuf, ModelDownloadError> {
        let cache_path = get_model_cache_path(&self.model_name);
        let snapshots_dir = cache_path.join("snapshots");
        
        fs::create_dir_all(&snapshots_dir)?;
        
        let snapshot_hash = "main";
        let snapshot_path = snapshots_dir.join(snapshot_hash);
        fs::create_dir_all(&snapshot_path)?;
        
        Ok(snapshot_path)
    }
    
    fn download_file_with_progress(
        &self,
        url: &str,
        dest_path: &PathBuf,
        pb: &ProgressBar,
        file_name: &str,
    ) -> Result<(), ModelDownloadError> {
        let mut response = self.client.get(url).send()?;
        
        if !response.status().is_success() {
            return Err(ModelDownloadError::DownloadFailed(
                file_name.to_string(),
                response.status().to_string()
            ));
        }
        
        fs::create_dir_all(dest_path.parent().unwrap())?;
        let mut file = fs::File::create(dest_path)?;
        
        pb.set_message(format!("Downloading {}", file_name));
        
        let mut downloaded = 0u64;
        let mut buffer = vec![0; 65536];
        
        loop {
            let bytes_read = response.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            
            file.write_all(&buffer[..bytes_read])?;
            downloaded += bytes_read as u64;
            pb.set_position(downloaded);
        }
        
        file.flush()?;
        Ok(())
    }
}