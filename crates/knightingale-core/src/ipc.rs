use std::io::{BufRead, BufReader, Write};

use interprocess::local_socket::traits::{ListenerExt, Stream as StreamTrait};
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, SendHalf, Stream, ToFsName, ToNsName,
};
use serde::{Deserialize, Serialize};

use crate::config::runtime_socket;
use crate::error::{KnightError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Toggle,
    Status,
    SetHotkey { binding: String },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "ok", rename_all = "snake_case")]
pub enum Response {
    Ok { message: Option<String> },
    Err { message: String },
}

impl Response {
    pub fn ok() -> Self {
        Response::Ok { message: None }
    }

    pub fn ok_msg(s: impl Into<String>) -> Self {
        Response::Ok { message: Some(s.into()) }
    }

    pub fn err(s: impl Into<String>) -> Self {
        Response::Err { message: s.into() }
    }
}

fn socket_name() -> Result<interprocess::local_socket::Name<'static>> {
    let path = runtime_socket()?;
    if cfg!(windows) {
        let s = path.to_string_lossy().to_string();
        s.to_ns_name::<GenericNamespaced>()
            .map(|n| n.into_owned())
            .map_err(|e| KnightError::Ipc(format!("ns name: {e}")))
    } else {
        let s = path.to_string_lossy().to_string();
        s.to_fs_name::<GenericFilePath>()
            .map(|n| n.into_owned())
            .map_err(|e| KnightError::Ipc(format!("fs name: {e}")))
    }
}

/// Send a single request to a running daemon and return the response.
pub fn send(req: &Request) -> Result<Response> {
    let name = socket_name()?;
    let stream = Stream::connect(name)
        .map_err(|e| KnightError::Ipc(format!("connect: {e}")))?;
    let (rx, mut tx) = stream.split();
    let line = serde_json::to_string(req)
        .map_err(|e| KnightError::Ipc(format!("serialize: {e}")))?;
    writeln!(tx, "{line}").map_err(|e| KnightError::Ipc(format!("write: {e}")))?;
    tx.flush().map_err(|e| KnightError::Ipc(format!("flush: {e}")))?;
    let mut reader = BufReader::new(rx);
    let mut buf = String::new();
    reader
        .read_line(&mut buf)
        .map_err(|e| KnightError::Ipc(format!("read: {e}")))?;
    serde_json::from_str(buf.trim())
        .map_err(|e| KnightError::Ipc(format!("parse response: {e}")))
}

/// Check whether a daemon is currently listening.
pub fn probe() -> bool {
    matches!(send(&Request::Status), Ok(_))
}

/// Try to bind the IPC socket. Returns Err if another daemon is running.
/// Cleans up a stale socket file (Unix) if no peer responded.
pub fn bind_listener() -> Result<Listener> {
    if probe() {
        return Err(KnightError::Ipc(
            "another knightingale-daemon is already running".into(),
        ));
    }
    #[cfg(unix)]
    {
        let path = runtime_socket()?;
        if path.exists() {
            // Stale socket; remove.
            let _ = std::fs::remove_file(&path);
        }
    }
    let name = socket_name()?;
    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()
        .map_err(|e| KnightError::Ipc(format!("bind: {e}")))?;
    Ok(Listener { inner: listener })
}

pub struct Listener {
    inner: interprocess::local_socket::Listener,
}

impl Listener {
    /// Iterate over incoming connections. Each yielded `(Request, Replier)`
    /// must be answered exactly once.
    pub fn accept(&self) -> Result<(Request, Replier)> {
        let mut iter = self.inner.incoming();
        let stream = iter
            .next()
            .ok_or_else(|| KnightError::Ipc("listener closed".into()))?
            .map_err(|e| KnightError::Ipc(format!("accept: {e}")))?;
        let (rx, tx) = stream.split();
        let mut reader = BufReader::new(rx);
        let mut buf = String::new();
        reader
            .read_line(&mut buf)
            .map_err(|e| KnightError::Ipc(format!("read: {e}")))?;
        let req: Request = serde_json::from_str(buf.trim())
            .map_err(|e| KnightError::Ipc(format!("parse: {e}")))?;
        Ok((req, Replier { tx }))
    }
}

pub struct Replier {
    tx: SendHalf,
}

impl Replier {
    pub fn reply(mut self, resp: &Response) -> Result<()> {
        let line = serde_json::to_string(resp)
            .map_err(|e| KnightError::Ipc(format!("serialize: {e}")))?;
        writeln!(self.tx, "{line}")
            .map_err(|e| KnightError::Ipc(format!("write: {e}")))?;
        self.tx
            .flush()
            .map_err(|e| KnightError::Ipc(format!("flush: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_json() {
        let req = Request::SetHotkey { binding: "super+k".into() };
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("set_hotkey"));
        assert!(s.contains("super+k"));
        let back: Request = serde_json::from_str(&s).unwrap();
        match back {
            Request::SetHotkey { binding } => assert_eq!(binding, "super+k"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn response_round_trips() {
        let r = Response::ok_msg("hi");
        let s = serde_json::to_string(&r).unwrap();
        let back: Response = serde_json::from_str(&s).unwrap();
        assert!(matches!(back, Response::Ok { message: Some(m) } if m == "hi"));
    }
}
