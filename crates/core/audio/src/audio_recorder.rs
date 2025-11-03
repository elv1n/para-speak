use crate::conversion::convert_audio_data;
use crate::dynamic_buffer::DynamicBuffer;
use crate::error::{AudioError, Result};
use crate::ring_buffer::RingBuffer;
use config::Config;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

struct SharedState {
    is_recording: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
}

pub struct AudioData {
    pub samples: Arc<Vec<u8>>,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_ms: u64,
}

enum Command {
    StartRecording,
    StopRecording,
    PauseRecording,
    ResumeRecording,
    GetBufferSnapshot,
    Shutdown,
}

enum Response {
    Started,
    Stopped(AudioData),
    Paused(AudioData),
    Resumed,
    BufferSnapshot(Vec<u8>),
    Error(AudioError),
}

struct InternalState {
    stream: Option<Stream>,
    buffer: Arc<Mutex<DynamicBuffer>>,
    realtime_ring: Option<Arc<RingBuffer>>,
    start_time: Option<Instant>,
    segments: Vec<Vec<u8>>,
    is_paused: bool,
}

impl InternalState {
    fn new(realtime_ring: Option<Arc<RingBuffer>>) -> Self {
        Self {
            stream: None,
            buffer: Arc::new(Mutex::new(DynamicBuffer::new())),
            realtime_ring,
            start_time: None,
            segments: Vec::new(),
            is_paused: false,
        }
    }
}

fn create_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    buffer: Arc<Mutex<DynamicBuffer>>,
    realtime_ring: Option<Arc<RingBuffer>>,
) -> Result<Stream> {
    let channels = config.channels as usize;

    match sample_format {
        SampleFormat::I16 => {
            let buffer_clone = buffer.clone();
            let ring_clone = realtime_ring.clone();
            let stream = device.build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mono_bytes = convert_audio_data(data, channels);
                    if let Ok(mut buffer) = buffer_clone.lock() {
                        buffer.write(&mono_bytes);
                    }
                    if let Some(ring) = &ring_clone {
                        ring.write(&mono_bytes);
                    }
                },
                move |err| {
                    log::error!("Audio input stream error: {}", err);
                },
                None,
            )?;
            Ok(stream)
        }
        SampleFormat::F32 => {
            let buffer_clone = buffer.clone();
            let ring_clone = realtime_ring.clone();
            let stream = device.build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mono_bytes = convert_audio_data(data, channels);
                    if let Ok(mut buffer) = buffer_clone.lock() {
                        buffer.write(&mono_bytes);
                    }
                    if let Some(ring) = &ring_clone {
                        ring.write(&mono_bytes);
                    }
                },
                move |err| {
                    log::error!("Audio input stream error: {}", err);
                },
                None,
            )?;
            Ok(stream)
        }
        format => Err(AudioError::UnsupportedFormat(format)),
    }
}

fn create_stream_with_retry(
    stream_config: &StreamConfig,
    sample_format: SampleFormat,
    buffer: Arc<Mutex<DynamicBuffer>>,
    realtime_ring: Option<Arc<RingBuffer>>,
) -> Result<Stream> {
    const MAX_RETRIES: u8 = 1;
    let host = cpal::default_host();

    for attempt in 1..=MAX_RETRIES {
        let device = host
            .default_input_device()
            .ok_or(AudioError::NoInputDevice)?;

        match create_stream(&device, stream_config, sample_format, buffer.clone(), realtime_ring.clone()) {
            Ok(stream) => {
                if attempt > 1 {
                    log::info!(
                        "Successfully switched to new default input device on attempt {}",
                        attempt
                    );
                }
                return Ok(stream);
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    log::warn!("Audio device unavailable on attempt {}, retrying with new default device: {}", attempt, e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }

                log::error!(
                    "Failed to create audio stream after {} attempts: {}",
                    attempt,
                    e
                );
                return Err(e);
            }
        }
    }

    Err(AudioError::StateError(
        "Maximum retry attempts exceeded".into(),
    ))
}

