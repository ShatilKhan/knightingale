//! Evaluation harness: WER + RTF + SER + resource sampling.

pub mod metrics;

pub use metrics::{cer, levenshtein, ser, wer};

use std::path::Path;
use std::time::Instant;

use serde::Serialize;
use sysinfo::System;

use crate::audio;
use crate::error::{KnightError, Result};
use crate::stt::Transcriber;

#[derive(Debug, Clone, Serialize)]
pub struct ClipResult {
    pub clip: String,
    pub reference: String,
    pub hypothesis: String,
    pub audio_secs: f64,
    pub processing_secs: f64,
    pub rtf: f64,
    pub wer_pct: f64,
    pub correct: bool,
    pub peak_rss_mb: u64,
}

pub fn run_clip(
    transcriber: &dyn Transcriber,
    wav_path: &Path,
    reference: &str,
    language: &str,
) -> Result<ClipResult> {
    let bytes = std::fs::read(wav_path)?;
    let audio_secs = wav_duration_secs(&bytes)?;

    let mut sys = System::new();
    sys.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(std::process::id())]),
        true,
    );
    let baseline_rss = sys
        .process(sysinfo::Pid::from_u32(std::process::id()))
        .map(|p| p.memory() / 1024 / 1024)
        .unwrap_or(0);

    let start = Instant::now();
    let hypothesis = transcriber.transcribe(&bytes, language)?;
    let processing_secs = start.elapsed().as_secs_f64();

    sys.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(std::process::id())]),
        true,
    );
    let peak_rss = sys
        .process(sysinfo::Pid::from_u32(std::process::id()))
        .map(|p| p.memory() / 1024 / 1024)
        .unwrap_or(0);

    let rtf = if audio_secs > 0.0 {
        processing_secs / audio_secs
    } else {
        0.0
    };
    let wer_pct = wer(reference, &hypothesis) * 100.0;
    let correct = hypothesis.trim().eq_ignore_ascii_case(reference.trim());

    Ok(ClipResult {
        clip: wav_path.display().to_string(),
        reference: reference.to_string(),
        hypothesis,
        audio_secs,
        processing_secs,
        rtf,
        wer_pct,
        correct,
        peak_rss_mb: peak_rss.saturating_sub(baseline_rss).max(peak_rss),
    })
}

fn wav_duration_secs(wav: &[u8]) -> Result<f64> {
    let cursor = std::io::Cursor::new(wav);
    let reader =
        hound::WavReader::new(cursor).map_err(|e| KnightError::Audio(format!("wav: {e}")))?;
    let spec = reader.spec();
    let samples = reader.duration() as f64;
    Ok(samples / spec.sample_rate as f64)
}

#[derive(Debug, Serialize)]
pub struct Aggregate {
    pub avg_wer_pct: f64,
    pub avg_rtf: f64,
    pub ser_pct: f64,
}

pub fn aggregate(rows: &[ClipResult]) -> Aggregate {
    if rows.is_empty() {
        return Aggregate {
            avg_wer_pct: 0.0,
            avg_rtf: 0.0,
            ser_pct: 0.0,
        };
    }
    let n = rows.len() as f64;
    let avg_wer_pct = rows.iter().map(|r| r.wer_pct).sum::<f64>() / n;
    let avg_rtf = rows.iter().map(|r| r.rtf).sum::<f64>() / n;
    let wrong = rows.iter().filter(|r| !r.correct).count() as f64;
    let ser_pct = (wrong / n) * 100.0;
    Aggregate {
        avg_wer_pct,
        avg_rtf,
        ser_pct,
    }
}

// Re-export audio helpers so the CLI's eval command can reuse them.
pub use audio::{pcm_to_wav, trim_edge_silence};
