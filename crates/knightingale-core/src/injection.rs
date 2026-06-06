use enigo::{Enigo, Keyboard, Settings};
use tracing::warn;

use crate::config::InjectionMethod;
use crate::error::{KnightError, Result};

/// Type `text` into whatever window currently has focus.
pub fn inject(text: &str, method: InjectionMethod) -> Result<()> {
    match method {
        InjectionMethod::Auto => auto(text),
        InjectionMethod::Enigo => via_enigo(text),
        InjectionMethod::Uinput => via_uinput(text),
        InjectionMethod::ClipboardPaste => via_clipboard_paste(text),
    }
}

fn auto(text: &str) -> Result<()> {
    if let Err(e) = via_enigo(text) {
        warn!(error = %e, "enigo failed; falling back to clipboard paste");
        via_clipboard_paste(text)
    } else {
        Ok(())
    }
}

fn via_enigo(text: &str) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| KnightError::Permission(format!("enigo init: {e}")))?;
    enigo
        .text(text)
        .map_err(|e| KnightError::Permission(format!("enigo type: {e}")))?;
    Ok(())
}

fn via_clipboard_paste(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| KnightError::Permission(format!("clipboard: {e}")))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| KnightError::Permission(format!("clipboard set: {e}")))?;
    // Simulate Ctrl+V via enigo.
    use enigo::{Direction, Key};
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| KnightError::Permission(format!("enigo init: {e}")))?;
    let modifier = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };
    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| KnightError::Permission(format!("modifier press: {e}")))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| KnightError::Permission(format!("v key: {e}")))?;
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| KnightError::Permission(format!("modifier release: {e}")))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn via_uinput(text: &str) -> Result<()> {
    // Placeholder — uinput requires root or input group + writable /dev/uinput.
    // For Wayland environments this is the more reliable injection path; the
    // implementation is tracked separately. Fall back to enigo for now.
    warn!("uinput injection not yet implemented; falling back to enigo");
    via_enigo(text)
}

#[cfg(not(target_os = "linux"))]
fn via_uinput(_text: &str) -> Result<()> {
    Err(KnightError::Permission(
        "uinput is Linux-only".into(),
    ))
}
