use crate::ml_engine_native::MLEngine;
use crate::{ml_error::Result, text_manipulation::handle_transcribed_text};
use config::Config;
use para_log::{debug, error, info, warn};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

static TRANSCRIPTION_SERVICE: OnceLock<Arc<TranscriptionService>> = OnceLock::new();

pub struct TranscriptionService {
    engine: Arc<Mutex<MLEngine>>,
}

impl TranscriptionService {
    fn new() -> Self {
        Self {
            engine: Arc::new(Mutex::new(MLEngine::new())),
        }
    }

    pub fn global() -> Arc<Self> {
        TRANSCRIPTION_SERVICE
            .get_or_init(|| {
                let service = Self::new();
                if let Err(e) = service.initialize() {
                    warn!("Failed to initialize ML engine: {}", e);
                }
                Arc::new(service)
            })
            .clone()
    }

    pub fn initialize(&self) -> Result<()> {
        let mut engine = self.engine.lock()?;
        engine.initialize()
    }

    pub fn transcribe(&self, audio_data: &[u8], sample_rate: u32) -> Result<String> {
        let audio_for_ml = if sample_rate != crate::ML_SAMPLE_RATE {
            debug!(
                "Resampling audio from {}Hz to {}Hz for ML model",
                sample_rate,
                crate::ML_SAMPLE_RATE
            );
            audio::resample_audio(audio_data, sample_rate, crate::ML_SAMPLE_RATE)?
        } else {
            audio_data.to_vec()
        };

        let start_time = Instant::now();
        let result = {
            let mut engine = self.engine.lock()?;
            engine.transcribe(&audio_for_ml)
        };
        let elapsed = start_time.elapsed();

        match result {
            Ok(text) => {
                let processed = handle_transcribed_text(text, Config::global());
                if processed.is_empty() {
                    return Ok(String::new());
                }
                debug!("[ML] Transcription successful: {}", processed);
                info!(
                    "[ML] Transcription completed in {:.2}s, {} chars returned",
                    elapsed.as_secs_f32(),
                    processed.len()
                );
                Ok(processed)
            }
            Err(e) => {
                error!(
                    "[ML] Transcription failed after {:.2}s: {}",
                    elapsed.as_secs_f32(),
                    e
                );
                Err(e)
            }
        }
    }

    pub fn load_model(&self, model_type: Option<&str>) -> Result<String> {
        let config = Config::global();
        let model = model_type.unwrap_or(config.model_name());
        let mut engine = self.engine.lock()?;
        engine.load_model(model)?;
        Ok(format!(
            "parakeet-mlx-{}",
            model.split('/').next_back().unwrap_or(model)
        ))
    }

    pub fn is_ready(&self) -> bool {
        self.engine
            .lock()
            .map(|engine| engine.is_initialized())
            .unwrap_or(false)
    }

    pub fn shutdown_model(&self) -> Result<()> {
        let mut engine = self.engine.lock()?;
        engine.unload_model()?;
        info!("[ML] Model shutdown complete");
        Ok(())
    }
}
