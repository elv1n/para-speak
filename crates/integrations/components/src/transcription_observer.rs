use crate::{Component, ExecutionMode, SmartCollector};
use anyhow::Result;
use audio::RingBuffer;
use config::Config;
use ml_core::TranscriptionService;
use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct TranscriptionObserver {
    ring_buffer: Arc<RingBuffer>,
    running: Arc<AtomicBool>,
    collector: Arc<Mutex<SmartCollector>>,
    thread_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl std::fmt::Debug for TranscriptionObserver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscriptionObserver")
            .field("running", &self.running)
            .finish()
    }
}

impl TranscriptionObserver {
    pub fn new(ring_buffer: Arc<RingBuffer>) -> Self {
        Self {
            ring_buffer,
            running: Arc::new(AtomicBool::new(false)),
            collector: Arc::new(Mutex::new(SmartCollector::new())),
            thread_handle: Arc::new(Mutex::new(None)),
        }
    }

    fn should_run() -> bool {
        Config::global().realtime
    }

    fn handle_transcription(accumulated_audio: Vec<u8>) {
        let bytes_per_second = Config::global().sample_rate as f64 * 2.0;
        let duration_sec = accumulated_audio.len() as f64 / bytes_per_second;
        log::info!(
            "[RealTime] Transcription triggered: {} bytes ({:.2}s)",
            accumulated_audio.len(),
            duration_sec
        );

        let audio_arc = Arc::new(accumulated_audio);
        let transcription_start = std::time::Instant::now();

        match TranscriptionService::global().transcribe(&audio_arc) {
            Ok(text) => {
                let elapsed = transcription_start.elapsed();
                if !text.trim().is_empty() {
                    log::info!(
                        "[RealTime] Result ({:.2}s, {} chars): {}",
                        elapsed.as_secs_f32(),
                        text.trim().len(),
                        text.trim()
                    );
                } else {
                    log::info!("[RealTime] Empty transcription");
                }
            }
            Err(e) => {
                log::error!("[RealTime] Transcription error: {}", e);
            }
        }
    }
}

impl Component for TranscriptionObserver {
    fn name(&self) -> &str {
        "TranscriptionObserver"
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }

    fn on_start(&self) -> Result<()> {
        if !Self::should_run() {
            return Ok(());
        }

        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        self.ring_buffer.reset_reader();

        let ring_buffer = self.ring_buffer.clone();
        let running = self.running.clone();
        let collector = self.collector.clone();

        let handle = thread::spawn(move || {
            log::info!("[RealTime] Starting real-time observer");

            const MIN_CHUNK_SIZE: usize = 38400;
            const MAX_CHUNK_SIZE: usize = 76800;
            const POLL_INTERVAL_MS: u64 = 100;

            while running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));

                if !running.load(Ordering::SeqCst) {
                    break;
                }

                let available = ring_buffer.available_bytes();

                if available >= MIN_CHUNK_SIZE {
                    let chunk_size = available.min(MAX_CHUNK_SIZE);

                    if let Some(chunk) = ring_buffer.read_chunk(chunk_size) {
                        match collector.lock() {
                            Ok(mut collector) => {
                                if let Some(accumulated_audio) = collector.process_chunk(&chunk) {
                                    Self::handle_transcription(accumulated_audio);
                                }
                            }
                            Err(e) => {
                                log::error!("[TranscriptionObserver] Failed to lock collector: {}", e);
                            }
                        }
                    }
                }
            }

            log::info!("[RealTime] Stopping real-time observer");
        });

        if let Ok(mut thread_handle_lock) = self.thread_handle.lock() {
            *thread_handle_lock = Some(handle);
        }

        Ok(())
    }

    fn on_stop(&self) -> Result<()> {
        if !Self::should_run() {
            return Ok(());
        }

        self.running.store(false, Ordering::SeqCst);

        if let Ok(mut thread_handle_lock) = self.thread_handle.lock() {
            if let Some(handle) = thread_handle_lock.take() {
                log::debug!("[TranscriptionObserver] Waiting for thread to finish");
                if let Err(e) = handle.join() {
                    log::error!("[TranscriptionObserver] Thread panicked: {:?}", e);
                }
            }
        }

        if let Ok(mut collector) = self.collector.lock() {
            if let Some(final_audio) = collector.extract_final_segment() {
                Self::handle_transcription(final_audio);
            }
            collector.reset();
        }

        Ok(())
    }

    fn on_cancel(&self) -> Result<()> {
        if !Self::should_run() {
            return Ok(());
        }

        self.running.store(false, Ordering::SeqCst);

        if let Ok(mut thread_handle_lock) = self.thread_handle.lock() {
            if let Some(handle) = thread_handle_lock.take() {
                log::debug!("[TranscriptionObserver] Waiting for thread to finish");
                if let Err(e) = handle.join() {
                    log::error!("[TranscriptionObserver] Thread panicked: {:?}", e);
                }
            }
        }

        if let Ok(mut collector) = self.collector.lock() {
            collector.reset();
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
