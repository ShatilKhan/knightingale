//! CLI style: palette pulled from `knightingale_core::tokens`, banner embedded
//! from `design-system/cli/banner.txt`. Edit `design-system/tokens/tokens.rs`
//! and `design-system/cli/banner.txt` to rebrand.

use anstyle::{AnsiColor, Color, RgbColor, Style};

#[allow(dead_code)]
fn _ansi_dim() -> AnsiColor {
    AnsiColor::BrightBlack
}
use knightingale_core::tokens;

pub const BANNER: &str = include_str!("../../../design-system/cli/banner.txt");
pub const BANNER_WIDE: &str = include_str!("../../../design-system/cli/banner-wide.txt");

pub fn forest() -> Style {
    Style::new().fg_color(Some(rgb(tokens::FOREST)))
}

pub fn sage() -> Style {
    Style::new().fg_color(Some(rgb(tokens::SAGE)))
}

pub fn moss() -> Style {
    Style::new().fg_color(Some(rgb(tokens::MOSS)))
}

pub fn amber() -> Style {
    Style::new().fg_color(Some(rgb(tokens::AMBER)))
}

pub fn coral() -> Style {
    Style::new().fg_color(Some(rgb(tokens::CORAL)))
}

#[allow(dead_code)]
pub fn dim() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)))
}

fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::Rgb(RgbColor(r, g, b))
}

/// clap-compatible styles. Applied via `#[command(styles = ...)]`.
pub fn clap_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .header(forest().bold())
        .usage(forest().bold())
        .literal(moss())
        .placeholder(sage())
        .error(coral().bold())
        .valid(moss())
        .invalid(amber())
}
