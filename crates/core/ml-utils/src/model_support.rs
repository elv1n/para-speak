use std::path::{Path, PathBuf};
use std::fs;

pub const REQUIRED_FILES: &[&str] = &[
    "config.json",
    "model.safetensors", 
    "tokenizer.model",
    "tokenizer.vocab"
];

pub const AVAILABLE_MODELS: &[&str] = &[
    "mlx-community/parakeet-tdt-0.6b-v3",
    "mlx-community/parakeet-tdt-1.1b",
    "mlx-community/parakeet-ctc-0.6b",
    "mlx-community/parakeet-ctc-1.1b",
];

pub fn get_models_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("models")
}

pub fn get_model_cache_path(model_name: &str) -> PathBuf {
    let models_dir = get_models_dir();
    let model_id = model_name.replace("/", "--");
    models_dir.join("hub").join(format!("models--{}", model_id))
}

pub fn model_exists(model_name: &str) -> bool {
    model_exists_at_path(&get_model_cache_path(model_name))
}

pub fn model_exists_at_path(cache_path: &Path) -> bool {
    if !cache_path.exists() {
        return false;
    }

    let snapshot_dir = cache_path.join("snapshots");
    if !snapshot_dir.exists() {
        return false;
    }

    if let Ok(entries) = fs::read_dir(&snapshot_dir) {
        for entry in entries.flatten() {
            let snapshot_path = entry.path();
            if snapshot_path.is_dir() {
                let all_files_exist = REQUIRED_FILES.iter().all(|file| {
                    snapshot_path.join(file).exists()
                });
                
                if all_files_exist {
                    return true;
                }
            }
        }
    }

    false
}

pub fn get_default_model() -> String {
    AVAILABLE_MODELS[0].to_string()
}

pub fn is_model_supported(model_name: &str) -> bool {
    AVAILABLE_MODELS.contains(&model_name)
}