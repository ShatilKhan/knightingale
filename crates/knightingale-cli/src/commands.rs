use std::process::Command;

use comfy_table::Table;
use comfy_table::presets::UTF8_FULL;
use inquire::{Confirm, Password, PasswordDisplayMode, Select, Text};
use knightingale_core::audio;
use knightingale_core::config::{Config, SttBackend, config_file, env_file};
use knightingale_core::error::KnightError;
use knightingale_core::secret::{SecretString, redact, set_in_env_file};
use knightingale_core::stt::Provider;

pub fn logs(since: Option<String>, path_only: bool) -> miette::Result<()> {
    let dir = knightingale_core::config::state_dir().map_err(miette::Report::from)?;
    let log_path = dir.join("daemon.log");
    if path_only {
        println!("{}", log_path.display());
        return Ok(());
    }
    let body = std::fs::read_to_string(&log_path)
        .unwrap_or_else(|_| format!("(no log at {})", log_path.display()));
    let since = since.and_then(|s| parse_duration(&s));
    if let Some(_d) = since {
        // Cheap line filter: keep the last 200 lines; tracing timestamps are
        // not a stable parseable format across subscribers. A real time filter
        // is a Phase 4 nicety.
        let lines: Vec<_> = body.lines().collect();
        let take = lines.len().min(200);
        for line in &lines[lines.len() - take..] {
            println!("{line}");
        }
    } else {
        print!("{body}");
    }
    Ok(())
}

fn parse_duration(s: &str) -> Option<std::time::Duration> {
    let s = s.trim();
    let (n, unit) = s.split_at(s.len().saturating_sub(1));
    let n: u64 = n.parse().ok()?;
    let secs = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        _ => return None,
    };
    Some(std::time::Duration::from_secs(secs))
}

pub fn doctor() -> miette::Result<()> {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["check", "status", "detail"]);

    let cfg = Config::load().map_err(miette::Report::from)?;
    table.add_row(vec![
        "config",
        "ok",
        &config_file().unwrap().display().to_string(),
    ]);
    table.add_row(vec![
        "env file",
        "ok",
        &env_file().unwrap().display().to_string(),
    ]);

    match audio::list_input_devices() {
        Ok(devices) => {
            let names = if devices.is_empty() {
                "(none)".into()
            } else {
                devices.join(", ")
            };
            table.add_row(vec!["mic devices", "ok", &names]);
        }
        Err(e) => {
            table.add_row(vec!["mic devices", "fail", &e.to_string()]);
        }
    }

    let provider = Provider::from_env();
    table.add_row(vec!["provider", "info", provider.as_str()]);
    if provider != Provider::Local {
        let key_var = format!("{}_API_KEY", provider.env_prefix());
        let present = std::env::var(&key_var).is_ok();
        table.add_row(vec![
            "api key",
            if present { "ok" } else { "missing" },
            &key_var,
        ]);
    }

    table.add_row(vec!["hotkey", "info", &cfg.hotkey.toggle]);
    table.add_row(vec![
        "max recording",
        "info",
        &format!("{}s", cfg.stt.max_recording_secs),
    ]);

    println!("{table}");
    Ok(())
}

