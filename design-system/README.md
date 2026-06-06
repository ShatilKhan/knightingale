# Design System

Brand assets, tokens, and copy guidelines for Knightingale.

This folder ships with the source code. The CLI binary embeds `cli/banner.txt` via `include_str!`; the docs site reads `tokens/tokens.css` at build time; the `knightingale-cli` style module sources palette constants from `tokens/tokens.rs`.

## Layout

| Path | Purpose |
|---|---|
| `brand/knitingale-icon/1.png`, `1.svg` | Full logo (originals) |
| `brand/knitingale-icon/1-readme.png` | 480 px wide, used by README `<img>` |
| `brand/knitingale-icon/1-social.png` | 1200×630 social preview / `og:image` |
| `brand/knitingale-icon/2.png`, `2.svg` | Bird-only mark (originals) |
| `brand/knitingale-icon/2-mark.png` | 240 px wide bird mark for inline use |
| `tokens/colors.md` | Palette with hex, ANSI 256, and usage rules |
| `tokens/tokens.css` | CSS custom properties for the landing page |
| `tokens/tokens.rs` | Rust constants `include!`d by `knightingale-cli` |
| `cli/banner.txt` | ASCII bird (~40×16) shown on `knightingale setup` and `--version` |
| `cli/banner-wide.txt` | ~60×24 variant for full-screen banners |
| `cli/style-guide.md` | Terminal colour rules, spacing, microcopy |
| `web/typography.md` | Font choices and scale for the docs site |
| `web/voice.md` | Tone and copy guidelines |

## A note on the SVGs

The two `.svg` originals are Canva exports — each is a thin SVG wrapper around six base64-embedded raster images. They are not editable vectors. Stripping their C2PA metadata only trims ~20 KB out of ~4 MB, because the bulk is raster payload. In practice we serve the optimised PNGs everywhere; the SVGs stay in the repo only as the highest-fidelity source we have today. Replace with a true vector when time permits.

## Brand

Vintage botanical-engraving nightingale, forest-and-sage palette. Audubon aesthetic. Voice register: rustup / uv / ripgrep — dry, precise, no marketing exclamation, active voice, concrete numbers.
