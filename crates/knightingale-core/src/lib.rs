//! Knightingale core: shared library for the daemon and CLI binaries.

pub mod config;
pub mod error;

pub use config::Config;
pub use error::{KnightError, Result};