pub fn setup() -> miette::Result<()> {
    println!("{}\n", crate::style::BANNER);
    println!("Welcome to Knightingale.\n");

    let backend_choice = Select::new(
        "How do you want to transcribe?",
        vec!["Cloud (BYOK)", "Local (whisper.cpp)"],
    )
    .prompt()
    .map_err(|e| miette::miette!(e.to_string()))?;

    let mut cfg = Config::load().unwrap_or_default();

    if backend_choice.starts_with("Cloud") {
        let providers = vec![
            "groq",
            "openai",
            "deepinfra",
            "fireworks",
            "lemonfox",
            "sambanova",
            "azure",
            "custom",
        ];
        let pick = Select::new("Pick a provider:", providers)
            .prompt()
            .map_err(|e| miette::miette!(e.to_string()))?;
        // SAFETY: setup runs single-threaded before the daemon starts.
        unsafe {
            std::env::set_var("KNIGHTINGALE_PROVIDER", pick);
        }
        cfg.stt.backend = SttBackend::OpenaiCompatible;
        let provider: Provider = match pick {
            "groq" => Provider::Groq,
            "openai" => Provider::Openai,
            "deepinfra" => Provider::Deepinfra,
            "fireworks" => Provider::Fireworks,
            "lemonfox" => Provider::Lemonfox,
            "sambanova" => Provider::Sambanova,
            "azure" => Provider::Azure,
            "custom" => Provider::Custom,
            _ => Provider::Groq,
        };
        let key_var = format!("{}_API_KEY", provider.env_prefix());
        let key = Password::new(&format!("Enter your {key_var}:"))
            .with_display_mode(PasswordDisplayMode::Masked)
            .without_confirmation()
            .prompt()
            .map_err(|e| miette::miette!(e.to_string()))?;
        if !key.is_empty() {
            set_in_env_file(&key_var, &SecretString::from(key)).map_err(miette::Report::from)?;
        }
        set_in_env_file(
            "KNIGHTINGALE_PROVIDER",
            &SecretString::from(pick.to_string()),
        )
        .map_err(miette::Report::from)?;
    } else {
        cfg.stt.backend = SttBackend::Local;
        println!("(local model setup wired in Phase 2; for now the daemon will fail when toggled)");
    }

    let default_hotkey = if cfg!(target_os = "macos") {
        "cmd+shift+k"
    } else {
        "super+k"
    };
    let hotkey = Text::new("Hotkey:")
        .with_default(default_hotkey)
        .prompt()
        .map_err(|e| miette::miette!(e.to_string()))?;
    cfg.hotkey.toggle = hotkey;
    cfg.save().map_err(miette::Report::from)?;

    let _ = Confirm::new("Run `knightingale doctor` now?")
        .with_default(true)
        .prompt();
    doctor()
}

pub fn config_show() -> miette::Result<()> {
    let cfg = Config::load().map_err(miette::Report::from)?;
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["key", "value"]);
    table.add_row(vec!["hotkey.toggle", &cfg.hotkey.toggle]);
    table.add_row(vec!["stt.backend", &format!("{:?}", cfg.stt.backend)]);
    table.add_row(vec!["stt.language", &cfg.stt.language]);
    table.add_row(vec![
        "stt.max_recording_secs",
        &cfg.stt.max_recording_secs.to_string(),
    ]);
    table.add_row(vec![
        "injection.method",
        &format!("{:?}", cfg.injection.method),
    ]);
    table.add_row(vec![
        "audio.mic",
        &cfg.audio.mic.clone().unwrap_or_else(|| "(default)".into()),
    ]);
    table.add_row(vec![
        "audio.silence_threshold",
        &cfg.audio.silence_threshold.to_string(),
    ]);

    println!("{table}\n");
    println!("secrets:");
    let provider = Provider::from_env();
    let key_var = format!("{}_API_KEY", provider.env_prefix());
    if let Ok(val) = std::env::var(&key_var) {
        println!("  {key_var} = {}", redact(&val));
    } else {
        println!("  {key_var} = (not set)");
    }
    Ok(())
}

pub fn config_edit(secrets: bool) -> miette::Result<()> {
    let path = if secrets {
        env_file().map_err(miette::Report::from)?
    } else {
        config_file().map_err(miette::Report::from)?
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if !path.exists() && !secrets {
        Config::default().save().map_err(miette::Report::from)?;
    }
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".into());
    let status = Command::new(editor)
        .arg(&path)
        .status()
        .map_err(|e| miette::miette!("spawn editor: {e}"))?;
    if !status.success() {
        return Err(miette::miette!("editor exited with status {status}"));
    }
    // Re-tighten permissions on .env after edit.
    if secrets {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = std::fs::metadata(&path)
                .map_err(|e| miette::miette!(e.to_string()))?
                .permissions();
            p.set_mode(0o600);
            std::fs::set_permissions(&path, p).map_err(|e| miette::miette!(e.to_string()))?;
        }
    }
    Ok(())
}

