use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use knightingale_core::audio::{self, Recording, TARGET_SAMPLE_RATE};
use knightingale_core::config::{Config, SttBackend, runtime_socket, state_dir};
use knightingale_core::error::{KnightError, Result};
use knightingale_core::hotkey::HotkeyHandle;
use knightingale_core::injection;
use knightingale_core::ipc::{self, Listener, Replier, Request, Response};
use knightingale_core::status::{Status, notify};
use knightingale_core::stt::{Provider, Transcriber, build_transcriber};
use tracing::{error, info, warn};
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() -> miette::Result<()> {
    let _guard = init_logging();
    knightingale_core::load_env();

    let cfg = Config::load().map_err(miette::Report::from)?;
    info!(
        hotkey = %cfg.hotkey.toggle,
        backend = ?cfg.stt.backend,
        "starting daemon"
    );

    if let Err(e) = run(cfg) {
        error!(error = %e, "fatal");
        return Err(miette::Report::from(e));
    }
    Ok(())
}

fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("knightingale=info,knightingale_core=info,knightingale_daemon=info")
    });

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_writer(std::io::stderr);

    if let Ok(dir) = state_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let appender = rolling::daily(&dir, "daemon.log");
        let (writer, guard) = tracing_appender::non_blocking(appender);
        let file_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_ansi(false)
            .with_writer(writer);
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(stderr_layer)
            .with(file_layer)
            .try_init();
        install_panic_hook();
        return Some(guard);
    }
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer)
        .try_init();
    install_panic_hook();
    None
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "<unknown>".into());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<unknown panic>");
        error!(location, payload, "daemon panic");
    }));
}

fn run(cfg: Config) -> Result<()> {
    let _socket_path = runtime_socket()?;
    let listener = ipc::bind_listener()?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_signal = stop.clone();
    ctrlc::set_handler(move || {
        warn!("received shutdown signal");
        stop_for_signal.store(true, Ordering::SeqCst);
    })
    .map_err(|e| KnightError::Other(format!("ctrlc handler: {e}")))?;

    let hotkey = HotkeyHandle::register_with_fallback(&cfg.hotkey.toggle)?;
    info!(actual = %hotkey.binding, "hotkey active");

    let mut session = Session::new(cfg);
    let hotkey_rx = HotkeyHandle::receiver();
    let (ipc_tx, ipc_rx) = crossbeam_channel::unbounded::<(Request, Replier)>();

    spawn_ipc_thread(listener, stop.clone(), ipc_tx);

    while !stop.load(Ordering::SeqCst) {
        crossbeam_channel::select! {
            recv(hotkey_rx) -> ev => {
                if ev.is_ok() {
                    session.toggle();
                }
            }
            recv(ipc_rx) -> msg => {
                if let Ok((req, replier)) = msg {
                    handle_request(req, replier, &mut session, &stop);
                }
            }
            default(Duration::from_millis(100)) => {
                session.tick();
            }
        }
    }

    info!("shutting down");
    session.cancel();
    drop(hotkey);
    Ok(())
}

fn spawn_ipc_thread(
    listener: Listener,
    stop: Arc<AtomicBool>,
    tx: crossbeam_channel::Sender<(Request, Replier)>,
) {
    thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((req, replier)) => {
                    if tx.send((req, replier)).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    warn!(error = %e, "ipc accept");
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }
    });
}

fn handle_request(req: Request, replier: Replier, session: &mut Session, stop: &Arc<AtomicBool>) {
    let resp = match req {
        Request::Toggle => {
            session.toggle();
            Response::ok()
        }
        Request::Status => Response::ok_msg(session.status_label()),
        Request::SetHotkey { binding: _ } => Response::err("set_hotkey not yet implemented"),
        Request::Shutdown => {
            stop.store(true, Ordering::SeqCst);
            Response::ok_msg("shutting down")
        }
    };
    if let Err(e) = replier.reply(&resp) {
        warn!(error = %e, "ipc reply");
    }
}

enum SessionState {
    Idle,
    Recording { rec: Recording, started: Instant },
    Transcribing,
}

struct Session {
    cfg: Config,
    state: SessionState,
}

impl Session {
    fn new(cfg: Config) -> Self {
        Self {
            cfg,
            state: SessionState::Idle,
        }
    }

    fn status_label(&self) -> String {
        match &self.state {
            SessionState::Idle => "idle".into(),
            SessionState::Recording { .. } => "recording".into(),
            SessionState::Transcribing => "transcribing".into(),
        }
    }

