# Colour tokens

Eyedropped from the brand asset. Truecolor first, 256-colour fallback via `anstyle`.

| Token | Hex | ANSI 256 | Use |
|---|---|---|---|
| `forest` | `#1F3A2A` | 22 | Primary accent; logo ring; headings |
| `forest-2` | `#2D4A3E` | 23 | Hover/secondary accent |
| `sage` | `#6B7A66` | 65 | Dim text, secondary content |
| `sage-2` | `#8A9B82` | 108 | Tertiary text, captions |
| `cream` | `#F4F1E8` | 230 | Web surface; light-terminal foreground |
| `cream-2` | `#E8E2D0` | 223 | Web alt surface |
| `ink` | `#1A2622` | 235 | Body text on web |
| `moss` | `#5C8052` | 70 | Success state |
| `amber` | `#B8924A` | 137 | Warning state |
| `coral` | `#A45248` | 167 | Error state |

## CLI rules

- Never set background colours.
- Use `forest` only as foreground on success accents.
- Status states map to `moss` / `amber` / `coral`.
- Never use `forest` and `moss` together as foreground in the same line — too close in hue.

## Web rules

- Surface uses `cream`; body text uses `ink`.
- Primary accent on headings uses `forest`; links use `forest-2` underlined.
- Alt sections use `cream-2`.
