# CLI style guide

Terminal output conventions for `knightingale` and `knightingale-daemon`.

## Spacing

- Two-space indent everywhere. Tab characters never.
- Blank line between sections inside multi-section output (e.g. `doctor`, `config show`).
- Tables rendered with `comfy-table` `UTF8_FULL` preset.

## Colour budget

| State | Token | Glyph |
|---|---|---|
| Accent | `forest` | — |
| Success | `moss` | `✓` |
| Warning | `amber` | `!` |
| Failure | `coral` | `✗` |
| Dim | `sage` | — |

Never set background colours. `--no-emoji` swaps glyphs for plain ASCII (`+`, `!`, `-`).

## Microcopy

Exact strings used by `status::Dispatcher`:

| Event | String |
|---|---|
| Recording started | `Recording…` |
| Recording stopped, awaiting STT | `Transcribing…` |
| Success | `✓ Done` |
| Cancelled mid-flight | `Cancelled` |
| Failure | `✗ Failed: <reason>. Run \`knightingale doctor\`.` |

## Banner

Banner appears on `knightingale setup` welcome and `knightingale --version` only. Never on every invocation. Two variants:

- `banner.txt` — ~40×16, used by both subcommands.
- `banner-wide.txt` — ~60×24, optional `--banner=wide` opt-in.

## Timing footers

Long-running operations end with a single line in `sage` colour:

```
Done in 1.24s
```

## Errors

`miette::GraphicalReportHandler` for user-facing errors. Help line points at `doctor` or the specific `config set …` command.
