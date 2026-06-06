//! OS-aware setup helpers: compositor detection, permission checks, service
//! installation.

use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Compositor {
    Gnome,
    Kde,
    Hyprland,
    Sway,
    I3,
    X11,
    Unknown,
}

impl Compositor {
    pub fn as_str(self) -> &'static str {
        match self {
            Compositor::Gnome => "gnome",
            Compositor::Kde => "kde",
            Compositor::Hyprland => "hyprland",
            Compositor::Sway => "sway",
            Compositor::I3 => "i3",
            Compositor::X11 => "x11",
            Compositor::Unknown => "unknown",
        }
    }
}

/// Detect the current desktop session.
pub fn detect_compositor() -> Compositor {
    if !cfg!(target_os = "linux") {
        return Compositor::Unknown;
    }

    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return Compositor::Hyprland;
    }
    if std::env::var("SWAYSOCK").is_ok() {
        return Compositor::Sway;
    }
    if let Ok(desk) = std::env::var("XDG_CURRENT_DESKTOP") {
        let d = desk.to_lowercase();
        if d.contains("gnome") {
            return Compositor::Gnome;
        }
        if d.contains("kde") {
            return Compositor::Kde;
        }
        if d.contains("hyprland") {
            return Compositor::Hyprland;
        }
        if d.contains("sway") {
            return Compositor::Sway;
        }
        if d.contains("i3") {
            return Compositor::I3;
        }
    }
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        return Compositor::X11;
    }
    Compositor::Unknown
}

/// Suggested hotkey-binding command for the detected compositor.
///
/// The CLI prints this so the user can paste it into their config. For
/// reversible compositors (GNOME/Hyprland/KDE) the same command also actually
/// writes the binding when run with --apply.
pub fn hotkey_command(comp: Compositor, binding: &str, exec: &str) -> Option<String> {
    match comp {
        Compositor::Hyprland => Some(format!("hyprctl keyword bind \"{binding}, exec, {exec}\"")),
        Compositor::Gnome => Some(format!(
            "# Add a custom keybinding to GNOME:\n\
             gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings \"['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/knightingale/']\"\n\
             gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/knightingale/ name 'Knightingale'\n\
             gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/knightingale/ command '{exec}'\n\
             gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/knightingale/ binding '{binding}'"
        )),
        Compositor::Kde => Some(format!(
            "kwriteconfig5 --file kglobalshortcutsrc --group knightingale --key toggle \"{binding},none,Knightingale toggle\""
        )),
        Compositor::Sway | Compositor::I3 => Some(format!(
            "# Append to ~/.config/{}/config:\nbindsym {binding} exec {exec}",
            if comp == Compositor::Sway {
                "sway"
            } else {
                "i3"
            }
        )),
        Compositor::X11 | Compositor::Unknown => None,
    }
}

/// On Linux, check whether the current user can write to `/dev/uinput`.
#[cfg(target_os = "linux")]
pub fn uinput_writable() -> bool {
    use std::os::unix::fs::PermissionsExt;
    let path = std::path::Path::new("/dev/uinput");
    if !path.exists() {
        return false;
    }
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let mode = meta.permissions().mode();
    if mode & 0o002 != 0 {
        return true;
    }
    // Check group membership when group-write bit is set.
    Command::new("id")
        .arg("-nG")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.split_whitespace().any(|g| g == "input"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
pub fn uinput_writable() -> bool {
    true
}

/// macOS: open System Settings to the right pane via x-apple.systempreferences:.
#[cfg(target_os = "macos")]
pub fn open_macos_settings(pane: MacosPane) -> std::io::Result<()> {
    let url = pane.url();
    Command::new("open").arg(url).status()?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy)]
pub enum MacosPane {
    Accessibility,
    InputMonitoring,
    Microphone,
}

#[cfg(target_os = "macos")]
impl MacosPane {
    pub fn url(self) -> &'static str {
        match self {
            MacosPane::Accessibility => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            MacosPane::InputMonitoring => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
            }
            MacosPane::Microphone => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
            }
        }
    }
}
