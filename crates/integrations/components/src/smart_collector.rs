use config::Config;
use std::collections::VecDeque;
use std::time::Instant;

const SPEECH_THRESHOLD: f32 = 0.04;
const PRE_SPEECH_CONTEXT_SEC: f64 = 1.0;
const POST_SPEECH_CONTEXT_SEC: f64 = 2.0;

pub struct SmartCollector {
    pre_speech_buffer: VecDeque<Vec<u8>>,
    pre_speech_buffer_max_bytes: usize,
    accumulated_speech: Vec<u8>,
    post_speech_buffer: VecDeque<Vec<u8>>,
    post_speech_buffer_max_bytes: usize,
    speech_detected: bool,
    silence_start: Option<Instant>,
}

impl Default for SmartCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartCollector {
    pub fn new() -> Self {
        let bytes_per_second = (Config::global().sample_rate * 2) as usize;
        let pre_speech_buffer_max_bytes = (bytes_per_second as f64 * PRE_SPEECH_CONTEXT_SEC) as usize;
        let post_speech_buffer_max_bytes = (bytes_per_second as f64 * POST_SPEECH_CONTEXT_SEC) as usize;

        Self {
            pre_speech_buffer: VecDeque::new(),
            pre_speech_buffer_max_bytes,
            accumulated_speech: Vec::new(),
            post_speech_buffer: VecDeque::new(),
            post_speech_buffer_max_bytes,
            speech_detected: false,
            silence_start: None,
        }
    }

    pub fn process_chunk(&mut self, audio_data: &[u8]) -> Option<Vec<u8>> {
        let rms = calculate_rms(audio_data);
        let is_speech = rms >= SPEECH_THRESHOLD;

        if !self.speech_detected {
            if is_speech {
                self.speech_detected = true;
                self.accumulated_speech.extend_from_slice(audio_data);
                log::info!(
                    "[SmartCollector] Speech detected, including {:.2}s of pre-speech context",
                    self.get_pre_speech_duration()
                );
            } else {
                self.pre_speech_buffer.push_back(audio_data.to_vec());

                while !self.pre_speech_buffer.is_empty() {
                    let current_size: usize = self.pre_speech_buffer.iter().map(|c| c.len()).sum();
                    if current_size <= self.pre_speech_buffer_max_bytes {
                        break;
                    }
                    self.pre_speech_buffer.pop_front();
                }
            }
        } else if is_speech {
            if self.silence_start.is_some() {
                self.post_speech_buffer.clear();
                self.silence_start = None;
            }
            self.accumulated_speech.extend_from_slice(audio_data);
        } else {
            if self.silence_start.is_none() {
                self.silence_start = Some(Instant::now());
            }

            self.post_speech_buffer.push_back(audio_data.to_vec());
            let post_speech_bytes: usize = self.post_speech_buffer.iter().map(|c| c.len()).sum();

            if post_speech_bytes >= self.post_speech_buffer_max_bytes {
                let bytes_per_second = Config::global().sample_rate as f64 * 2.0;

                let pre_bytes: usize = self.pre_speech_buffer.iter().map(|c| c.len()).sum();
                let speech_bytes = self.accumulated_speech.len();
                let post_bytes: usize = self.post_speech_buffer.iter().map(|c| c.len()).sum();
                let total_bytes = pre_bytes + speech_bytes + post_bytes;

                log::info!(
                    "[SmartCollector] Speech segment complete: {:.2}s total ({:.2}s pre + {:.2}s speech + {:.2}s post)",
                    total_bytes as f64 / bytes_per_second,
                    pre_bytes as f64 / bytes_per_second,
                    speech_bytes as f64 / bytes_per_second,
                    post_bytes as f64 / bytes_per_second,
                );

                return self.extract_segment();
            }
        }

        None
    }

    pub fn reset(&mut self) {
        self.pre_speech_buffer.clear();
        self.accumulated_speech.clear();
        self.post_speech_buffer.clear();
        self.speech_detected = false;
        self.silence_start = None;
    }

    fn get_pre_speech_duration(&self) -> f64 {
        let bytes: usize = self.pre_speech_buffer.iter().map(|c| c.len()).sum();
        let bytes_per_second = Config::global().sample_rate as f64 * 2.0;
        bytes as f64 / bytes_per_second
    }

    pub fn extract_final_segment(&mut self) -> Option<Vec<u8>> {
        if self.accumulated_speech.is_empty() {
            return None;
        }

        let mut final_audio = Vec::new();

        for chunk in &self.pre_speech_buffer {
            final_audio.extend_from_slice(chunk);
        }

        final_audio.extend_from_slice(&self.accumulated_speech);

        for chunk in &self.post_speech_buffer {
            final_audio.extend_from_slice(chunk);
        }

        self.pre_speech_buffer.clear();
        self.accumulated_speech.clear();
        self.post_speech_buffer.clear();
        self.speech_detected = false;
        self.silence_start = None;

        Some(final_audio)
    }

    fn extract_segment(&mut self) -> Option<Vec<u8>> {
        self.extract_final_segment()
    }
}

fn calculate_rms(audio_bytes: &[u8]) -> f32 {
    if audio_bytes.len() < 2 {
        return 0.0;
    }

    let sum: f64 = audio_bytes
        .chunks_exact(2)
        .map(|chunk| {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0;
            (sample * sample) as f64
        })
        .sum();

    let count = audio_bytes.len() / 2;
    if count == 0 {
        return 0.0;
    }

    (sum / count as f64).sqrt() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_speech_chunk(value: u8, size: usize) -> Vec<u8> {
        vec![value; size]
    }

    fn create_silence_chunk(size: usize) -> Vec<u8> {
        vec![0; size]
    }

    #[test]
    fn test_pre_speech_buffer_rotation() {
        let mut collector = SmartCollector::new();

        let chunk_size = 10000;
        for _ in 0..20 {
            collector.process_chunk(&create_silence_chunk(chunk_size));
        }

        let pre_bytes: usize = collector.pre_speech_buffer.iter().map(|c| c.len()).sum();
        assert!(pre_bytes <= collector.pre_speech_buffer_max_bytes);
    }

    #[test]
    fn test_speech_detection_includes_pre_context() {
        let mut collector = SmartCollector::new();

        collector.process_chunk(&create_silence_chunk(10000));
        collector.process_chunk(&create_silence_chunk(10000));

        let result = collector.process_chunk(&create_speech_chunk(200, 10000));
        assert!(result.is_none());
        assert!(collector.speech_detected);
        assert!(!collector.pre_speech_buffer.is_empty() || !collector.accumulated_speech.is_empty());
    }

    #[test]
    fn test_post_speech_context_collection() {
        let mut collector = SmartCollector::new();

        collector.process_chunk(&create_speech_chunk(200, 10000));
        assert!(collector.speech_detected);

        for _ in 0..30 {
            if let Some(_result) = collector.process_chunk(&create_silence_chunk(10000)) {
                break;
            }
        }

        assert!(collector.post_speech_buffer.is_empty() || !collector.speech_detected);
    }

    #[test]
    fn test_interrupted_silence() {
        let mut collector = SmartCollector::new();

        collector.process_chunk(&create_speech_chunk(200, 10000));
        collector.process_chunk(&create_silence_chunk(10000));
        collector.process_chunk(&create_speech_chunk(200, 10000));

        assert!(collector.post_speech_buffer.is_empty());
        assert!(collector.accumulated_speech.len() > 10000);
    }
}
