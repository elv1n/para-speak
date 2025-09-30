use crate::ml_error::{Result, TranscriptionError};
use config::Config;
use para_log::{debug, info};
use ml_utils::{get_model_cache_path, model_exists};
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicU8, Ordering};
use transcribe_rs::engines::parakeet::ParakeetEngine;
use transcribe_rs::TranscriptionEngine;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
enum ModelState {
    NotInitialized = 0,
    Initialized = 1,
    ModelLoaded = 2,
}

pub struct MLEngine {
    engine: Option<ManuallyDrop<ParakeetEngine>>,
    model_state: AtomicU8,
    current_model: Option<String>,
}

impl MLEngine {
    pub fn new() -> Self {
        Self {
            engine: None,
            model_state: AtomicU8::new(ModelState::NotInitialized as u8),
            current_model: None,
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        if self.is_initialized() {
            return Ok(());
        }

        let config = Config::global();
        let model_name = config.model_name();

        if !model_exists(model_name) {
            return Err(TranscriptionError::ModelLoadingError(
                format!(
                    "Model '{}' not found. Please download it first by running:\n  cargo run -p verify-cli",
                    model_name
                ),
            ));
        }

        let model_path = get_model_cache_path(model_name);
        debug!("[MLEngine] Model '{}' available at: {:?}", model_name, model_path);

        self.model_state.store(ModelState::Initialized as u8, Ordering::Release);
        Ok(())
    }

    pub fn load_model(&mut self, model_type: &str) -> Result<()> {
        if !self.is_initialized() {
            return Err(TranscriptionError::NotInitialized);
        }

        if self.is_model_loaded() && self.current_model.as_deref() == Some(model_type) {
            debug!("[MLEngine] Model {} already loaded", model_type);
            return Ok(());
        }

        if self.is_model_loaded() {
            self.unload_model()?;
        }

        let model_path = get_model_cache_path(model_type);
        let snapshot_path = model_path.join("snapshots").join("main");

        if !snapshot_path.exists() {
            return Err(TranscriptionError::ModelLoadingError(format!(
                "Model directory not found: {}",
                snapshot_path.display()
            )));
        }

        let mut engine = ParakeetEngine::new();
        {
            let _suppress = para_log::SuppressLogs::new();
            engine
                .load_model(&snapshot_path)
                .map_err(|e| TranscriptionError::ModelLoadingError(format!("Failed to load ONNX model: {}", e)))?;
        }

        info!("[MLEngine] ONNX model '{}' loaded successfully", model_type);
        self.engine = Some(ManuallyDrop::new(engine));
        self.current_model = Some(model_type.to_string());
        self.model_state.store(ModelState::ModelLoaded as u8, Ordering::Release);

        Ok(())
    }

    pub fn transcribe(&mut self, audio_data: &[u8]) -> Result<String> {
        if !self.is_model_loaded() {
            return Err(TranscriptionError::ModelNotLoaded);
        }

        let engine = self.engine.as_mut().ok_or(TranscriptionError::ModelNotLoaded)?;

        let wav_bytes = create_wav_bytes(audio_data, crate::ML_SAMPLE_RATE)?;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("transcribe_{}.wav", std::process::id()));

        std::fs::write(&temp_path, wav_bytes)
            .map_err(|e| TranscriptionError::TranscriptionFailed(format!("Failed to write temp WAV: {}", e)))?;

        let result = engine
            .transcribe_file(&temp_path, None)
            .map_err(|e| TranscriptionError::TranscriptionFailed(format!("Transcription failed: {}", e)))?;

        let _ = std::fs::remove_file(&temp_path);

        Ok(result.text)
    }

    pub fn unload_model(&mut self) -> Result<()> {
        if !self.is_model_loaded() {
            return Ok(());
        }

        if let Some(mut engine) = self.engine.take() {
            unsafe {
                ManuallyDrop::drop(&mut engine);
            }
        }
        self.current_model = None;
        self.model_state.store(ModelState::Initialized as u8, Ordering::Release);
        info!("[MLEngine] Model unloaded");
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.model_state.load(Ordering::Acquire) >= ModelState::Initialized as u8
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model_state.load(Ordering::Acquire) == ModelState::ModelLoaded as u8
    }
}

fn create_wav_bytes(pcm_data: &[u8], sample_rate: u32) -> Result<Vec<u8>> {
    use std::io::Cursor;

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, spec)
        .map_err(|e| TranscriptionError::TranscriptionFailed(format!("Failed to create WAV writer: {}", e)))?;

    let samples: Vec<i16> = pcm_data
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    for sample in samples {
        writer
            .write_sample(sample)
            .map_err(|e| TranscriptionError::TranscriptionFailed(format!("Failed to write sample: {}", e)))?;
    }

    writer
        .finalize()
        .map_err(|e| TranscriptionError::TranscriptionFailed(format!("Failed to finalize WAV: {}", e)))?;

    Ok(cursor.into_inner())
}