pub struct AudioRecorder {
    command_tx: Sender<Command>,
    response_rx: Receiver<Response>,
    worker_thread: Option<JoinHandle<()>>,
    shared_state: SharedState,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self::with_realtime_ring(None)
    }

    pub fn with_realtime_ring(realtime_ring: Option<Arc<RingBuffer>>) -> Self {
        let (command_tx, command_rx) = bounded::<Command>(10);
        let (response_tx, response_rx) = bounded::<Response>(10);

        let shared_state = SharedState {
            is_recording: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
        };

        let shared_state_worker = SharedState {
            is_recording: shared_state.is_recording.clone(),
            is_paused: shared_state.is_paused.clone(),
        };

        let worker_thread = thread::spawn(move || {
            if let Err(e) = run_audio_thread(command_rx, response_tx, shared_state_worker, realtime_ring) {
                log::error!("Audio recorder thread failed: {}", e);
            }
        });

        Self {
            command_tx,
            response_rx,
            worker_thread: Some(worker_thread),
            shared_state,
        }
    }

    pub fn start_recording(&self) -> Result<()> {
        self.command_tx
            .send(Command::StartRecording)
            .map_err(|_| AudioError::ThreadCommunicationFailed)?;

        match self.response_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Response::Started) => Ok(()),
            Ok(Response::Error(e)) => Err(e),
            Ok(_) => Err(AudioError::StateError("Unexpected response".into())),
            Err(_) => Err(AudioError::ThreadCommunicationFailed),
        }
    }

    pub fn stop_recording(&self) -> Result<AudioData> {
        self.command_tx
            .send(Command::StopRecording)
            .map_err(|_| AudioError::ThreadCommunicationFailed)?;

        match self.response_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Response::Stopped(data)) => Ok(data),
            Ok(Response::Error(e)) => Err(e),
            Ok(_) => Err(AudioError::StateError("Unexpected response".into())),
            Err(_) => Err(AudioError::ThreadCommunicationFailed),
        }
    }

    pub fn pause_recording(&self) -> Result<AudioData> {
        self.command_tx
            .send(Command::PauseRecording)
            .map_err(|_| AudioError::ThreadCommunicationFailed)?;

        match self.response_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Response::Paused(data)) => Ok(data),
            Ok(Response::Error(e)) => Err(e),
            Ok(_) => Err(AudioError::StateError("Unexpected response".into())),
            Err(_) => Err(AudioError::ThreadCommunicationFailed),
        }
    }

    pub fn resume_recording(&self) -> Result<()> {
        self.command_tx
            .send(Command::ResumeRecording)
            .map_err(|_| AudioError::ThreadCommunicationFailed)?;

        match self.response_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Response::Resumed) => Ok(()),
            Ok(Response::Error(e)) => Err(e),
            Ok(_) => Err(AudioError::StateError("Unexpected response".into())),
            Err(_) => Err(AudioError::ThreadCommunicationFailed),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.shared_state.is_recording.load(Ordering::Relaxed)
    }

    pub fn is_paused(&self) -> bool {
        self.shared_state.is_paused.load(Ordering::Relaxed)
    }

    pub fn get_buffer_snapshot(&self) -> Result<Vec<u8>> {
        self.command_tx
            .send(Command::GetBufferSnapshot)
            .map_err(|_| AudioError::ThreadCommunicationFailed)?;

        match self.response_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Response::BufferSnapshot(data)) => Ok(data),
            Ok(Response::Error(e)) => Err(e),
            Ok(_) => Err(AudioError::StateError("Unexpected response".into())),
            Err(_) => Err(AudioError::ThreadCommunicationFailed),
        }
    }

    pub fn shutdown(&mut self) -> Result<()> {
        if self.is_recording() {
            let _ = self.stop_recording();
        }

        self.command_tx
            .send(Command::Shutdown)
            .map_err(|_| AudioError::ThreadCommunicationFailed)?;

        if let Some(thread) = self.worker_thread.take() {
            thread
                .join()
                .map_err(|_| AudioError::ThreadCommunicationFailed)?;
        }

        Ok(())
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        let _ = self.command_tx.send(Command::Shutdown);
        if let Some(thread) = self.worker_thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_audio_thread(
    command_rx: Receiver<Command>,
    response_tx: Sender<Response>,
    shared_state: SharedState,
    realtime_ring: Option<Arc<RingBuffer>>,
) -> Result<()> {
    let config = Config::global();
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AudioError::NoInputDevice)?;

    let default_config = device.default_input_config()?;
    let stream_config = StreamConfig {
        channels: 1, // Force mono
        sample_rate: cpal::SampleRate(config.sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let mut state = InternalState::new(realtime_ring);

    while let Ok(cmd) = command_rx.recv() {
        match handle_command(
            cmd,
            &mut state,
            &stream_config,
            default_config.sample_format(),
            &shared_state,
        ) {
            Ok(Some(response)) => {
                if response_tx.send(response).is_err() {
                    break; // Main thread dropped
                }
            }
            Ok(None) => break, // Shutdown
            Err(e) => {
                if response_tx.send(Response::Error(e)).is_err() {
                    break; // Main thread dropped
                }
            }
        }
    }

    Ok(())
}

fn handle_command(
    cmd: Command,
    state: &mut InternalState,
    stream_config: &StreamConfig,
    sample_format: SampleFormat,
    shared_state: &SharedState,
) -> Result<Option<Response>> {
    let config = Config::global();
    match cmd {
        Command::StartRecording => {
            if state.stream.is_some() {
                return Ok(Some(Response::Error(AudioError::AlreadyRecording)));
            }

            state.buffer.lock().unwrap().reset();
            state.segments.clear();

            log::debug!("Starting new recording, buffer cleared");

            let stream =
                create_stream_with_retry(stream_config, sample_format, state.buffer.clone(), state.realtime_ring.clone())?;
            stream.play()?;

            state.stream = Some(stream);
            state.start_time = Some(Instant::now());
            state.is_paused = false;

            shared_state.is_recording.store(true, Ordering::Relaxed);
            shared_state.is_paused.store(false, Ordering::Relaxed);

            Ok(Some(Response::Started))
        }
        Command::StopRecording => {
            if state.stream.is_none() && !state.is_paused {
                return Ok(Some(Response::Error(AudioError::NotRecording)));
            }

            // Add delay to capture last bits of audio
            std::thread::sleep(std::time::Duration::from_millis(500));

            let current_data = if state.stream.is_some() {
                state.buffer.lock().unwrap().read_all()
            } else {
                Vec::new()
            };

            let mut all_samples = Vec::new();
            for segment in &state.segments {
                all_samples.extend_from_slice(segment);
            }
            all_samples.extend(current_data);

            log::debug!(
                "Stopping recording, total audio: {} bytes ({:.2} seconds)",
                all_samples.len(),
                all_samples.len() as f64 / (config.sample_rate as f64 * 2.0)
            );

            let duration_ms = state
                .start_time
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0);

            state.stream = None;
            state.start_time = None;
            state.segments.clear();
            state.is_paused = false;

            state.buffer.lock().unwrap().reset();

            shared_state.is_recording.store(false, Ordering::Relaxed);
            shared_state.is_paused.store(false, Ordering::Relaxed);

            let audio_data = AudioData {
                samples: Arc::new(all_samples),
                sample_rate: config.sample_rate,
                channels: 1,
                duration_ms,
            };

            Ok(Some(Response::Stopped(audio_data)))
        }
        Command::PauseRecording => {
            if state.stream.is_none() {
                return Ok(Some(Response::Error(AudioError::NotRecording)));
            }
            if state.is_paused {
                return Ok(Some(Response::Error(AudioError::AlreadyPaused)));
            }

            let current_data = state.buffer.lock().unwrap().read_all();
            let duration_ms = state
                .start_time
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0);

            state.segments.push(current_data.clone());
            state.stream = None;
            state.is_paused = true;

            shared_state.is_recording.store(false, Ordering::Relaxed);
            shared_state.is_paused.store(true, Ordering::Relaxed);

            log::debug!(
                "Paused recording, saved {} bytes to segments",
                current_data.len()
            );

            let audio_data = AudioData {
                samples: Arc::new(current_data),
                sample_rate: config.sample_rate,
                channels: 1,
                duration_ms,
            };

            Ok(Some(Response::Paused(audio_data)))
        }
        Command::ResumeRecording => {
            if !state.is_paused {
                return Ok(Some(Response::Error(AudioError::NotPaused)));
            }

            log::debug!("Resuming recording, continuing with same buffer");

            let stream =
                create_stream_with_retry(stream_config, sample_format, state.buffer.clone(), state.realtime_ring.clone())?;
            stream.play()?;

            state.stream = Some(stream);
            state.is_paused = false;

            shared_state.is_recording.store(true, Ordering::Relaxed);
            shared_state.is_paused.store(false, Ordering::Relaxed);

            Ok(Some(Response::Resumed))
        }
        Command::GetBufferSnapshot => {
            let data = if state.stream.is_some() {
                state.buffer.lock().unwrap().get_data().to_vec()
            } else {
                Vec::new()
            };
            Ok(Some(Response::BufferSnapshot(data)))
        }
        Command::Shutdown => {
            shared_state.is_recording.store(false, Ordering::Relaxed);
            shared_state.is_paused.store(false, Ordering::Relaxed);
            Ok(None)
        }
    }
}
