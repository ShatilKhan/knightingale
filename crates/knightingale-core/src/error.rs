use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, KnightError>;

#[derive(Debug, Error, Diagnostic)]
pub enum KnightError {
    #[error("authentication failed: {0}")]
    #[diagnostic(
        code(knightingale::auth),
        help("check your API key with `knightingale config show` or rerun `knightingale config set-key <provider>`")
    )]
    Auth(String),

    #[error("network error: {0}")]
    #[diagnostic(
        code(knightingale::network),
        help("confirm the provider endpoint is reachable; run `knightingale doctor`")
    )]
    Network(String),

    #[error("audio capture failed: {0}")]
    #[diagnostic(
        code(knightingale::audio),
        help("list mic devices with `knightingale config mic list` and confirm permissions; run `knightingale doctor`")
    )]
    Audio(String),

    #[error("permission denied: {0}")]
    #[diagnostic(
        code(knightingale::permission),
        help("on Linux confirm membership of the `input` group; on macOS grant Accessibility + Input Monitoring in System Settings")
    )]
    Permission(String),

    #[error("model not found: {0}")]
    #[diagnostic(
        code(knightingale::model_missing),
        help("install with `knightingale model pull <name>` or pick a different model with `knightingale config set model <name>`")
    )]
    ModelMissing(String),

    #[error("hotkey error: {0}")]
    #[diagnostic(
        code(knightingale::hotkey),
        help("set a different binding with `knightingale config set hotkey \"<binding>\"`")
    )]
    Hotkey(String),

    #[error("configuration error: {0}")]
    #[diagnostic(
        code(knightingale::config),
        help("inspect with `knightingale config show` or edit `~/.config/knightingale/config.toml`")
    )]
    Config(String),

    #[error("ipc error: {0}")]
    #[diagnostic(
        code(knightingale::ipc),
        help("another daemon may already be running; check with `knightingale status`")
    )]
    Ipc(String),

    #[error("{0}")]
    #[diagnostic(code(knightingale::other))]
    Other(String),
}

impl From<std::io::Error> for KnightError {
    fn from(e: std::io::Error) -> Self {
        KnightError::Other(e.to_string())
    }
}