pub fn config_test() -> miette::Result<()> {
    let provider = Provider::from_env();
    if provider == Provider::Local {
        println!("local backend: nothing to test over network");
        return Ok(());
    }
    // Build the transcriber; if it succeeds, auth + env are present. A real
    // round-trip with 1 second of silent PCM is implemented in a Phase 4
    // refinement (currently most providers reject empty audio).
    let _ = knightingale_core::stt::build_transcriber(provider).map_err(miette::Report::from)?;
    println!("provider={} configured.", provider.as_str());
    Ok(())
}

pub fn provider_list() -> miette::Result<()> {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        "provider",
        "default base url",
        "default model",
        "env prefix",
    ]);
    for p in [
        Provider::Groq,
        Provider::Openai,
        Provider::Deepinfra,
        Provider::Fireworks,
        Provider::Lemonfox,
        Provider::Sambanova,
        Provider::Azure,
        Provider::Custom,
        Provider::Local,
    ] {
        table.add_row(vec![
            p.as_str(),
            p.default_base_url().unwrap_or("(per deployment)"),
            p.default_model(),
            p.env_prefix(),
        ]);
    }
    println!("{table}");
    Ok(())
}

pub fn mic_list() -> miette::Result<()> {
    let devices = audio::list_input_devices().map_err(miette::Report::from)?;
    if devices.is_empty() {
        println!("(no input devices)");
        return Ok(());
    }
    for d in devices {
        println!("- {d}");
    }
    Ok(())
}

pub fn mic_set(name: String) -> miette::Result<()> {
    let mut cfg = Config::load().unwrap_or_default();
    cfg.audio.mic = Some(name);
    cfg.save().map_err(miette::Report::from)?;
    Ok(())
}

pub fn config_set(key: String, value: String) -> miette::Result<()> {
    let mut cfg = Config::load().unwrap_or_default();
    match key.as_str() {
        "hotkey" | "hotkey.toggle" => cfg.hotkey.toggle = value,
        "stt.language" => cfg.stt.language = value,
        "stt.max_recording_secs" => {
            cfg.stt.max_recording_secs = value
                .parse()
                .map_err(|e| miette::miette!("bad number: {e}"))?
        }
        "audio.mic" => cfg.audio.mic = Some(value),
        "audio.silence_threshold" => {
            cfg.audio.silence_threshold = value
                .parse()
                .map_err(|e| miette::miette!("bad number: {e}"))?
        }
        other => return Err(miette::miette!("unknown key: {other}")),
    }
    cfg.save().map_err(miette::Report::from)?;
    Ok(())
}

pub fn config_set_key(provider: String) -> miette::Result<()> {
    let p: Provider = match provider.as_str() {
        "groq" => Provider::Groq,
        "openai" => Provider::Openai,
        "deepinfra" => Provider::Deepinfra,
        "fireworks" => Provider::Fireworks,
        "lemonfox" => Provider::Lemonfox,
        "sambanova" => Provider::Sambanova,
        "azure" => Provider::Azure,
        "custom" => Provider::Custom,
        other => {
            return Err(miette::Report::from(KnightError::Config(format!(
                "unknown provider {other}"
            ))));
        }
    };
    let var = format!("{}_API_KEY", p.env_prefix());
    let key = Password::new(&format!("Enter {var}:"))
        .with_display_mode(PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt()
        .map_err(|e| miette::miette!(e.to_string()))?;
    set_in_env_file(&var, &SecretString::from(key)).map_err(miette::Report::from)?;
    println!("✓ {var} saved");
    Ok(())
}
