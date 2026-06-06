use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

use crate::error::{KnightError, Result};

pub const APP_QUALIFIER: &str = "dev";
pub const APP_ORGANIZATION: &str = "shatilkhan";
pub const APP_NAME: &str = "knightingale";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    #[serde(default)]
    pub stt: SttConfig,
    #[serde(default)]
    pub injection: InjectionConfig,
    #[serde(default)]
    pub audio: AudioConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub toggle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    pub backend: SttBackend,
    pub language: String,
    pub max_recording_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SttBackend {
    OpenaiCompatible,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionConfig {
    pub method: InjectionMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectionMethod {
    Auto,
    Enigo,
    Uinput,
    ClipboardPaste,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub mic: Option<String>,
    pub silence_threshold: i16,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            toggle: default_hotkey().to_string(),
        }
    }
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            backend: SttBackend::OpenaiCompatible,
            language: "en".to_string(),
            max_recording_secs: 300,
        }
    }
}

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            method: InjectionMethod::Auto,
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            mic: None,
            silence_threshold: 500,
        }
    }
}

fn default_hotkey() -> &'static str {
    if cfg!(target_os = "macos") {
        "cmd+shift+k"
    } else {
        "super+k"
    }
}

pub fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
        .ok_or_else(|| KnightError::Config("could not resolve project directories".into()))
}

pub fn config_dir() -> Result<PathBuf> {
    Ok(project_dirs()?.config_dir().to_path_buf())
}

pub fn cache_dir() -> Result<PathBuf> {
    Ok(project_dirs()?.cache_dir().to_path_buf())
}

pub fn state_dir() -> Result<PathBuf> {
    let dirs = project_dirs()?;
    // ProjectDirs only exposes state_dir on Linux; fall back to data_dir elsewhere.
    Ok(dirs
        .state_dir()
        .unwrap_or_else(|| dirs.data_local_dir())
        .to_path_buf())
}

pub fn config_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn env_file() -> Result<PathBuf> {
    Ok(config_dir()?.join(".env"))
}

pub fn runtime_socket() -> Result<PathBuf> {
    let path = if cfg!(target_os = "linux") {
        std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| cache_dir().unwrap_or_else(|_| PathBuf::from("/tmp")))
            .join("knightingale.sock")
    } else if cfg!(target_os = "macos") {
        cache_dir()?.join("knightingale.sock")
    } else {
        PathBuf::from(r"\\.\pipe\knightingale")
    };
    Ok(path)
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_file()?;
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let mut fig = Figment::from(Serialized::defaults(Self::default()));
        if path.exists() {
            fig = fig.merge(Toml::file(path));
        }
        fig = fig.merge(Env::prefixed("KNIGHTINGALE_").split("__"));
        fig.extract()
            .map_err(|e| KnightError::Config(e.to_string()))
    }

    pub fn save(&self) -> Result<()> {
        let path = config_file()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)
            .map_err(|e| KnightError::Config(format!("serialize: {e}")))?;
        std::fs::write(&path, body)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_serialize_round_trip() {
        let cfg = Config::default();
        let s = toml::to_string_pretty(&cfg).unwrap();
        let parsed: Config = toml::from_str(&s).unwrap();
        assert_eq!(parsed.stt.backend, SttBackend::OpenaiCompatible);
        assert_eq!(parsed.stt.max_recording_secs, 300);
    }

    #[test]
    fn defaults_hotkey_per_os() {
        let cfg = Config::default();
        if cfg!(target_os = "macos") {
            assert_eq!(cfg.hotkey.toggle, "cmd+shift+k");
        } else {
            assert_eq!(cfg.hotkey.toggle, "super+k");
        }
    }
}