    fn tick(&mut self) {
        // Enforce hard cap so a stuck recording doesn't burn forever.
        if let SessionState::Recording { started, .. } = &self.state {
            let elapsed = started.elapsed();
            if elapsed.as_secs() >= self.cfg.stt.max_recording_secs {
                warn!("hit hard cap; auto-stopping");
                self.stop_and_transcribe();
            }
        }
    }

    fn toggle(&mut self) {
        match std::mem::replace(&mut self.state, SessionState::Idle) {
            SessionState::Idle => self.start(),
            SessionState::Recording { rec, .. } => {
                self.state = SessionState::Recording {
                    rec,
                    started: Instant::now(),
                };
                self.stop_and_transcribe();
            }
            SessionState::Transcribing => {
                // Toggle during transcribe = cancel.
                notify(&Status::Cancelled);
                self.state = SessionState::Idle;
            }
        }
    }

    fn start(&mut self) {
        match audio::start_recording(self.cfg.audio.mic.as_deref()) {
            Ok(rec) => {
                notify(&Status::Recording);
                self.state = SessionState::Recording {
                    rec,
                    started: Instant::now(),
                };
            }
            Err(e) => {
                warn!(error = %e, "start recording");
                notify(&Status::Failed(format!("audio: {e}")));
                self.state = SessionState::Idle;
            }
        }
    }

    fn stop_and_transcribe(&mut self) {
        let prev = std::mem::replace(&mut self.state, SessionState::Transcribing);
        let SessionState::Recording { rec, .. } = prev else {
            self.state = SessionState::Idle;
            return;
        };
        notify(&Status::Transcribing);
        match rec.stop() {
            Ok(samples) => {
                let trimmed = audio::trim_edge_silence(&samples, self.cfg.audio.silence_threshold);
                if trimmed.is_empty() {
                    notify(&Status::Done);
                    self.state = SessionState::Idle;
                    return;
                }
                match self.transcribe_and_inject(trimmed) {
                    Ok(()) => notify(&Status::Done),
                    Err(e) => notify(&Status::Failed(e.to_string())),
                }
            }
            Err(e) => notify(&Status::Failed(e.to_string())),
        }
        self.state = SessionState::Idle;
    }

    fn transcribe_and_inject(&self, samples: &[i16]) -> Result<()> {
        let wav = audio::pcm_to_wav(samples)?;
        let text = if self.cfg.stt.backend == SttBackend::Local {
            self.transcribe_local(&wav)?
        } else {
            self.transcribe_cloud(&wav)?
        };
        if text.is_empty() {
            return Ok(());
        }
        info!(chars = text.len(), "injecting transcript");
        injection::inject(&text, self.cfg.injection.method)?;
        let _ = TARGET_SAMPLE_RATE;
        Ok(())
    }

    fn transcribe_cloud(&self, wav: &[u8]) -> Result<String> {
        let provider = Provider::from_env();
        let mut transcriber: Box<dyn Transcriber> = build_transcriber(provider)?;
        // Vocab hints: prepend global + per-app hints as the `prompt` field.
        let prompt = knightingale_core::focus::build_prompt(
            &self.cfg.stt.vocabulary_hints,
            &self.cfg.stt.vocabulary_per_app,
        );
        if prompt.is_some()
            && let Ok(Some(mut client)) = provider.build_openai_client()
        {
            client = client.with_prompt(prompt);
            transcriber = Box::new(client);
        }
        transcriber.transcribe(wav, &self.cfg.stt.language)
    }

    #[cfg(feature = "local-stt")]
    fn transcribe_local(&self, wav: &[u8]) -> Result<String> {
        use knightingale_core::stt::LocalWhisper;
        let path = std::env::var("KNIGHTINGALE_MODEL_PATH").map_err(|_| {
            KnightError::ModelMissing(
                "KNIGHTINGALE_MODEL_PATH not set; run `knightingale model where <alias>`".into(),
            )
        })?;
        let path = std::path::PathBuf::from(shellexpand::tilde(&path).into_owned());
        let model = LocalWhisper::load(&path, Some(self.cfg.stt.language.clone()))?;
        model.transcribe(wav, &self.cfg.stt.language)
    }

    #[cfg(not(feature = "local-stt"))]
    fn transcribe_local(&self, _wav: &[u8]) -> Result<String> {
        Err(KnightError::ModelMissing(
            "this binary was built without the `local-stt` feature; \
             rebuild with `cargo build --features knightingale-daemon/local-stt` or use a cloud provider"
                .into(),
        ))
    }

    fn cancel(&mut self) {
        let prev = std::mem::replace(&mut self.state, SessionState::Idle);
        if !matches!(prev, SessionState::Idle) {
            notify(&Status::Cancelled);
        }
    }
}
