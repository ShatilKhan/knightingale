# Typography

Fonts and scale for the docs landing page (`docs/index.html`).

## Stack

| Role | Stack |
|---|---|
| Headline | `EB Garamond`, `Lora`, Georgia, serif |
| Body | `Inter`, `IBM Plex Sans`, system-ui, sans-serif |
| Mono | `JetBrains Mono`, `IBM Plex Mono`, ui-monospace, monospace |

EB Garamond matches the engraving aesthetic without being theme-park. Inter on body for legibility on small screens.

## Scale (rem, root 16 px)

| Token | Size | Use |
|---|---|---|
| `--type-display` | 3.75 | Hero |
| `--type-h1` | 2.5 | Section heading |
| `--type-h2` | 1.75 | Subsection |
| `--type-h3` | 1.25 | Block heading |
| `--type-body` | 1.0 | Paragraph |
| `--type-small` | 0.875 | Caption, footnote |
| `--type-code` | 0.9375 | Inline code |

Line height: 1.5 for body, 1.2 for headings.

## Source

Hosted via Google Fonts CDN with `display=swap`. Self-host if/when the landing page sees production traffic.
