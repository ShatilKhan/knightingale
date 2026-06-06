use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::cache_dir;
use crate::error::{KnightError, Result};

const CATALOG: &str = include_str!("models.toml");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Model {
    pub alias: String,
    pub filename: String,
    pub url: String,
    pub sha256: String,
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

pub fn pull(alias: &str) -> Result<PathBuf> {
    let model = find(alias)?
        .ok_or_else(|| KnightError::ModelMissing(format!("unknown model alias: {alias}")))?;
    let dir = models_dir()?;
    fs::create_dir_all(&dir)?;
    let dest = local_path(&model)?;
    if dest.exists() {
        return Ok(dest);
    }
    let mut resp = reqwest::blocking::get(&model.url)
        .map_err(|e| KnightError::Network(format!("download: {e}")))?;
    if !resp.status().is_success() {
        return Err(KnightError::Network(format!(
            "download {}: HTTP {}",
            model.alias,
            resp.status()
        )));
    }
    let tmp = dest.with_extension("partial");
    let mut file = fs::File::create(&tmp)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = resp
            .read(&mut buf)
            .map_err(|e| KnightError::Network(format!("read: {e}")))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n])?;
    }
    file.flush()?;
    drop(file);
    let got = format!("{:x}", hasher.finalize());
    if !model.sha256.is_empty() && model.sha256 != got && !model.sha256.chars().all(|c| c == '0') {
        let _ = fs::remove_file(&tmp);
        return Err(KnightError::Other(format!(
            "sha256 mismatch for {}: expected {}, got {}",
            model.alias, model.sha256, got
        )));
    }
    fs::rename(&tmp, &dest)?;
    Ok(dest)
}
