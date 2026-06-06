//! Streaming STT scaffold (deepgram, openai-realtime, assemblyai).
//!
//! Phase 4 placeholder. Streaming providers use websocket transports that need
//! tokio + tokio-tungstenite. The trait below lets us plug them in once a
//! concrete user need surfaces without rewiring the daemon.

use crate::error::Result;

/// Push-driven incremental transcription.
///
/// Implementations live in `streaming::deepgram`, `streaming::openai_realtime`,
/// `streaming::assemblyai`. None are wired today.
pub trait StreamingTranscriber: Send {
    /// Open the connection. Returns when the server has acknowledged.
    fn begin(&mut self) -> Result<()>;
    /// Push a chunk of 16 kHz mono i16 PCM.
    fn push(&mut self, samples: &[i16]) -> Result<()>;
    /// Close the connection and return the final transcript.
    fn finish(&mut self) -> Result<String>;
}
