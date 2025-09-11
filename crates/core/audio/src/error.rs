use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Audio device error: {0}")]
    DeviceError(String),

    #[error("Stream error: {0}")]
    StreamError(#[from] cpal::StreamError),

    #[error("Build stream error: {0}")]
    BuildStreamError(#[from] cpal::BuildStreamError),

    #[error("Play stream error: {0}")]
    PlayStreamError(#[from] cpal::PlayStreamError),

    #[error("Default stream config error: {0}")]
    DefaultStreamConfigError(#[from] cpal::DefaultStreamConfigError),

    #[error("Unsupported sample format: {0:?}")]
    UnsupportedFormat(cpal::SampleFormat),

    #[error("Channel communication error: {0}")]
    ChannelError(String),

    #[error("Recording state error: {0}")]
    StateError(String),

    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("WAV file error: {0}")]
    WavError(#[from] hound::Error),

    #[error("No audio input device available")]
    NoInputDevice,

    #[error("No audio output device available")]
    NoOutputDevice,

    #[error("Recording already in progress")]
    AlreadyRecording,

    #[error("Recording not active")]
    NotRecording,

    #[error("Recording already paused")]
    AlreadyPaused,

    #[error("Recording not paused")]
    NotPaused,

    #[error("Thread communication failed")]
    ThreadCommunicationFailed,
}

pub type Result<T> = std::result::Result<T, AudioError>;
