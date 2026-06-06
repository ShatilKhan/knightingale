<p align="center">
  <img src="design-system/brand/knitingale-icon/1-readme.png" width="280" alt="Knightingale">
</p>

<h1 align="center">Knightingale</h1>

<p align="center">
  Voice dictation that minds its own business.
</p>

<p align="center">
  <a href="https://github.com/ShatilKhan/knightingale/stargazers"><img alt="stars" src="https://img.shields.io/github/stars/ShatilKhan/knightingale?style=flat-square&color=2D4A3E"></a>
  <a href="https://github.com/ShatilKhan/knightingale/network/members"><img alt="forks" src="https://img.shields.io/github/forks/ShatilKhan/knightingale?style=flat-square&color=2D4A3E"></a>
  <a href="https://github.com/ShatilKhan/knightingale/watchers"><img alt="watchers" src="https://img.shields.io/github/watchers/ShatilKhan/knightingale?style=flat-square&color=2D4A3E"></a>
  <a href="https://github.com/ShatilKhan/knightingale/releases/latest"><img alt="release" src="https://img.shields.io/github/v/release/ShatilKhan/knightingale?style=flat-square&color=5C8052"></a>
  <a href="https://github.com/ShatilKhan/knightingale/releases"><img alt="downloads" src="https://img.shields.io/github/downloads/ShatilKhan/knightingale/total?style=flat-square&color=5C8052"></a>
  <a href="https://github.com/ShatilKhan/knightingale/actions/workflows/ci.yml"><img alt="ci" src="https://img.shields.io/github/actions/workflow/status/ShatilKhan/knightingale/ci.yml?style=flat-square"></a>
  <a href="LICENSE"><img alt="license" src="https://img.shields.io/github/license/ShatilKhan/knightingale?style=flat-square"></a>
  <img alt="repo size" src="https://img.shields.io/github/repo-size/ShatilKhan/knightingale?style=flat-square">
  <img alt="hits" src="https://hits.sh/github.com/ShatilKhan/knightingale.svg?style=flat-square&color=2D4A3E">
</p>

---

A minimal voice dictation daemon for Linux, Windows, and macOS. A single static binary listens for a global hotkey, records your microphone, transcribes either through a cloud STT API you choose or a Whisper model running locally, and types the result into whatever window has focus.

## Install

```sh
curl -fsSL https://shatilkhan.github.io/knightingale/install.sh | sh
```

Then:

```sh
knightingale setup    # pick Cloud (BYOK) or Local, set hotkey
knightingale toggle   # start dictating
```

The default hotkey is **Super+K** on Linux/Windows and **Cmd+Shift+K** on macOS. Hit it to start, hit it again to stop and type.

Alternatives:

```sh
cargo install knightingale                     # if you have Rust installed
brew install ShatilKhan/tap/knightingale       # macOS (later)
scoop install knightingale                     # Windows (later)
```

## Why

- **xhisper** (~5 MB, bash + C) is Linux-only and needs shell glue.
- **openwhispr** (~300 MB RAM, Electron) is feature-rich but heavy.
- **WisprFlow / SuperWhisper** are closed-source SaaS.

Knightingale picks the middle: ~15–30 MB binary, ~10–40 MB RAM idle, cross-platform, BYOK + local STT, no telemetry, MIT licensed.

## Cloud providers (bring your own key)

One config field selects which credential block to read. Eight named providers share the same `/v1/audio/transcriptions` HTTP shape; a `custom` provider plus a `local` whisper.cpp backend cover everything else.

| Provider | env prefix | default model |
|---|---|---|
| `groq` *(default)* | `GROQ_*` | `whisper-large-v3-turbo` |
| `openai` | `OPENAI_*` | `whisper-1` |
| `deepinfra` | `DEEPINFRA_*` | `openai/whisper-large-v3-turbo` |
| `fireworks` | `FIREWORKS_*` | `whisper-v3-turbo` |
| `lemonfox` | `LEMONFOX_*` | `whisper-1` |
| `sambanova` | `SAMBANOVA_*` | `Whisper-Large-v3` |
| `azure` | `AZURE_OPENAI_*` | per-deployment |
| `custom` | `CUSTOM_BASE_URL`, `CUSTOM_API_KEY`, `CUSTOM_MODEL` | self-hosted Speaches / LocalAI / vLLM |
| `local` | `KNIGHTINGALE_MODEL_PATH` | `distil-small.en` (Phase 2) |

API keys live in `~/.config/knightingale/.env` (mode 0600). The TOML config beside it is safe to share.

## Local models *(Phase 2)*

Pull a model once, run it forever:

```sh
knightingale model pull distil-small.en
knightingale model recommend       # spec-aware pick based on your hardware
```

`knightingale model list` shows the catalog. Sizes range from `tiny.en` (~32 MB) to `large-v3` (~1.1 GB Q5). The default `distil-small.en` is real-time on CPU and fits 4 GB VRAM laptops; `large-v3-turbo` is the sweet spot for RTX-class GPUs.

## Privacy

Knightingale does not collect or transmit telemetry of any kind. No usage metrics, no error reports, no analytics, no phone-home. Your audio, transcripts, and configuration stay on your machine unless you explicitly point Knightingale at a cloud provider — in which case audio is sent **only** to the API endpoint configured in your `.env`, with your key, and nowhere else. No proxy, no cache, no logging by us.

## Commands

| | |
|---|---|
| `knightingale toggle` | Start / stop a recording |
| `knightingale status` | Print daemon state |
| `knightingale doctor` | Diagnose problems |
| `knightingale setup` | Re-run the interactive install flow |
| `knightingale logs [--since 1h] [--path]` | Tail the daemon log |
| `knightingale config show` | Print config (secrets redacted) |
| `knightingale config edit` | Open `config.toml` in `$EDITOR` |
| `knightingale config edit-secrets` | Open `.env` in `$EDITOR` |
| `knightingale config set <key> <value>` | Set a config field |
| `knightingale config set-key <provider>` | Hidden prompt for an API key |
| `knightingale config test` | Verify the active provider |
| `knightingale config provider list` | Print providers + defaults |
| `knightingale config mic list` / `set <name>` | Pick the input device |
| `knightingale banner [--wide]` | Print the ASCII bird |

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full design. In one paragraph: a Cargo workspace splits the code into `knightingale-core` (audio, STT, IPC, config, status), a thin `knightingale-daemon` that pulls those modules together inside a `std::thread` loop, and a `knightingale` CLI client that talks to the daemon over a local socket. No async runtime. No tokio. No Electron.

## Documentation

Documentation site: [shatilkhan.github.io/knightingale](https://shatilkhan.github.io/knightingale/) *(published from `docs/` after Phase 1 polish)*.

## Contributing

Issues and PRs welcome. The plan and decision log live in a separate context repo at [ShatilKhan/cuntxt](https://github.com/ShatilKhan/cuntxt) under `knightingale/PLAN.md` — read it before proposing a refactor.

## License

[MIT](LICENSE).
