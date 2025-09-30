use crate::error::{AudioError, Result};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};

pub fn resample_audio(
    audio_data: &[u8],
    from_sample_rate: u32,
    to_sample_rate: u32,
) -> Result<Vec<u8>> {
    if from_sample_rate == to_sample_rate {
        return Ok(audio_data.to_vec());
    }

    let samples: Vec<i16> = audio_data
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    let samples_f32: Vec<f32> = samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect();

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        to_sample_rate as f64 / from_sample_rate as f64,
        2.0,
        params,
        samples_f32.len(),
        1,
    )
    .map_err(|e| {
        AudioError::ResamplingError(format!(
            "Failed to create resampler ({}Hz -> {}Hz): {}",
            from_sample_rate, to_sample_rate, e
        ))
    })?;

    let waves_in = vec![samples_f32];
    let waves_out = resampler.process(&waves_in, None).map_err(|e| {
        AudioError::ResamplingError(format!("Failed to resample audio: {}", e))
    })?;

    let resampled_f32 = &waves_out[0];
    let resampled_i16: Vec<i16> = resampled_f32
        .iter()
        .map(|&s| {
            let clamped = s.clamp(-1.0, 1.0);
            (clamped * i16::MAX as f32) as i16
        })
        .collect();

    let mut resampled_bytes = Vec::with_capacity(resampled_i16.len() * 2);
    for sample in resampled_i16 {
        resampled_bytes.extend_from_slice(&sample.to_le_bytes());
    }

    log::debug!(
        "Resampled audio from {}Hz to {}Hz: {} bytes -> {} bytes",
        from_sample_rate,
        to_sample_rate,
        audio_data.len(),
        resampled_bytes.len()
    );

    Ok(resampled_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_same_rate() {
        let audio_data = vec![0u8, 1, 2, 3, 4, 5];
        let result = resample_audio(&audio_data, 48000, 48000).unwrap();
        assert_eq!(result, audio_data);
    }

    #[test]
    fn test_resample_downsample() {
        let sample_count = 48000;
        let mut audio_data = Vec::new();
        for i in 0..sample_count {
            let sample = ((i as f32 / sample_count as f32) * i16::MAX as f32) as i16;
            audio_data.extend_from_slice(&sample.to_le_bytes());
        }

        let result = resample_audio(&audio_data, 48000, 16000).unwrap();

        let expected_samples = (sample_count as f32 * (16000.0 / 48000.0)) as usize;
        let expected_bytes = expected_samples * 2;
        let tolerance = expected_bytes / 10;

        assert!(
            result.len() > expected_bytes - tolerance
                && result.len() < expected_bytes + tolerance,
            "Expected ~{} bytes, got {}",
            expected_bytes,
            result.len()
        );
    }

    #[test]
    fn test_resample_upsample() {
        let sample_count = 16000;
        let mut audio_data = Vec::new();
        for i in 0..sample_count {
            let sample = ((i as f32 / sample_count as f32) * i16::MAX as f32) as i16;
            audio_data.extend_from_slice(&sample.to_le_bytes());
        }

        let result = resample_audio(&audio_data, 16000, 48000).unwrap();

        let expected_samples = (sample_count as f32 * (48000.0 / 16000.0)) as usize;
        let expected_bytes = expected_samples * 2;
        let tolerance = expected_bytes / 10;

        assert!(
            result.len() > expected_bytes - tolerance
                && result.len() < expected_bytes + tolerance,
            "Expected ~{} bytes, got {}",
            expected_bytes,
            result.len()
        );
    }
}