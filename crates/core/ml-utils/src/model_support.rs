use std::path::{Path, PathBuf};
use std::fmt;
use std::fs;
use std::collections::HashMap;

pub struct ModelInfo {
    pub name: &'static str,
    pub profile: &'static str,
    pub required_files: &'static [&'static str],
    pub file_sizes: &'static [(&'static str, u64)],
}

impl fmt::Display for ModelInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

const PARAKEET_REQUIRED_FILES: &[&str] = &[
    "config.json",
    "model.safetensors",
    "tokenizer.model",
    "tokenizer.vocab",
];

const CANARY_REQUIRED_FILES: &[&str] = &[
    "canary-1b-v2.nemo",
];

const PARAKEET_TDT_0_6B_V3_SIZES: &[(&str, u64)] = &[
    ("config.json", 244093),
    ("model.safetensors", 2508288736),
    ("tokenizer.model", 360916),
    ("tokenizer.vocab", 101024),
];

const PARAKEET_TDT_1_1B_SIZES: &[(&str, u64)] = &[
    ("config.json", 37693),
    ("model.safetensors", 4282259416),
    ("tokenizer.model", 259162),
    ("tokenizer.vocab", 11383),
];

const PARAKEET_CTC_0_6B_SIZES: &[(&str, u64)] = &[
    ("config.json", 22393),
    ("model.safetensors", 2435504684),
    ("tokenizer.model", 259162),
    ("tokenizer.vocab", 11383),
];

const PARAKEET_CTC_1_1B_SIZES: &[(&str, u64)] = &[
    ("config.json", 22393),
    ("model.safetensors", 4250695964),
    ("tokenizer.model", 259162),
    ("tokenizer.vocab", 11383),
];

const CANARY_1B_V2_SIZES: &[(&str, u64)] = &[
    ("canary-1b-v2.nemo", 6358958080),
];

pub const AVAILABLE_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "mlx-community/parakeet-tdt-0.6b-v3",
        profile: "parakeet",
        required_files: PARAKEET_REQUIRED_FILES,
        file_sizes: PARAKEET_TDT_0_6B_V3_SIZES,
    },
    ModelInfo {
        name: "mlx-community/parakeet-tdt-1.1b",
        profile: "parakeet",
        required_files: PARAKEET_REQUIRED_FILES,
        file_sizes: PARAKEET_TDT_1_1B_SIZES,
    },
    ModelInfo {
        name: "mlx-community/parakeet-ctc-0.6b",
        profile: "parakeet",
        required_files: PARAKEET_REQUIRED_FILES,
        file_sizes: PARAKEET_CTC_0_6B_SIZES,
    },
    ModelInfo {
        name: "mlx-community/parakeet-ctc-1.1b",
        profile: "parakeet",
        required_files: PARAKEET_REQUIRED_FILES,
        file_sizes: PARAKEET_CTC_1_1B_SIZES,
    },
    ModelInfo {
        name: "nvidia/canary-1b-v2",
        profile: "canary",
        required_files: CANARY_REQUIRED_FILES,
        file_sizes: CANARY_1B_V2_SIZES,
    },
];

pub fn get_model_info(model_name: &str) -> Result<&'static ModelInfo, String> {
    AVAILABLE_MODELS
        .iter()
        .find(|m| m.name == model_name)
        .ok_or_else(|| format!("Model '{}' not found in supported models", model_name))
}

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

pub fn verify_model_files(model_name: &str) -> Result<(), String> {
    let model_info = get_model_info(model_name)?;
    let cache_path = get_model_cache_path(model_name);
    let snapshot_path = cache_path.join("snapshots").join("main");

    if !snapshot_path.exists() {
        return Err(format!("Model directory not found: {}", snapshot_path.display()));
    }

    let file_sizes: HashMap<&str, u64> = model_info.file_sizes.iter().copied().collect();

    for &file_name in model_info.required_files {
        let file_path = snapshot_path.join(file_name);

        if !file_path.exists() {
            return Err(format!("Required file missing: {}", file_name));
        }

        let actual_size = fs::metadata(&file_path)
            .map_err(|e| format!("Failed to get file metadata for {}: {}", file_name, e))?
            .len();

        if let Some(&expected_size) = file_sizes.get(file_name) {
            if actual_size != expected_size {
                let percent_downloaded = (actual_size as f64 / expected_size as f64) * 100.0;
                return Err(format!(
                    "File size mismatch for {}: expected {} bytes, got {} bytes ({:.1}% downloaded)",
                    file_name, expected_size, actual_size, percent_downloaded
                ));
            }
        }
    }

    Ok(())
}

pub fn model_exists(model_name: &str) -> bool {
    verify_model_files(model_name).is_ok()
}

pub fn model_exists_at_path(cache_path: &Path) -> bool {
    cache_path.exists()
}

pub fn get_default_model() -> String {
    AVAILABLE_MODELS[0].name.to_string()
}

pub fn is_model_supported(model_name: &str) -> bool {
    AVAILABLE_MODELS.iter().any(|m| m.name == model_name)
}

pub fn get_model_profile(model_name: &str) -> Result<&'static str, String> {
    get_model_info(model_name).map(|info| info.profile)
}

pub fn verify_python_requirements(model_name: &str) -> Result<(), String> {
    let expected_profile = get_model_profile(model_name)?;

    let project_root = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    let requirements_path = project_root.join("python").join("requirements.txt");

    if !requirements_path.exists() {
        return Err(format!(
            "Python requirements.txt not found at: {}",
            requirements_path.display()
        ));
    }

    let content = fs::read_to_string(&requirements_path)
        .map_err(|e| format!("Failed to read requirements.txt: {}", e))?;

    let expected_line = format!("-r requirements/{}.txt", expected_profile);

    if content.contains(&expected_line) {
        Ok(())
    } else {
        let current_profile = if content.contains("-r requirements/parakeet.txt") {
            "parakeet"
        } else if content.contains("-r requirements/canary.txt") {
            "canary"
        } else {
            "unknown"
        };

        Err(format!(
            "Python requirements mismatch: model '{}' requires '{}' profile but found '{}' profile",
            model_name, expected_profile, current_profile
        ))
    }
}