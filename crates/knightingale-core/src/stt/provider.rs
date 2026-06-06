use serde::{Deserialize, Serialize};

use crate::error::{KnightError, Result};
use crate::secret::SecretString;

use super::openai::OpenAiClient;
use super::Transcriber;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Groq,
    Openai,
    Deepinfra,
    Fireworks,
    Lemonfox,
    Sambanova,
    Azure,
    Custom,
    Local,
}

impl Provider {
    pub fn from_env() -> Self {
        match std::env::var("KNIGHTINGALE_PROVIDER")
            .ok()
            .map(|s| s.to_lowercase())
            .as_deref()
        {
            Some("groq") => Provider::Groq,
            Some("openai") => Provider::Openai,
            Some("deepinfra") => Provider::Deepinfra,
            Some("fireworks") => Provider::Fireworks,
            Some("lemonfox") => Provider::Lemonfox,
            Some("sambanova") => Provider::Sambanova,
            Some("azure") => Provider::Azure,
            Some("custom") => Provider::Custom,
            Some("local") => Provider::Local,
            _ => Provider::Groq,
        }
    }

    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            Provider::Groq => Some("https://api.groq.com/openai/v1"),
            Provider::Openai => Some("https://api.openai.com/v1"),
            Provider::Deepinfra => Some("https://api.deepinfra.com/v1/openai"),
            Provider::Fireworks => Some("https://api.fireworks.ai/inference/v1"),
            Provider::Lemonfox => Some("https://api.lemonfox.ai/v1"),
            Provider::Sambanova => Some("https://api.sambanova.ai/v1"),
            Provider::Azure | Provider::Custom => None,
            Provider::Local => None,
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Groq => "whisper-large-v3-turbo",
            Provider::Openai => "whisper-1",
            Provider::Deepinfra => "openai/whisper-large-v3-turbo",
            Provider::Fireworks => "whisper-v3-turbo",
            Provider::Lemonfox => "whisper-1",
            Provider::Sambanova => "Whisper-Large-v3",
            Provider::Azure => "",
            Provider::Custom => "",
            Provider::Local => "distil-small.en",
        }
    }

    pub fn env_prefix(self) -> &'static str {
        match self {
            Provider::Groq => "GROQ",
            Provider::Openai => "OPENAI",
            Provider::Deepinfra => "DEEPINFRA",
            Provider::Fireworks => "FIREWORKS",
            Provider::Lemonfox => "LEMONFOX",
            Provider::Sambanova => "SAMBANOVA",
            Provider::Azure => "AZURE_OPENAI",
            Provider::Custom => "CUSTOM",
            Provider::Local => "KNIGHTINGALE",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Groq => "groq",
            Provider::Openai => "openai",
            Provider::Deepinfra => "deepinfra",
            Provider::Fireworks => "fireworks",
            Provider::Lemonfox => "lemonfox",
            Provider::Sambanova => "sambanova",
            Provider::Azure => "azure",
            Provider::Custom => "custom",
            Provider::Local => "local",
        }
    }

    /// Build an OpenAI-compatible client from process environment + .env file.
    /// Returns `None` for providers that do not use the OpenAI shape (Local).
    pub fn build_openai_client(self) -> Result<Option<OpenAiClient>> {
        match self {
            Provider::Local => Ok(None),
            Provider::Azure => Err(KnightError::Config(
                "azure uses a separate adapter; not yet wired".into(),
            )),
            Provider::Custom => {
                let base_url = std::env::var("CUSTOM_BASE_URL")
                    .map_err(|_| KnightError::Config("CUSTOM_BASE_URL is required for custom provider".into()))?;
                let api_key = std::env::var("CUSTOM_API_KEY").unwrap_or_default();
                let model = std::env::var("CUSTOM_MODEL")
                    .map_err(|_| KnightError::Config("CUSTOM_MODEL is required for custom provider".into()))?;
                Ok(Some(OpenAiClient::new(
                    base_url,
                    SecretString::from(api_key),
                    model,
                )))
            }
            _ => {
                let key_var = format!("{}_API_KEY", self.env_prefix());
                let api_key = std::env::var(&key_var)
                    .map_err(|_| KnightError::Auth(format!("{key_var} not set")))?;
                let base_url = self.default_base_url().unwrap_or("").to_string();
                let model = std::env::var(format!("{}_MODEL", self.env_prefix()))
                    .unwrap_or_else(|_| self.default_model().to_string());
                Ok(Some(OpenAiClient::new(
                    base_url,
                    SecretString::from(api_key),
                    model,
                )))
            }
        }
    }
}

pub fn build_transcriber(provider: Provider) -> Result<Box<dyn Transcriber>> {
    match provider {
        Provider::Local => Err(KnightError::ModelMissing(
            "local backend not yet wired; configure a cloud provider for now".into(),
        )),
        Provider::Azure => Err(KnightError::Config(
            "azure adapter not yet wired".into(),
        )),
        other => {
            let client = other.build_openai_client()?.ok_or_else(|| {
                KnightError::Config(format!("no client for provider {}", other.as_str()))
            })?;
            Ok(Box::new(client))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groq_defaults() {
        assert_eq!(Provider::Groq.default_base_url().unwrap(), "https://api.groq.com/openai/v1");
        assert_eq!(Provider::Groq.default_model(), "whisper-large-v3-turbo");
        assert_eq!(Provider::Groq.env_prefix(), "GROQ");
    }

    #[test]
    fn provider_round_trips_via_str() {
        for p in [
            Provider::Groq,
            Provider::Openai,
            Provider::Deepinfra,
            Provider::Fireworks,
            Provider::Lemonfox,
            Provider::Sambanova,
            Provider::Azure,
            Provider::Custom,
            Provider::Local,
        ] {
            assert!(!p.as_str().is_empty());
            assert!(!p.env_prefix().is_empty());
        }
    }
}
