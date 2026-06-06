use std::time::Duration;

use serde::Deserialize;

use crate::error::{KnightError, Result};
use crate::secret::{ExposeSecret, SecretString};

use super::Transcriber;

/// Azure OpenAI uses a templated URL with deployment + api-version.
///
/// Final endpoint:
///   {endpoint}/openai/deployments/{deployment}/audio/transcriptions?api-version={version}
#[derive(Clone)]
pub struct AzureClient {
    pub endpoint: String,
    pub api_key: SecretString,
    pub deployment: String,
    pub api_version: String,
    pub timeout: Duration,
}

impl AzureClient {
    pub fn new(
        endpoint: impl Into<String>,
        api_key: SecretString,
        deployment: impl Into<String>,
        api_version: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into().trim_end_matches('/').to_string(),
            api_key,
            deployment: deployment.into(),
            api_version: api_version.into(),
            timeout: Duration::from_secs(60),
        }
    }

    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT")
            .map_err(|_| KnightError::Config("AZURE_OPENAI_ENDPOINT not set".into()))?;
        let api_key = std::env::var("AZURE_OPENAI_API_KEY")
            .map_err(|_| KnightError::Auth("AZURE_OPENAI_API_KEY not set".into()))?;
        let deployment = std::env::var("AZURE_OPENAI_DEPLOYMENT")
            .map_err(|_| KnightError::Config("AZURE_OPENAI_DEPLOYMENT not set".into()))?;
        let api_version =
            std::env::var("AZURE_OPENAI_API_VERSION").unwrap_or_else(|_| "2024-06-01".to_string());
        Ok(Self::new(
            endpoint,
            SecretString::from(api_key),
            deployment,
            api_version,
        ))
    }

    fn url(&self) -> String {
        format!(
            "{}/openai/deployments/{}/audio/transcriptions?api-version={}",
            self.endpoint, self.deployment, self.api_version
        )
    }
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

impl Transcriber for AzureClient {
    fn transcribe(&self, wav: &[u8], language: &str) -> Result<String> {
        let url = self.url();
        let part = reqwest::blocking::multipart::Part::bytes(wav.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| KnightError::Network(format!("mime: {e}")))?;
        let form = reqwest::blocking::multipart::Form::new()
            .text("language", language.to_string())
            .text("response_format", "json")
            .part("file", part);

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| KnightError::Network(format!("client: {e}")))?;

        let resp = client
            .post(&url)
            .header("api-key", self.api_key.expose_secret())
            .multipart(form)
            .send()
            .map_err(|e| KnightError::Network(format!("send: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_templating() {
        let client = AzureClient::new(
            "https://example.openai.azure.com/",
            SecretString::from("k".to_string()),
            "whisper-deploy",
            "2024-06-01",
        );
        assert_eq!(
            client.url(),
            "https://example.openai.azure.com/openai/deployments/whisper-deploy/audio/transcriptions?api-version=2024-06-01"
        );
    }
}
