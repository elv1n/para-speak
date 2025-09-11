use crate::error::{AudioError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use hound::WavReader;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
struct PreloadedSound {
    samples: Arc<Vec<f32>>,
    channels: u16,
}

impl PreloadedSound {
    fn load_from_file(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let buf_reader = BufReader::new(file);
        let mut reader = WavReader::new(buf_reader)?;

        let spec = reader.spec();

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => reader
                .samples::<i16>()
                .collect::<std::result::Result<Vec<_>, _>>()?
                .into_iter()
                .map(|sample| sample as f32 / i16::MAX as f32)
                .collect(),
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .collect::<std::result::Result<Vec<_>, _>>()?,
        };

        Ok(PreloadedSound {
            samples: Arc::new(samples),
            channels: spec.channels,
        })
    }
}

static SOUNDS: OnceLock<Option<(PreloadedSound, PreloadedSound, PreloadedSound)>> = OnceLock::new();

pub fn preload_sounds() {
    let start_time = std::time::Instant::now();

    let on_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("sounds")
        .join("on.wav");

    let off_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("sounds")
        .join("off.wav");

    let complete_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("sounds")
        .join("complete.wav");

    let sounds = match (
        PreloadedSound::load_from_file(&on_path),
        PreloadedSound::load_from_file(&off_path),
        PreloadedSound::load_from_file(&complete_path),
    ) {
        (Ok(on_sound), Ok(off_sound), Ok(complete_sound)) => {
            Some((on_sound, off_sound, complete_sound))
        }
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
            log::warn!("Failed to load audio feedback sounds: {}", e);
            None
        }
    };

    if SOUNDS.set(sounds).is_err() {
        log::error!("preload_sounds() was called more than once - this should not happen");
        return;
    }

    let elapsed = start_time.elapsed();
    if let Some(sounds_option) = SOUNDS.get() {
        if sounds_option.is_some() {
            log::debug!(
                "Sound preloading completed in {:.2}ms",
                elapsed.as_secs_f64() * 1000.0
            );
        } else {
            log::warn!(
                "Sound preloading failed in {:.2}ms",
                elapsed.as_secs_f64() * 1000.0
            );
        }
    } else {
        log::error!("Failed to access sound storage after initialization");
    }
}

pub fn play_start_sound() {
    if let Some(Some((ref on_sound, _, _))) = SOUNDS.get() {
        play_async(on_sound.samples.clone(), on_sound.channels);
    }
}

pub fn play_stop_sound() {
    if let Some(Some((_, ref off_sound, _))) = SOUNDS.get() {
        play_async(off_sound.samples.clone(), off_sound.channels);
    }
}

pub fn play_complete_sound() {
    if let Some(Some((_, _, ref complete_sound))) = SOUNDS.get() {
        play_async(complete_sound.samples.clone(), complete_sound.channels);
    }
}

fn play_async(samples: Arc<Vec<f32>>, channels: u16) {
    let result =
        thread::Builder::new()
            .name("sound-player".into())
            .spawn(move || match play_samples(&samples, channels) {
                Ok(()) => {}
                Err(e) => {
                    log::error!("Failed to play sound: {} - retrying once", e);
                    thread::sleep(Duration::from_millis(50));
                    if let Err(e2) = play_samples(&samples, channels) {
                        log::error!("Sound playback retry also failed: {}", e2);
                    }
                }
            });

    if let Err(e) = result {
        log::error!(
            "Failed to spawn sound playback thread: {} - system may be out of resources",
            e
        );
    }
}

fn play_samples(samples: &[f32], wav_channels: u16) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| {
            log::error!("No audio output device available - device may be disconnected or audio subsystem crashed");
            AudioError::NoOutputDevice
        })?;

    let config = device.default_output_config()?;
    let config: StreamConfig = config.into();

    let output_channels = config.channels as usize;
    let wav_channels = wav_channels as usize;

    let samples = Arc::new(samples.to_vec());
    let sample_index = Arc::new(std::sync::Mutex::new(0usize));
    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_count = Arc::new(AtomicU32::new(0));

    let samples_clone = samples.clone();
    let sample_index_clone = sample_index.clone();
    let callback_called_clone = callback_called.clone();
    let callback_count_clone = callback_count.clone();

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            callback_called_clone.store(true, Ordering::Relaxed);
            callback_count_clone.fetch_add(1, Ordering::Relaxed);
            fill_buffer(
                data,
                output_channels,
                &samples_clone,
                wav_channels,
                &sample_index_clone,
            );
        },
        |err| log::error!("Audio playback error: {}", err),
        None,
    )?;

    match stream.play() {
        Ok(()) => {}
        Err(e) => {
            log::error!("Failed to play audio stream: {}", e);
        }
    }

    let sample_count = samples.len();
    let samples_per_second = (config.sample_rate.0 * wav_channels as u32) as usize;
    let duration_ms = (sample_count * 1000) / samples_per_second + 50;

    thread::sleep(Duration::from_millis(100));

    if !callback_called.load(Ordering::Relaxed) {
        log::error!("Audio callback never called after 100ms - stream may have failed silently");
    }

    thread::sleep(Duration::from_millis(duration_ms.saturating_sub(100) as u64));

    let final_callbacks = callback_count.load(Ordering::Relaxed);
    if final_callbacks == 0 {
        log::error!("No audio callbacks were executed - sound did not play");
    } else if final_callbacks < 2 {
        log::warn!(
            "Only {} audio callback(s) executed - sound may have been cut short",
            final_callbacks
        );
    }

    Ok(())
}

fn fill_buffer(
    output: &mut [f32],
    output_channels: usize,
    wav_data: &[f32],
    wav_channels: usize,
    sample_index: &Arc<std::sync::Mutex<usize>>,
) {
    let mut index = match sample_index.lock() {
        Ok(idx) => idx,
        Err(poisoned) => {
            log::warn!("Audio sample index mutex was poisoned, recovering");
            poisoned.into_inner()
        }
    };

    for frame in output.chunks_mut(output_channels) {
        if *index >= wav_data.len() {
            for sample in frame.iter_mut() {
                *sample = 0.0;
            }
            continue;
        }

        if wav_channels == 1 {
            let wav_sample = wav_data[*index];
            for sample in frame.iter_mut() {
                *sample = wav_sample;
            }
            *index += 1;
        } else {
            for (channel_idx, sample) in frame.iter_mut().enumerate() {
                let wav_channel_idx = channel_idx.min(wav_channels - 1);
                let sample_idx = (*index / wav_channels) * wav_channels + wav_channel_idx;

                if sample_idx < wav_data.len() {
                    *sample = wav_data[sample_idx];
                } else {
                    *sample = 0.0;
                }
            }
            *index += wav_channels;
        }
    }
}
