use crate::registry::{create_default_registry, ComponentRegistry};
use audio::{play_error_sound, AudioRecorder};
use config::Config;
use indicatif::{ProgressBar, ProgressStyle};
use ml_core::TranscriptionService;
use shortcut_matcher::ShortcutAction;
use shortcut_matcher::ShortcutHandler;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct Controllers {
    audio: Arc<Mutex<AudioRecorder>>,
    registry: Arc<ComponentRegistry>,
}

impl Controllers {
    pub fn new() -> anyhow::Result<Self> {
        let config = Config::global();

        let ring_buffer = if config.realtime {
            Some(Arc::new(audio::RingBuffer::new(10, config.sample_rate)))
        } else {
            None
        };

        let audio = Arc::new(Mutex::new(audio::AudioRecorder::with_realtime_ring(ring_buffer.clone())));

        let registry = create_default_registry(ring_buffer)
            .map_err(|e| anyhow::anyhow!("Failed to create component registry: {}", e))?
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build component registry: {}", e))?;

        let config = Config::global();
        let model_type = config.model_name();

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Loading model {}...", model_type));
        pb.enable_steady_tick(Duration::from_millis(100));

        let model_init_start = Instant::now();
        let model_name = TranscriptionService::global()
            .load_model(None)
            .map_err(|e| {
                pb.finish_and_clear();
                anyhow::anyhow!("{}", e)
            })?;
        let model_init_duration = model_init_start.elapsed();

        pb.finish_with_message(format!("✅ Model {} loaded in {:.2}s", model_name, model_init_duration.as_secs_f32()));

        log::info!(
            "Model {} initialized in {:.2}s",
            model_name,
            model_init_duration.as_secs_f32()
        );

        Ok(Self {
            audio,
            registry,
        })
    }

    pub fn handle_start(&self) -> anyhow::Result<()> {
        {
            let audio = self
                .audio
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock audio: {}", e))?;
            if let Err(e) = audio.start_recording() {
                play_error_sound();
                return Err(e.into());
            }
        }

        let _ = self.registry.notify_start();
        Ok(())
    }

    pub fn handle_stop(&self) -> anyhow::Result<()> {
        let audio_data = {
            let audio = self
                .audio
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock audio: {}", e))?;
            audio.stop_recording()?.samples
        };

        let _ = self.registry.notify_stop();

        let registry = self.registry.clone();

        thread::spawn(move || {
            Self::process_transcription(audio_data, registry);
        });

        Ok(())
    }

    pub fn handle_cancel(&self) -> anyhow::Result<()> {
        if let Ok(audio) = self.audio.try_lock() {
            let _ = audio.stop_recording();
        }

        let _ = self.registry.notify_cancel();
        Ok(())
    }

    pub fn handle_pause(&self) -> anyhow::Result<()> {
        let audio_data = {
            let audio = self
                .audio
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock audio: {}", e))?;
            audio.pause_recording()?
        };

        if Config::global().transcribe_on_pause {
            self.process_partial_transcription(audio_data.samples)?;
        }

        let _ = self.registry.notify_pause();
        Ok(())
    }

    pub fn handle_resume(&self) -> anyhow::Result<()> {
        {
            let audio = self
                .audio
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock audio: {}", e))?;
            if let Err(e) = audio.resume_recording() {
                play_error_sound();
                return Err(e.into());
            }
        }

        log::debug!("Resuming transcription");
        let _ = self.registry.notify_resume();
        Ok(())
    }

    pub fn shutdown(&self) -> anyhow::Result<()> {
        {
            let audio = self
                .audio
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock audio: {}", e))?;

            if audio.is_recording() {
                drop(audio);
                let _ = self.registry.notify_cancel();
            }
        }

        {
            let mut audio = self
                .audio
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock audio: {}", e))?;
            audio.shutdown()?;
        }

        // Ensure ML model is unloaded to avoid native aborts during process teardown
        // This is best-effort during shutdown - errors are logged but not fatal
        match ml_core::TranscriptionService::global().shutdown_model() {
            Ok(_) => log::info!("ML model shutdown completed successfully"),
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("KeyboardInterrupt") {
                    log::info!(
                        "ML model shutdown interrupted by signal (expected during shutdown)"
                    );
                } else {
                    log::warn!("ML model shutdown encountered non-fatal error: {}", e);
                }
            }
        }

        Ok(())
    }

    fn process_transcription(audio_data: Arc<Vec<u8>>, registry: Arc<ComponentRegistry>) {
        let _ = registry.notify_processing_start();

        match TranscriptionService::global().transcribe(&audio_data) {
            Ok(text) => {
                let _ = registry.notify_processing_complete(&text);
            }
            Err(e) => {
                let error_msg = e.to_string();
                let _ = registry.notify_error(&error_msg);
            }
        }
    }

    pub fn process_partial_transcription(&self, audio_data: Arc<Vec<u8>>) -> anyhow::Result<()> {
        if !Config::global().transcribe_on_pause {
            return Ok(());
        }

        let registry = self.registry.clone();

        thread::spawn(move || {
            let _ = registry.notify_partial_processing_start();

            match TranscriptionService::global().transcribe(&audio_data) {
                Ok(text) => {
                    let _ = registry.notify_partial_processing_complete(&text);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    let _ = registry.notify_error(&error_msg);
                }
            }
        });

        Ok(())
    }
}

impl ShortcutHandler for Controllers {
    fn handle_action(&self, action: ShortcutAction) {
        match action {
            ShortcutAction::Start => {
                if let Err(e) = self.handle_start() {
                    log::error!("Failed to handle start action: {}", e);
                }
            }
            ShortcutAction::Stop => {
                if let Err(e) = self.handle_stop() {
                    log::error!("Failed to handle stop action: {}", e);
                }
            }
            ShortcutAction::Cancel => {
                if let Err(e) = self.handle_cancel() {
                    log::error!("Failed to handle cancel action: {}", e);
                }
            }
            ShortcutAction::Pause => {
                if let Ok(audio) = self.audio.try_lock() {
                    log::debug!(
                        "Pausing transcription what will happen now? {}",
                        audio.is_paused()
                    );
                    if audio.is_paused() {
                        drop(audio);
                        if let Err(e) = self.handle_resume() {
                            log::error!("Failed to handle resume action: {}", e);
                        }
                    } else {
                        drop(audio);
                        if let Err(e) = self.handle_pause() {
                            log::error!("Failed to handle pause action: {}", e);
                        }
                    }
                }
            }
        }
    }

    fn handle_error(&self, error: String) {
        log::error!("Shortcut error: {}", error);
    }
}
