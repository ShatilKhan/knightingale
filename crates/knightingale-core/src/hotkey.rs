use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyEventReceiver, GlobalHotKeyManager};
use tracing::{info, warn};

use crate::error::{KnightError, Result};

/// Default fallback chains per platform. Tried in order until one registers.
pub fn fallback_chain() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &[
            "cmd+shift+k",
            "cmd+shift+semicolon",
            "cmd+shift+slash",
            "cmd+option+space",
        ]
    } else {
        &[
            "super+k",
            "super+semicolon",
            "super+slash",
            "super+backslash",
            "ctrl+alt+space",
        ]
    }
}

pub fn default_binding() -> &'static str {
    fallback_chain()[0]
}

/// Parse a binding string like `"super+k"` or `"ctrl+alt+space"` into a HotKey.
pub fn parse(binding: &str) -> Result<HotKey> {
    let parts: Vec<&str> = binding
        .split('+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(KnightError::Hotkey(format!("empty binding: {binding}")));
    }
    let mut mods = Modifiers::empty();
    let mut key: Option<Code> = None;
    for tok in parts {
        let low = tok.to_ascii_lowercase();
        match low.as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "super" | "win" | "windows" | "meta" | "cmd" | "command" => mods |= Modifiers::SUPER,
            other => {
                key = Some(parse_key(other).ok_or_else(|| {
                    KnightError::Hotkey(format!("unknown key in binding: {other}"))
                })?);
            }
        }
    }
    let key = key.ok_or_else(|| KnightError::Hotkey(format!("no key in binding: {binding}")))?;
    Ok(HotKey::new(Some(mods), key))
}

fn parse_key(s: &str) -> Option<Code> {
    let upper = s.to_ascii_uppercase();
    match upper.as_str() {
        "SPACE" => Some(Code::Space),
        "ENTER" | "RETURN" => Some(Code::Enter),
        "ESC" | "ESCAPE" => Some(Code::Escape),
        "TAB" => Some(Code::Tab),
        "BACKSPACE" => Some(Code::Backspace),
        "SEMICOLON" | ";" => Some(Code::Semicolon),
        "SLASH" | "/" => Some(Code::Slash),
        "BACKSLASH" | "\\" => Some(Code::Backslash),
        "PERIOD" | "." => Some(Code::Period),
        "COMMA" | "," => Some(Code::Comma),
        "QUOTE" | "'" => Some(Code::Quote),
        "F1" => Some(Code::F1),
        "F2" => Some(Code::F2),
        "F3" => Some(Code::F3),
        "F4" => Some(Code::F4),
        "F5" => Some(Code::F5),
        "F6" => Some(Code::F6),
        "F7" => Some(Code::F7),
        "F8" => Some(Code::F8),
        "F9" => Some(Code::F9),
        "F10" => Some(Code::F10),
        "F11" => Some(Code::F11),
        "F12" => Some(Code::F12),
        // Single character a-z or 0-9.
        single if single.len() == 1 => {
            let c = single.chars().next().unwrap();
            if c.is_ascii_alphabetic() {
                let idx = c as u8 - b'A';
                match idx {
                    0 => Some(Code::KeyA),
                    1 => Some(Code::KeyB),
                    2 => Some(Code::KeyC),
                    3 => Some(Code::KeyD),
                    4 => Some(Code::KeyE),
                    5 => Some(Code::KeyF),
                    6 => Some(Code::KeyG),
                    7 => Some(Code::KeyH),
                    8 => Some(Code::KeyI),
                    9 => Some(Code::KeyJ),
                    10 => Some(Code::KeyK),
                    11 => Some(Code::KeyL),
                    12 => Some(Code::KeyM),
                    13 => Some(Code::KeyN),
                    14 => Some(Code::KeyO),
                    15 => Some(Code::KeyP),
                    16 => Some(Code::KeyQ),
                    17 => Some(Code::KeyR),
                    18 => Some(Code::KeyS),
                    19 => Some(Code::KeyT),
                    20 => Some(Code::KeyU),
                    21 => Some(Code::KeyV),
                    22 => Some(Code::KeyW),
                    23 => Some(Code::KeyX),
                    24 => Some(Code::KeyY),
                    25 => Some(Code::KeyZ),
                    _ => None,
                }
            } else if c.is_ascii_digit() {
                match c {
                    '0' => Some(Code::Digit0),
                    '1' => Some(Code::Digit1),
                    '2' => Some(Code::Digit2),
                    '3' => Some(Code::Digit3),
                    '4' => Some(Code::Digit4),
                    '5' => Some(Code::Digit5),
                    '6' => Some(Code::Digit6),
                    '7' => Some(Code::Digit7),
                    '8' => Some(Code::Digit8),
                    '9' => Some(Code::Digit9),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Wrap a `GlobalHotKeyManager` and remember which binding succeeded so we can
/// unregister cleanly on shutdown.
pub struct HotkeyHandle {
    manager: GlobalHotKeyManager,
    pub binding: String,
    pub hotkey: HotKey,
}

impl HotkeyHandle {
    /// Register `requested`. If it fails, walk the fallback chain and return
    /// whichever binding actually succeeded.
    pub fn register_with_fallback(requested: &str) -> Result<Self> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| KnightError::Hotkey(format!("init manager: {e}")))?;
        let chain = std::iter::once(requested).chain(fallback_chain().iter().copied());
        let mut last_err = String::new();
        for binding in chain {
            match parse(binding) {
                Ok(hk) => match manager.register(hk) {
                    Ok(()) => {
                        if binding != requested {
                            warn!(?requested, picked = binding, "hotkey fallback engaged");
                        } else {
                            info!(binding, "hotkey registered");
                        }
                        return Ok(Self {
                            manager,
                            binding: binding.to_string(),
                            hotkey: hk,
                        });
                    }
                    Err(e) => last_err = format!("{binding}: {e}"),
                },
                Err(e) => last_err = e.to_string(),
            }
        }
        Err(KnightError::Hotkey(format!(
            "all fallback bindings failed: {last_err}"
        )))
    }

    pub fn receiver() -> &'static GlobalHotKeyEventReceiver {
        GlobalHotKeyEvent::receiver()
    }
}

impl Drop for HotkeyHandle {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.hotkey);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_super_k() {
        let hk = parse("super+k").unwrap();
        assert!(hk.mods.contains(Modifiers::SUPER));
        assert_eq!(hk.key, Code::KeyK);
    }

    #[test]
    fn parse_ctrl_alt_space() {
        let hk = parse("ctrl+alt+space").unwrap();
        assert!(hk.mods.contains(Modifiers::CONTROL));
        assert!(hk.mods.contains(Modifiers::ALT));
        assert_eq!(hk.key, Code::Space);
    }

    #[test]
    fn fallback_chain_nonempty() {
        assert!(!fallback_chain().is_empty());
        assert_eq!(default_binding(), fallback_chain()[0]);
    }
}
