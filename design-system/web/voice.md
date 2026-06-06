# Voice

Copy guidelines for the README, docs site, and CLI output.

## Register

Dry, precise, slightly literary. Reference voices: `rustup`, `uv` (Astral), `ripgrep`, `bat`. Avoid: marketing exclamations, "blazingly fast", "the future of", emoji confetti, AI-adjacent superlatives.

## Rules

- **Active voice.** "Knightingale captures audio" — not "Audio is captured by Knightingale".
- **Concrete numbers.** "~120 MB on disk" — not "small".
- **Short sentences.** One idea per sentence.
- **No exclamation points** outside microcopy success states (`✓ Done`).
- **No second-person hype.** "You'll love it" is out; "Knightingale runs in the background and types what you say" is in.
- **Lowercase for code-like terms** in prose: `cargo`, `dotenvy`, `~/.config/knightingale/.env`.

## Tagline candidates

(Final pick written into README on commit 44.)

- "Voice dictation that minds its own business."
- "Hotkey, speak, paste."
- "Local-first dictation for the terminal."

## Privacy statement (committed prose)

> Knightingale does not collect or transmit telemetry of any kind. No usage metrics, no error reports, no analytics, no phone-home. Your audio, transcripts, and configuration stay on your machine unless you explicitly point Knightingale at a cloud provider — in which case audio is sent **only** to the API endpoint configured in your `.env`, with your key, and nowhere else. No proxy, no cache, no logging by us.
