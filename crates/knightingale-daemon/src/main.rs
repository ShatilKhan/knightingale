#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use knightingale_core::audio::{self, Recording};
use knightingale_core::config::{Config, InjectionMethod, runtime_socket, state_dir};
use knightingale_core::error::{KnightError, Result};
use knightingale_core::hotkey::HotkeyHandle;
use knightingale_core::injection;
use knightingale_core::ipc::{self, Listener, Replier, Request, Response};
use knightingale_core::status::{Status, notify};
use knightingale_core::stt::{Provider, Transcriber, build_transcriber};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

fn main() -> miette::Result<()> {
    init_logging();
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

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("knightingale=info,knightingale_core=info,knightingale_daemon=info")
    });
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
    // Daemon log file writer is wired in a later commit alongside rotation.
    install_panic_hook();
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
    let _state_dir = state_dir().ok();
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
            default(Duration::from_millis(100)) => {}
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

struct Session {
    cfg: Config,
    state: SessionState,
}

enum SessionState {
    Idle,
    Recording(Recording),
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
            SessionState::Recording(_) => "recording".into(),
        }
    }

    fn toggle(&mut self) {
        // Real toggle / recording lifecycle / cap arrive in the next commit; this
        // scaffold lets the daemon build and respond to IPC before the audio +
        // STT pipeline is wired in.
        info!("toggle");
        notify(&Status::Recording);
    }

    fn cancel(&mut self) {
        if let SessionState::Recording(_) = std::mem::replace(&mut self.state, SessionState::Idle) {
            notify(&Status::Cancelled);
        }
    }
}

// Silence unused-warning until later commits hook them up.
#[allow(dead_code)]
fn _unused(
    _t: &dyn Transcriber,
    _p: Provider,
    _build: fn(Provider) -> Result<Box<dyn Transcriber>>,
    _inject: fn(&str, InjectionMethod) -> Result<()>,
    _trim: fn(&[i16], i16) -> &[i16],
    _wav: fn(&[i16]) -> Result<Vec<u8>>,
    _start: fn(Option<&str>) -> Result<Recording>,
) {
    let _ = build_transcriber;
    let _ = injection::inject;
    let _ = audio::trim_edge_silence;
    let _ = audio::pcm_to_wav;
    let _ = audio::start_recording;
}
