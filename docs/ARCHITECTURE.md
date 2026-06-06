# Architecture

Knightingale is a small voice-dictation daemon plus a CLI client. The hard
choices are documented in
[`ShatilKhan/cuntxt → knightingale/PLAN.md`](https://github.com/ShatilKhan/cuntxt/blob/main/knightingale/PLAN.md);
this document is the read-the-code-with-a-map version.

## Workspace

```
knightingale/
├── Cargo.toml                # workspace root, resolver=3, edition=2024
├── crates/
│   ├── knightingale-core/    # library: config, audio, STT, IPC, status
│   ├── knightingale-daemon/  # bin: hotkey + recording + injection
│   └── knightingale-cli/     # bin: the `knightingale` command
├── design-system/            # brand, tokens, CLI banner, copy register
├── packaging/                # systemd unit, launchd plist, package recipes
├── install.sh                # one-line installer
└── .github/workflows/        # CI + release
```

`core` is the candidate for future `uniffi` mobile FFI; tests there don't pull
in daemon deps; the CLI builds fast (no whisper-rs needed for `toggle`).

## Concurrency model

`std::thread` plus `std::sync::mpsc` and `crossbeam-channel`. **No async
runtime.** Five long-lived threads with clean boundaries:

```
   ┌── global-hotkey ──────► [hotkey rx]
   │
   │   (cpal callback) ─────► samples_tx ──► collector worker ──► PCM
   │
   │   ipc Listener.accept ─► (Request, Replier) ──► [ipc rx]
   │
   ▼
   main loop ── crossbeam::select! over hotkey rx + ipc rx ──┐
                                                              │
                                                              ▼
                                      Session::toggle / Session::tick
                                              │
                                              ▼
                                Recording → trim → WAV → Transcriber → enigo
```

Cancellation is an `Arc<AtomicBool>`. Bounded shutdown (`SIGTERM`, Ctrl-C,
Windows service stop) is wired through `ctrlc` and drops in-flight work within
~2 s.

## Modules

### `knightingale-core::audio`
- `start_recording(mic)` builds a `cpal` input stream, picks the device's
  default config, downmixes to mono on the fly and ships chunks through a
  channel.
- A worker thread collects samples until stop, then resamples to 16 kHz with
  `rubato::SincFixedIn` if the device wasn't already at 16 kHz.
- `trim_edge_silence(samples, threshold)` strips the leading/trailing dead air
  that makes Whisper hallucinate "thank you for watching".
- `pcm_to_wav(samples)` writes a single-channel 16-bit WAV header via `hound`.

### `knightingale-core::stt`
- `Transcriber` trait: `fn transcribe(&self, wav: &[u8], language: &str) -> Result<String>`.
- `OpenAiClient` is a `reqwest::blocking` multipart POST to
  `{base_url}/audio/transcriptions`. Works for Groq, OpenAI, Deepinfra,
  Fireworks, Lemonfox, SambaNova, Custom — the URL-templating differences
  collapse into one client.
- `AzureClient` handles the deployment-keyed Azure variant
  (`{endpoint}/openai/deployments/{deployment}/audio/transcriptions?api-version=...`).
- `Provider` enum lives in `stt::provider`. `Provider::from_env()` reads
  `KNIGHTINGALE_PROVIDER`; `build_transcriber(p)` returns a `Box<dyn
  Transcriber>` after pulling env-var-based credentials.

### `knightingale-core::injection`
- `inject(text, method)` types `text` into the focused window. Backends:
  `enigo` (Windows / X11 / macOS), or clipboard-paste fallback via `arboard` +
  simulated `Ctrl+V` (Wayland-first; auto on Linux).
- `is_wayland()` checks `XDG_SESSION_TYPE` / `WAYLAND_DISPLAY` to choose the
  fallback by default.

### `knightingale-core::hotkey`
- `parse(binding)` accepts strings like `"super+k"`, `"cmd+shift+;"`,
  `"ctrl+alt+space"`.
- `fallback_chain()` returns per-OS candidates; `HotkeyHandle::register_with_fallback`
  walks them until one succeeds.
- The `global-hotkey` crate's global receiver feeds the daemon's main `select!`.

### `knightingale-core::ipc`
- One Unix socket / Windows named pipe per user
  (`$XDG_RUNTIME_DIR/knightingale.sock`,
  `\\.\pipe\knightingale`,
  `~/Library/Caches/knightingale/knightingale.sock`).
- Wire format: newline-delimited JSON, one request per connection.
- Request: `{"cmd": "toggle" | "status" | "set_hotkey" | "shutdown", ...}`.
- `bind_listener()` probes for a peer before binding so a second daemon refuses
  to start; stale Unix sockets are cleaned up.

### `knightingale-core::status`
- `Status` enum (`Recording`, `Transcribing`, `Done`, `Cancelled`,
  `Failed(reason)`). `microcopy()` returns the exact strings specified in
  `design-system/cli/style-guide.md`.
- `notify(&Status)` dispatches to `notify-rust` (Linux), `winrt-notification`
  (Windows), or `mac-notification-sys` (macOS). Falls back to a `tracing` line
  if no notification daemon is around.

### `knightingale-core::config`
- `Config` struct serialised as TOML.
- Loader uses `figment`: defaults → `~/.config/knightingale/config.toml` → env
  (`KNIGHTINGALE_*`, split on `__` for nested keys).
- All XDG path resolution is via the `directories` crate, so macOS gets
  `~/Library/Application Support/knightingale/` and Windows
  `%APPDATA%\knightingale\` for free.

### `knightingale-core::secret`
- API keys are wrapped in `secrecy::SecretString` so debug-logging them is a
  no-op.
- `load_env_file()` refuses to read a `.env` whose mode is world/group-readable.
- `set_in_env_file(var, value)` atomically rewrites the `.env`, restoring mode
  0600 after each edit (and the parent dir to 0700).
- `redact("gsk_1234567890abcdef42") -> "gsk_••••••••42"`.

### `knightingale-core::error`
- `enum KnightError { Auth, Network, Audio, Permission, ModelMissing, Hotkey, Config, Ipc, Other }`.
- Each variant carries a `miette::Diagnostic` with a `#[help(...)]` line
  pointing at the relevant `knightingale` subcommand.

### `knightingale-cli`
- `clap` derive root in `src/main.rs`. Subcommands dispatched into
  `src/commands.rs`.
- `src/style.rs` exposes palette constants from
  `design-system/tokens/tokens.rs` (re-exported through `knightingale-core::tokens`)
  and the ASCII banner via `include_str!`.
- Interactive flows (`setup`, `config set-key`) use `inquire`.
- Tables (`config show`, `provider list`, `mic list`, `doctor`) use
  `comfy-table` with the `UTF8_FULL` preset.

### `knightingale-daemon`
- `init_logging()` sets up `tracing-subscriber` with a stderr layer and (when
  the state dir is writable) a daily-rotated file layer via `tracing-appender`.
- A `std::panic` hook writes the location + payload through `tracing` so a
  crash always leaves a trace on disk.
- The `Session` state machine has three states: `Idle`,
  `Recording { rec, started }`, `Transcribing`. `toggle()` advances between
  them; `tick()` enforces the 5-minute hard cap as a failsafe.

## Privacy invariants

- The only network requests Knightingale ever makes are to the STT endpoint
  configured in the user's `.env`. Grep the codebase for `reqwest` to verify:
  there is exactly one HTTP client, owned by `stt::openai::OpenAiClient` (and
  its azure variant).
- No analytics, no crash reporting, no automatic update checks. `knightingale
  upgrade` is a thin wrapper that re-runs `install.sh` on demand.
- The panic hook writes only to the daemon log file; nothing leaves the
  machine.

## Distribution

- Releases are tagged `v0.x.y`. CI builds five targets
  (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
  `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`),
  packages each as `knightingale-<target>.tar.gz`, and produces a
  cosign-signed `SHA256SUMS` alongside.
- `install.sh` defaults to verifying that checksum file via the sigstore
  bundle (`--no-verify` to skip).
- macOS binaries are unsigned for now; `install.sh` runs `xattr -d
  com.apple.quarantine` after download so Gatekeeper allows them on first
  launch. Notarisation is deferred until there is real demand or a paid Apple
  Developer Program seat.

## See also

- `~/dev/cuntxt/knightingale/PLAN.md` — the rolling design doc
  (Plan / Architecture / Decisions / Updates).
- `~/dev/cuntxt/knightingale/LEARNING.md` — curated reading list.
- `design-system/cli/style-guide.md` — terminal colour rules, spacing,
  microcopy.
