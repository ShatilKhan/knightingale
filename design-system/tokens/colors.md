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
| `lichen` | `#BBDF8E` | 150 | Luminous accent on dark surfaces — active nav indicators, link hover on duotone photo sections (web only; too low-contrast for terminal) |
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

## Photo duotone rule (landing page)

Full-bleed photography is always duotone-mapped into the forest family — never
the raw photo, never a cool/teal wash. One hue axis, progressive depth as the
page scrolls (Canva draft's lavender→teal drift was reconciled to this ramp):

| Section | shadow → mid → highlight |
|---|---|
| 1 (hero) | `#14271C` → `#6B7A66` (sage) → `#F4F1E8` (cream) |
| 2 | `#14271C` → `#5C8052` (moss) → `#E8E2D0` |
| 3 | `#0F1E16` → `#4A6B50` → `#C9D2BE` |
| 4 | `#0C1810` → `#3A5740` → `#A8B8A0` |

Text over photos is `cream` on a soft `ink` scrim. Active page-dot indicator is
an elongated `lichen` pill; inactive dots are `cream` at 40% opacity.
Regenerate with `ImageOps.colorize` from the originals in
`brand/landing-draft/birds/`.

## Photo-section methodology (researched 2026-06-07)

Three rules, applied to every full-bleed photo section:

1. **Atmosphere, not wallpaper.** Photos never render at 100% opacity. Grade
   `0.68 → 0.50` from hero to footer over the `forest-3` base — the photo is
   mood, the content is the subject. (Pattern: hermes-agent.nousresearch.com.)
2. **Scrim discipline.** Flat ink scrim at ≥35% under any text, plus a radial
   focus scrim behind content blocks. Cream-on-photo text must hold WCAG
   contrast. (NN/g, Smashing Magazine text-over-images guidance.)
3. **One texture language.** The halftone dot field appears on *every*
   section (~12% cream, 14px grid), mask focal point varied per section so it
   reads organic, not stamped. A full-page film grain (~4.5%) unifies the
   sections. Texture used once is a glitch; texture used everywhere is a
   brand. (Biophilic-design tactile-depth principle.)
