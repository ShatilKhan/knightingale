//! Knightingale core: shared library for the daemon and CLI binaries.

pub mod audio;
pub mod config;
pub mod error;
pub mod secret;
pub mod tokens;

pub use config::Config;
pub use error::{KnightError, Result};
pub use secret::{ExposeSecret, SecretString};
