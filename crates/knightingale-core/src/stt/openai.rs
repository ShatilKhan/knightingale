use std::time::Duration;

use serde::Deserialize;

use crate::error::{KnightError, Result};
use crate::secret::{ExposeSecret, SecretString};

use super::Transcriber;

#[derive(Clone)]
pub struct OpenAiClient {
    pub base_url: String,
    pub api_key: SecretString,
    pub model: String,
    pub timeout: Duration,
}

impl OpenAiClient {
    pub fn new(base_url: impl Into<String>, api_key: SecretString, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            model: model.into(),
            timeout: Duration::from_secs(60),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/audio/transcriptions", self.base_url)
    }
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

impl Transcriber for OpenAiClient {
    fn transcribe(&self, wav: &[u8], language: &str) -> Result<String> {
        let url = self.endpoint();
        let part = reqwest::blocking::multipart::Part::bytes(wav.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| KnightError::Network(format!("mime: {e}")))?;
        let form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model.clone())
            .text("language", language.to_string())
            .text("response_format", "json")
            .part("file", part);

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| KnightError::Network(format!("client: {e}")))?;

        let resp = client
            .post(&url)
            .bearer_auth(self.api_key.expose_secret())
            .multipart(form)
            .send()
            .map_err(|e| KnightError::Network(format!("send: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
                return Err(KnightError::Auth(format!("{status}: {body}")));
            }
            return Err(KnightError::Network(format!("{status}: {body}")));
        }

        let parsed: TranscriptionResponse = resp
            .json()
            .map_err(|e| KnightError::Network(format!("parse: {e}")))?;
        Ok(parsed.text.trim().to_string())
    }
}
