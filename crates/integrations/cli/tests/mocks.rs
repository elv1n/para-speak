use audio::AudioData;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MockAudioRecorder {
    pub is_recording: Arc<AtomicBool>,
    pub is_paused: Arc<AtomicBool>,
    pub start_count: Arc<AtomicUsize>,
    pub stop_count: Arc<AtomicUsize>,
    pub pause_count: Arc<AtomicUsize>,
    pub resume_count: Arc<AtomicUsize>,
    mock_audio_data: Arc<Vec<u8>>,
}

impl Default for MockAudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAudioRecorder {
    pub fn new() -> Self {
        let mock_pcm_data = vec![0u8; 16000];

        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
            start_count: Arc::new(AtomicUsize::new(0)),
            stop_count: Arc::new(AtomicUsize::new(0)),
            pause_count: Arc::new(AtomicUsize::new(0)),
            resume_count: Arc::new(AtomicUsize::new(0)),
            mock_audio_data: Arc::new(mock_pcm_data),
        }
    }

    pub fn start_recording(&self) -> anyhow::Result<()> {
        self.is_recording.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        self.start_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn stop_recording(&self) -> anyhow::Result<AudioData> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        self.stop_count.fetch_add(1, Ordering::SeqCst);

        Ok(AudioData {
            samples: self.mock_audio_data.clone(),
            sample_rate: 16000,
            channels: 1,
            duration_ms: 1000,
        })
    }

    pub fn pause_recording(&self) -> anyhow::Result<AudioData> {
        self.is_paused.store(true, Ordering::SeqCst);
        self.pause_count.fetch_add(1, Ordering::SeqCst);

        Ok(AudioData {
            samples: Arc::new(self.mock_audio_data.as_ref()[0..8000].to_vec()),
            sample_rate: 16000,
            channels: 1,
            duration_ms: 500,
        })
    }

    pub fn resume_recording(&self) -> anyhow::Result<()> {
        self.is_paused.store(false, Ordering::SeqCst);
        self.resume_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Clone)]
pub struct MockTranscriptionService {
    pub transcribe_count: Arc<AtomicUsize>,
    pub last_transcription: Arc<Mutex<Option<String>>>,
}

impl Default for MockTranscriptionService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTranscriptionService {
    pub fn new() -> Self {
        Self {
            transcribe_count: Arc::new(AtomicUsize::new(0)),
            last_transcription: Arc::new(Mutex::new(None)),
        }
    }

    pub fn transcribe(&self, _audio_data: &[u8]) -> anyhow::Result<String> {
        self.transcribe_count.fetch_add(1, Ordering::SeqCst);
        let text = format!(
            "Transcribed text {}",
            self.transcribe_count.load(Ordering::SeqCst)
        );

        if let Ok(mut last) = self.last_transcription.lock() {
            *last = Some(text.clone());
        }

        Ok(text)
    }
}

#[derive(Clone)]
pub struct MockComponentRegistry {
    pub start_count: Arc<AtomicUsize>,
    pub stop_count: Arc<AtomicUsize>,
    pub cancel_count: Arc<AtomicUsize>,
    pub pause_count: Arc<AtomicUsize>,
    pub resume_count: Arc<AtomicUsize>,
    pub processing_start_count: Arc<AtomicUsize>,
    pub processing_complete_count: Arc<AtomicUsize>,
    pub last_transcription: Arc<Mutex<Option<String>>>,
}

impl Default for MockComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MockComponentRegistry {
    pub fn new() -> Self {
        Self {
            start_count: Arc::new(AtomicUsize::new(0)),
            stop_count: Arc::new(AtomicUsize::new(0)),
            cancel_count: Arc::new(AtomicUsize::new(0)),
            pause_count: Arc::new(AtomicUsize::new(0)),
            resume_count: Arc::new(AtomicUsize::new(0)),
            processing_start_count: Arc::new(AtomicUsize::new(0)),
            processing_complete_count: Arc::new(AtomicUsize::new(0)),
            last_transcription: Arc::new(Mutex::new(None)),
        }
    }

