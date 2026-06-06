//! Knightingale core: shared library for the daemon and CLI binaries.

pub mod config;
pub mod error;
pub mod secret;

pub use config::Config;
pub use error::{KnightError, Result};
pub use secret::{ExposeSecret, SecretString};
