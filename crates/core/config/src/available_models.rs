pub const AVAILABLE_MODELS: &[&str] = &[
    "mlx-community/parakeet-tdt-0.6b-v3",
    "mlx-community/parakeet-tdt-1.1b",
    "mlx-community/parakeet-ctc-0.6b",
    "mlx-community/parakeet-ctc-1.1b",
];

pub fn get_default_model() -> String {
    AVAILABLE_MODELS[0].to_string()
}

pub fn is_model_supported(model_name: &str) -> bool {
    AVAILABLE_MODELS.contains(&model_name)
}