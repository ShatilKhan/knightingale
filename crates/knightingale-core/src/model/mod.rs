use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::cache_dir;
use crate::error::{KnightError, Result};

const CATALOG: &str = include_str!("models.toml");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Model {
    pub alias: String,
    pub filename: String,
    pub url: String,
    pub size_mb: u32,
    pub vram_mb: u32,
    pub language: String,
    pub recommended_for: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Catalog {
    model: Vec<Model>,
}

pub fn catalog() -> Result<Vec<Model>> {
    let parsed: Catalog =
        toml::from_str(CATALOG).map_err(|e| KnightError::Other(format!("models.toml: {e}")))?;
    Ok(parsed.model)
}

pub fn models_dir() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("KNIGHTINGALE_MODEL_DIR") {
        return Ok(PathBuf::from(p));
    }
    Ok(cache_dir()?.join("models"))
}

pub fn find(alias: &str) -> Result<Option<Model>> {
    Ok(catalog()?.into_iter().find(|m| m.alias == alias))
}

pub fn local_path(model: &Model) -> Result<PathBuf> {
    Ok(models_dir()?.join(&model.filename))
}

pub fn is_installed(alias: &str) -> Result<bool> {
    if let Some(m) = find(alias)? {
        Ok(local_path(&m)?.exists())
    } else {
        Ok(false)
    }
}

/// Shell command the user can run to download a model.
///
/// Knightingale deliberately does not auto-download Whisper checkpoints —
/// that's the job of a model manager like Ollama. This returns the canonical
/// curl one-liner; the user runs it once and points
/// `KNIGHTINGALE_MODEL_PATH` at the result.
pub fn download_command(alias: &str) -> Result<String> {
    let model = find(alias)?
        .ok_or_else(|| KnightError::ModelMissing(format!("unknown model alias: {alias}")))?;
    let dir = models_dir()?;
    let dest = local_path(&model)?;
    Ok(format!(
        "mkdir -p {dir} && curl -fL '{url}' -o '{dest}'",
        dir = dir.display(),
        url = model.url,
        dest = dest.display()
    ))
}
