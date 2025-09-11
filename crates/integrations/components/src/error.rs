use thiserror::Error;

#[derive(Error, Debug)]
pub enum ComponentError {
    #[error("Component initialization failed: {0}")]
    InitializationFailed(String),
    #[error("Component execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Component configuration error: {0}")]
    ConfigurationError(String),
    #[error("Audio error: {0}")]
    AudioError(#[from] audio::AudioError),
    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}