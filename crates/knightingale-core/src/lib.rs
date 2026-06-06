//! Knightingale core: shared library for the daemon and CLI binaries.

pub mod audio;
pub mod config;
pub mod error;
pub mod eval;
pub mod focus;
pub mod hardware;
pub mod hotkey;
pub mod injection;
pub mod ipc;
pub mod model;
pub mod secret;
pub mod setup;
pub mod status;
pub mod stt;
pub mod tokens;

pub use config::Config;
pub use error::{KnightError, Result};
pub use secret::{ExposeSecret, SecretString};
pub use stt::{Provider, Transcriber, build_transcriber};

/// Load secrets from `~/.config/knightingale/.env`, then from a project-local
/// `.env` if present. Call this once at daemon startup.
pub fn load_env() {
    let _ = secret::load_env_file();
    let _ = dotenvy::dotenv();
}
