use crate::error::Result;

pub mod azure;
#[cfg(feature = "local-stt")]
pub mod local;
pub mod openai;
pub mod provider;
pub mod streaming;

pub use azure::AzureClient;
#[cfg(feature = "local-stt")]
pub use local::LocalWhisper;
pub use openai::OpenAiClient;
pub use provider::{Provider, build_transcriber};

/// Backend that turns a WAV blob into a text transcript.
pub trait Transcriber: Send {
    fn transcribe(&self, wav: &[u8], language: &str) -> Result<String>;
}
