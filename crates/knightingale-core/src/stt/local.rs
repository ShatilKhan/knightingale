//! Local STT backed by whisper-rs (whisper.cpp).
//!
//! Compiled in only when the `local-stt` cargo feature is on.

use std::path::Path;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::error::{KnightError, Result};

use super::Transcriber;

pub struct LocalWhisper {
    ctx: WhisperContext,
    language: Option<String>,
}

impl LocalWhisper {
    pub fn load(path: &Path, language: Option<String>) -> Result<Self> {
        if !path.exists() {
            return Err(KnightError::ModelMissing(format!(
                "{} not found; run `knightingale model pull <name>`",
                path.display()
            )));
        }
        let ctx = WhisperContext::new_with_params(
            path.to_string_lossy().as_ref(),
            WhisperContextParameters::default(),
        )
        .map_err(|e| KnightError::ModelMissing(format!("load model: {e}")))?;
        Ok(Self { ctx, language })
    }

    fn decode(&self, samples: &[f32], language: &str) -> Result<String> {
        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| KnightError::Audio(format!("whisper state: {e}")))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let lang = self.language.as_deref().unwrap_or(language);
        params.set_language(Some(lang));
        params.set_translate(false);
        params.set_print_realtime(false);
        params.set_print_progress(false);
        params.set_print_timestamps(false);
        params.set_print_special(false);
        params.set_suppress_blank(true);
        state
            .full(params, samples)
            .map_err(|e| KnightError::Audio(format!("whisper decode: {e}")))?;

        let mut text = String::new();
        let num = state
            .full_n_segments()
            .map_err(|e| KnightError::Audio(format!("segments: {e}")))?;
        for i in 0..num {
            let seg = state
                .full_get_segment_text(i)
                .map_err(|e| KnightError::Audio(format!("segment {i}: {e}")))?;
            text.push_str(&seg);
        }
        Ok(text.trim().to_string())
    }
}

impl Transcriber for LocalWhisper {
    fn transcribe(&self, wav: &[u8], language: &str) -> Result<String> {
        let samples = read_wav_samples(wav)?;
        self.decode(&samples, language)
    }
}

/// Parse a WAV blob (16-bit PCM mono) into f32 samples in [-1, 1].
fn read_wav_samples(wav: &[u8]) -> Result<Vec<f32>> {
    let cursor = std::io::Cursor::new(wav);
    let mut reader =
        hound::WavReader::new(cursor).map_err(|e| KnightError::Audio(format!("wav read: {e}")))?;
    let spec = reader.spec();
    if spec.channels != 1 {
        return Err(KnightError::Audio(format!(
            "expected mono wav, got {} channels",
            spec.channels
        )));
    }
    if spec.sample_rate != 16_000 {
        return Err(KnightError::Audio(format!(
            "expected 16kHz wav, got {}Hz",
            spec.sample_rate
        )));
    }
    let samples: Result<Vec<i16>> = reader
        .samples::<i16>()
        .map(|s| s.map_err(|e| KnightError::Audio(format!("sample: {e}"))))
        .collect();
    let samples = samples?;
    Ok(samples
        .iter()
        .map(|s| *s as f32 / i16::MAX as f32)
        .collect())
}
