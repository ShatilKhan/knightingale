use crate::error::Result;

pub mod openai;
pub mod provider;

pub use openai::OpenAiClient;
pub use provider::{Provider, build_transcriber};

/// Backend that turns a WAV blob into a text transcript.
pub trait Transcriber: Send {
    fn transcribe(&self, wav: &[u8], language: &str) -> Result<String>;
}
