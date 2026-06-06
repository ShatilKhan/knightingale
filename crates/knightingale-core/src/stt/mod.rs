use crate::error::Result;

pub mod openai;

pub use openai::OpenAiClient;

/// Backend that turns a WAV blob into a text transcript.
pub trait Transcriber: Send {
    fn transcribe(&self, wav: &[u8], language: &str) -> Result<String>;
}
