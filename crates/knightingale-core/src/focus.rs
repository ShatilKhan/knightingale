//! Focus-window detection for per-app vocabulary hints.
//!
//! Best-effort. Returns `None` when a backend isn't available.

#[cfg(target_os = "linux")]
pub fn focused_app() -> Option<String> {
    // Try xdotool first (X11 + XWayland). Then fall back to swaymsg / hyprctl.
    if let Some(name) = via_command("xdotool", &["getactivewindow", "getwindowname"]) {
        return Some(name);
    }
    if let Some(name) = via_swaymsg() {
        return Some(name);
    }
    if let Some(name) = via_hyprctl() {
        return Some(name);
    }
    None
}

#[cfg(target_os = "linux")]
fn via_command(prog: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(prog).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

#[cfg(target_os = "linux")]
fn via_swaymsg() -> Option<String> {
    let out = std::process::Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    find_focused(&json).and_then(|v| {
        v.get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
    })
}

#[cfg(target_os = "linux")]
fn find_focused(node: &serde_json::Value) -> Option<&serde_json::Value> {
    if node.get("focused").and_then(|v| v.as_bool()) == Some(true) {
        return Some(node);
    }
    for arr in ["nodes", "floating_nodes"] {
        if let Some(arr) = node.get(arr).and_then(|v| v.as_array()) {
            for child in arr {
                if let Some(f) = find_focused(child) {
                    return Some(f);
                }
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn via_hyprctl() -> Option<String> {
    let out = std::process::Command::new("hyprctl")
        .args(["-j", "activewindow"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    json.get("class")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

#[cfg(target_os = "macos")]
pub fn focused_app() -> Option<String> {
    let out = std::process::Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to name of first application process whose frontmost is true"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

#[cfg(target_os = "windows")]
pub fn focused_app() -> Option<String> {
    // Lightweight placeholder. Real impl uses GetForegroundWindow + GetWindowText.
    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn focused_app() -> Option<String> {
    None
}

/// Combine global hints + per-app hints for the currently focused window.
pub fn build_prompt(
    global: &[String],
    per_app: &std::collections::BTreeMap<String, Vec<String>>,
) -> Option<String> {
    let mut words: Vec<String> = global.to_vec();
    if let Some(app) = focused_app() {
        let key = app.to_lowercase();
        for (k, v) in per_app {
            if key.contains(&k.to_lowercase()) {
                words.extend(v.iter().cloned());
            }
        }
    }
    if words.is_empty() {
        None
    } else {
        Some(words.join(", "))
    }
}