    pub fn notify_start(&self) -> anyhow::Result<()> {
        self.start_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn notify_stop(&self) -> anyhow::Result<()> {
        self.stop_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn notify_cancel(&self) -> anyhow::Result<()> {
        self.cancel_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn notify_pause(&self) -> anyhow::Result<()> {
        self.pause_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn notify_resume(&self) -> anyhow::Result<()> {
        self.resume_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn notify_processing_start(&self) -> anyhow::Result<()> {
        self.processing_start_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn notify_processing_complete(&self, text: &str) -> anyhow::Result<()> {
        self.processing_complete_count
            .fetch_add(1, Ordering::SeqCst);
        if let Ok(mut last) = self.last_transcription.lock() {
            *last = Some(text.to_string());
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn notify_error(&self, _error: &str) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(dead_code)]
    pub fn notify_partial_processing_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(dead_code)]
    pub fn notify_partial_processing_complete(&self, _text: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct MockController {
    pub audio: Arc<Mutex<MockAudioRecorder>>,
    pub registry: Arc<MockComponentRegistry>,
    pub transcription: Arc<MockTranscriptionService>,

    // Track which methods were called
    pub handle_start_count: Arc<AtomicUsize>,
    pub handle_stop_count: Arc<AtomicUsize>,
    pub handle_pause_count: Arc<AtomicUsize>,
    pub handle_cancel_count: Arc<AtomicUsize>,
}

impl Default for MockController {
    fn default() -> Self {
        Self::new()
    }
}

impl MockController {
    pub fn new() -> Self {
        Self {
            audio: Arc::new(Mutex::new(MockAudioRecorder::new())),
            registry: Arc::new(MockComponentRegistry::new()),
            transcription: Arc::new(MockTranscriptionService::new()),
            handle_start_count: Arc::new(AtomicUsize::new(0)),
            handle_stop_count: Arc::new(AtomicUsize::new(0)),
            handle_pause_count: Arc::new(AtomicUsize::new(0)),
            handle_cancel_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn handle_start(&self) -> anyhow::Result<()> {
        self.handle_start_count.fetch_add(1, Ordering::SeqCst);

        {
            let audio = self.audio.lock().unwrap();
            audio.start_recording()?;
        }

        self.registry.notify_start()?;
        Ok(())
    }

    fn handle_stop(&self) -> anyhow::Result<()> {
        self.handle_stop_count.fetch_add(1, Ordering::SeqCst);

        let audio_data = {
            let audio = self.audio.lock().unwrap();
            audio.stop_recording()?
        };

        self.registry.notify_stop()?;

        let registry = self.registry.clone();
        let transcription = self.transcription.clone();

        std::thread::spawn(move || {
            let _ = registry.notify_processing_start();

            if let Ok(text) = transcription.transcribe(&audio_data.samples) {
                let _ = registry.notify_processing_complete(&text);
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(50));

        Ok(())
    }

    fn handle_pause(&self) -> anyhow::Result<()> {
        self.handle_pause_count.fetch_add(1, Ordering::SeqCst);

        let audio = self.audio.lock().unwrap();

        if audio.is_paused() {
            // Resume
            audio.resume_recording()?;
            self.registry.notify_resume()?;
        } else {
            // Pause
            let _audio_data = audio.pause_recording()?;
            self.registry.notify_pause()?;
        }

        Ok(())
    }

    fn handle_cancel(&self) -> anyhow::Result<()> {
        self.handle_cancel_count.fetch_add(1, Ordering::SeqCst);

        if let Ok(audio) = self.audio.try_lock() {
            let _ = audio.stop_recording();
        }

        self.registry.notify_cancel()?;
        Ok(())
    }
}

impl shortcut_matcher::ShortcutHandler for MockController {
    fn handle_action(&self, action: shortcut_matcher::ShortcutAction) {
        match action {
            shortcut_matcher::ShortcutAction::Start => {
                self.handle_start().ok();
            }
            shortcut_matcher::ShortcutAction::Stop => {
                self.handle_stop().ok();
            }
            shortcut_matcher::ShortcutAction::Cancel => {
                self.handle_cancel().ok();
            }
            shortcut_matcher::ShortcutAction::Pause => {
                self.handle_pause().ok();
            }
        }
    }

    fn handle_error(&self, error: String) {
        eprintln!("Shortcut error: {}", error);
    }
}
