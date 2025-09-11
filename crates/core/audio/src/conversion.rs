pub trait AudioSample {
    fn to_i16_mono(data: &[Self], channels: usize) -> Vec<u8>
    where
        Self: Sized;
}

impl AudioSample for i16 {
    fn to_i16_mono(data: &[i16], channels: usize) -> Vec<u8> {
        if channels == 1 {
            let mut bytes = Vec::with_capacity(data.len() * 2);
            for &sample in data {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
            bytes
        } else {
            let samples_count = data.len() / channels;
            let mut bytes = Vec::with_capacity(samples_count * 2);

            for chunk in data.chunks_exact(channels) {
                bytes.extend_from_slice(&chunk[0].to_le_bytes());
            }
            bytes
        }
    }
}

impl AudioSample for f32 {
    fn to_i16_mono(data: &[f32], channels: usize) -> Vec<u8> {
        if channels == 1 {
            let mut bytes = Vec::with_capacity(data.len() * 2);
            for &sample in data {
                let i16_sample = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                bytes.extend_from_slice(&i16_sample.to_le_bytes());
            }
            bytes
        } else {
            let samples_count = data.len() / channels;
            let mut bytes = Vec::with_capacity(samples_count * 2);

            for chunk in data.chunks_exact(channels) {
                let i16_sample = (chunk[0].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                bytes.extend_from_slice(&i16_sample.to_le_bytes());
            }
            bytes
        }
    }
}

pub fn convert_audio_data<T: AudioSample>(data: &[T], channels: usize) -> Vec<u8> {
    T::to_i16_mono(data, channels)
}
