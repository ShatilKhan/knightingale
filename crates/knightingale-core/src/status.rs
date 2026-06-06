use tracing::info;

#[derive(Debug, Clone)]
pub enum Status {
    Recording,
    Transcribing,
    Done,
    Cancelled,
    Failed(String),
}

impl Status {
    /// Microcopy strings, defined in `design-system/cli/style-guide.md`.
    pub fn microcopy(&self) -> String {
        match self {
            Status::Recording => "Recording…".into(),
            Status::Transcribing => "Transcribing…".into(),
            Status::Done => "✓ Done".into(),
            Status::Cancelled => "Cancelled".into(),
            Status::Failed(reason) => {
                format!("✗ Failed: {reason}. Run `knightingale doctor`.")
            }
        }
    }
}

/// Dispatch a status update to the OS-native notification system, falling back
/// to stderr via `tracing` when no notification daemon is available.
pub fn notify(status: &Status) {
    let body = status.microcopy();
    info!(?status, "{body}");
    let _ = native_notify(&body);
}

#[cfg(target_os = "linux")]
fn native_notify(body: &str) -> Result<(), Box<dyn std::error::Error>> {
    notify_rust::Notification::new()
        .summary("Knightingale")
        .body(body)
        .show()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn native_notify(body: &str) -> Result<(), Box<dyn std::error::Error>> {
    use winrt_notification::Toast;
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title("Knightingale")
        .text1(body)
        .show()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn native_notify(body: &str) -> Result<(), Box<dyn std::error::Error>> {
    mac_notification_sys::send_notification("Knightingale", None, body, None)?;
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn native_notify(_body: &str) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microcopy_strings() {
        assert_eq!(Status::Recording.microcopy(), "Recording…");
        assert_eq!(Status::Transcribing.microcopy(), "Transcribing…");
        assert_eq!(Status::Done.microcopy(), "✓ Done");
        assert_eq!(Status::Cancelled.microcopy(), "Cancelled");
        assert_eq!(
            Status::Failed("auth".into()).microcopy(),
            "✗ Failed: auth. Run `knightingale doctor`."
        );
    }
}
