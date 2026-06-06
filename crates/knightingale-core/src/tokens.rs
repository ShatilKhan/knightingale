//! Palette tokens.
//!
//! Re-exported from `design-system/tokens/tokens.rs` so designers can edit one
//! file and the CLI picks up the change at the next build.

#[path = "../../../design-system/tokens/tokens.rs"]
mod inner;

pub use inner::*;
