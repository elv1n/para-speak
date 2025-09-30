use thiserror::Error;

#[derive(Error, Debug)]
pub enum TranscriptionError {
    #[error("Transcription engine not initialized")]
    NotInitialized,

    #[error("Model not loaded")]
    ModelNotLoaded,

    #[error("Empty audio data provided")]
    EmptyAudioData,

    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    #[error("Failed to lock transcription engine: {0}")]
    LockError(String),

    #[error("Initialization failed: {0}")]
    InitializationError(String),

    #[error("Model loading failed: {0}")]
    ModelLoadingError(String),

    #[error("Invalid audio format or parameters: {0}")]
    InvalidAudioFormat(String),

    #[error("Service temporarily unavailable: {0}")]
    ServiceUnavailable(String),
}

pub type Result<T> = std::result::Result<T, TranscriptionError>;

impl<T> From<std::sync::PoisonError<T>> for TranscriptionError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        TranscriptionError::LockError(err.to_string())
    }
}

impl From<audio::AudioError> for TranscriptionError {
    fn from(err: audio::AudioError) -> Self {
        TranscriptionError::InvalidAudioFormat(err.to_string())
    }
}
