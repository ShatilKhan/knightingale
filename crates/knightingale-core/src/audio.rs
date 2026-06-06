use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use tracing::warn;

use crate::error::{KnightError, Result};

/// Whisper expects 16 kHz mono PCM.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// A handle to an active recording. Dropping it stops capture.
pub struct Recording {
    stop: Arc<AtomicBool>,
    samples_rx: Option<mpsc::Receiver<Vec<i16>>>,
    join: Option<JoinHandle<Result<Vec<i16>>>>,
    _stream: Stream,
}

impl Recording {
    pub fn stop(mut self) -> Result<Vec<i16>> {
        self.stop.store(true, Ordering::SeqCst);
        drop(self.samples_rx.take());
        match self.join.take() {
            Some(h) => h
                .join()
                .map_err(|_| KnightError::Audio("recorder thread panicked".into()))?,
            None => Err(KnightError::Audio("recorder already taken".into())),
        }
    }
}

pub fn list_input_devices() -> Result<Vec<String>> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|e| KnightError::Audio(format!("enumerate input devices: {e}")))?;
    let mut names = Vec::new();
    for device in devices {
        if let Ok(name) = device.name() {
            names.push(name);
        }
    }
    Ok(names)
}

fn pick_input(mic: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    if let Some(target) = mic {
        let devices = host
            .input_devices()
            .map_err(|e| KnightError::Audio(format!("enumerate input devices: {e}")))?;
        for d in devices {
            if d.name().map(|n| n == target).unwrap_or(false) {
                return Ok(d);
            }
        }
        warn!(?target, "configured mic not found; falling back to default");
    }
    host.default_input_device()
        .ok_or_else(|| KnightError::Audio("no default input device available".into()))
}

pub fn start_recording(mic: Option<&str>) -> Result<Recording> {
    let device = pick_input(mic)?;
    let config = device
        .default_input_config()
        .map_err(|e| KnightError::Audio(format!("default input config: {e}")))?;
    let sample_format = config.sample_format();
    let channels = config.channels() as usize;
    let source_rate = config.sample_rate().0;

    let (samples_tx, samples_rx) = mpsc::channel::<Vec<i16>>();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_stream = stop.clone();

    let err_fn = |err| warn!(error = %err, "audio stream error");
    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                if stop_for_stream.load(Ordering::SeqCst) {
                    return;
                }
                let mono = downmix_f32(data, channels);
                let _ = samples_tx.send(f32_to_i16(&mono));
            },
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _| {
                if stop_for_stream.load(Ordering::SeqCst) {
                    return;
                }
                let mono = downmix_i16(data, channels);
                let _ = samples_tx.send(mono);
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data: &[u16], _| {
                if stop_for_stream.load(Ordering::SeqCst) {
                    return;
                }
                let mono: Vec<i16> = data
                    .iter()
                    .map(|s| (*s as i32 - i16::MAX as i32) as i16)
                    .collect();
                let _ = samples_tx.send(downmix_i16(&mono, channels));
            },
            err_fn,
            None,
        ),
        other => {
            return Err(KnightError::Audio(format!(
                "unsupported sample format: {other:?}"
            )));
        }
    }
    .map_err(|e| KnightError::Audio(format!("build input stream: {e}")))?;

    stream
        .play()
        .map_err(|e| KnightError::Audio(format!("play stream: {e}")))?;

    let stop_for_thread = stop.clone();
    let (collector_tx, collector_rx) = mpsc::channel::<Vec<i16>>();
    let join = thread::spawn(move || -> Result<Vec<i16>> {
        let mut buffer: Vec<i16> = Vec::with_capacity(TARGET_SAMPLE_RATE as usize * 30);
        loop {
            match collector_rx.recv_timeout(Duration::from_millis(250)) {
                Ok(chunk) => buffer.extend(chunk),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if stop_for_thread.load(Ordering::SeqCst) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        if source_rate == TARGET_SAMPLE_RATE {
            Ok(buffer)
        } else {
            resample_to_16k(&buffer, source_rate)
        }
    });

    // Bridge samples_rx (callback) -> collector_tx (worker thread).
    let bridge_stop = stop.clone();
    thread::spawn(move || {
        while let Ok(chunk) = samples_rx.recv() {
            if bridge_stop.load(Ordering::SeqCst) {
                break;
            }
            if collector_tx.send(chunk).is_err() {
                break;
            }
        }
    });

    Ok(Recording {
        stop,
        samples_rx: None,
        join: Some(join),
        _stream: stream,
    })
}

fn downmix_f32(data: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }
    data.chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
        .collect()
}

fn downmix_i16(data: &[i16], channels: usize) -> Vec<i16> {
    if channels <= 1 {
        return data.to_vec();
    }
    data.chunks(channels)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|s| *s as i32).sum();
            (sum / channels as i32) as i16
        })
        .collect()
}

fn f32_to_i16(data: &[f32]) -> Vec<i16> {
    data.iter()
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

fn resample_to_16k(input: &[i16], source_rate: u32) -> Result<Vec<i16>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let in_f32: Vec<f32> = input
        .iter()
        .map(|s| *s as f32 / i16::MAX as f32)
        .collect();

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    let mut resampler = SincFixedIn::<f32>::new(
        TARGET_SAMPLE_RATE as f64 / source_rate as f64,
        2.0,
        params,
        in_f32.len(),
        1,
    )
    .map_err(|e| KnightError::Audio(format!("init resampler: {e}")))?;

    let out = resampler
        .process(&[in_f32], None)
        .map_err(|e| KnightError::Audio(format!("resample: {e}")))?;

    let resampled: Vec<i16> = out[0]
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();
    Ok(resampled)
}

/// Trim leading and trailing silence from a PCM buffer.
///
/// Samples below `threshold` (absolute value) are considered silent. Cuts the
/// dead air that Whisper sometimes hallucinates "thank you for watching" /
/// "subtitles by …" over, and shrinks payload to the cloud STT endpoint.
pub fn trim_edge_silence(samples: &[i16], threshold: i16) -> &[i16] {
    let threshold = threshold.unsigned_abs() as i32;
    let start = samples
        .iter()
        .position(|s| (*s as i32).unsigned_abs() as i32 > threshold)
        .unwrap_or(samples.len());
    if start == samples.len() {
        return &[];
    }
    let end = samples.len()
        - samples
            .iter()
            .rev()
            .position(|s| (*s as i32).unsigned_abs() as i32 > threshold)
            .unwrap_or(0);
    &samples[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_stereo_to_mono() {
        let stereo = vec![100i16, 200, 300, 400];
        let mono = downmix_i16(&stereo, 2);
        assert_eq!(mono, vec![150, 350]);
    }

    #[test]
    fn f32_clamps_and_scales() {
        let input = vec![0.0f32, 1.0, -1.0, 2.0, -2.0];
        let out = f32_to_i16(&input);
        assert_eq!(out, vec![0, i16::MAX, -i16::MAX, i16::MAX, -i16::MAX]);
    }

    #[test]
    fn trim_strips_edges() {
        let samples: Vec<i16> = vec![10, 20, 30, 5000, 6000, 7000, 40, 30, 20];
        let trimmed = trim_edge_silence(&samples, 100);
        assert_eq!(trimmed, &[5000, 6000, 7000]);
    }

    #[test]
    fn trim_handles_all_silent() {
        let samples: Vec<i16> = vec![10, 20, 30, 40];
        let trimmed = trim_edge_silence(&samples, 100);
        assert!(trimmed.is_empty());
    }
}
