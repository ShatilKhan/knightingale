use std::process::Command;

use comfy_table::Table;
use comfy_table::presets::UTF8_FULL;
use inquire::{Confirm, Password, PasswordDisplayMode, Select, Text};
use knightingale_core::audio;
use knightingale_core::config::{Config, SttBackend, config_file, env_file};
use knightingale_core::error::KnightError;
use knightingale_core::hardware;
use knightingale_core::model::{self, Model};
use knightingale_core::secret::{SecretString, redact, set_in_env_file};
use knightingale_core::setup as setup_mod;
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

    // OS-aware checks: compositor, uinput, macOS permissions.
    let comp = setup_mod::detect_compositor();
    table.add_row(vec!["compositor", "info", comp.as_str()]);

    #[cfg(target_os = "linux")]
    {
        let uinput_ok = setup_mod::uinput_writable();
        table.add_row(vec![
            "/dev/uinput",
            if uinput_ok { "ok" } else { "fail" },
            if uinput_ok {
                "writable"
            } else {
                "add user to `input` group: sudo usermod -aG input $USER"
            },
        ]);
    }

    // Hardware summary.
    let hw = hardware::detect();
    table.add_row(vec![
        "cpu",
        "info",
        &format!("{} ({} cores)", hw.cpu_brand, hw.cpu_cores),
    ]);
    table.add_row(vec!["ram", "info", &format!("{} MB", hw.ram_total_mb)]);
    if let Some(g) = &hw.gpu {
        table.add_row(vec![
            "gpu",
            "info",
            &format!("{} {} ({} MB)", g.vendor, g.name, g.vram_mb),
        ]);
    } else {
        table.add_row(vec!["gpu", "info", "none detected (CPU-only)"]);
    }

    println!("{table}");

    // Per-compositor hotkey hint.
    if let Some(cmd) = setup_mod::hotkey_command(comp, &cfg.hotkey.toggle, "knightingale toggle") {
        println!(
            "\n# {} — paste this into your config to bind the hotkey:",
            comp.as_str()
        );
        println!("{cmd}");
    }
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

pub fn model_list() -> miette::Result<()> {
    let cat = model::catalog().map_err(miette::Report::from)?;
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        "alias",
        "disk",
        "vram",
        "lang",
        "installed",
        "best for",
    ]);
    for m in &cat {
        let installed = model::is_installed(&m.alias).unwrap_or(false);
        table.add_row(vec![
            m.alias.clone(),
            format!("{} MB", m.size_mb),
            format!("{} MB", m.vram_mb),
            m.language.clone(),
            (if installed { "yes" } else { "no" }).into(),
            m.recommended_for.clone(),
        ]);
    }
    println!("{table}");
    Ok(())
}

pub fn model_pull(alias: &str) -> miette::Result<()> {
    eprintln!("pulling {alias} …");
    let path = model::pull(alias).map_err(miette::Report::from)?;
    println!("✓ {alias} installed at {}", path.display());
    Ok(())
}

pub fn model_recommend(english_only: bool) -> miette::Result<()> {
    let hw = hardware::detect();
    let cat = model::catalog().map_err(miette::Report::from)?;
    let pick: Option<&Model> = hardware::recommend(&hw, &cat, english_only);

    let mut header = Table::new();
    header.load_preset(UTF8_FULL);
    header.set_header(vec!["component", "detail"]);
    header.add_row(vec![
        "CPU",
        &format!("{} ({} cores)", hw.cpu_brand, hw.cpu_cores),
    ]);
    header.add_row(vec!["RAM", &format!("{} MB", hw.ram_total_mb)]);
    if let Some(g) = &hw.gpu {
        header.add_row(vec![
            "GPU",
            &format!("{} {} ({} MB)", g.vendor, g.name, g.vram_mb),
        ]);
    } else {
        header.add_row(vec!["GPU", "none detected (CPU-only)"]);
    }
    println!("{header}");

    let mut rows = Table::new();
    rows.load_preset(UTF8_FULL);
    rows.set_header(vec!["alias", "disk", "vram", "note"]);
    for m in &cat {
        let fits_gpu = hw
            .gpu
            .as_ref()
            .map(|g| (m.vram_mb as u64) * 2 <= g.vram_mb)
            .unwrap_or(false);
        let fits_cpu = (m.size_mb as u64) * 4 <= hw.ram_total_mb;
        let fits = fits_gpu || (hw.gpu.is_none() && fits_cpu);
        let recommended = pick.map(|p| p.alias == m.alias).unwrap_or(false);
        let note = match (recommended, fits) {
            (true, _) => format!("Recommended — {}", m.recommended_for),
            (false, true) => m.recommended_for.clone(),
            (false, false) => "⚠ exceeds VRAM headroom".into(),
        };
        rows.add_row(vec![
            m.alias.clone(),
            format!("{} MB", m.size_mb),
            format!("{} MB", m.vram_mb),
            note,
        ]);
    }
    println!("{rows}");
    if let Some(p) = pick {
        println!("\n→ knightingale model pull {}", p.alias);
    }
    Ok(())
}

pub fn eval_run(corpus: std::path::PathBuf) -> miette::Result<()> {
    use knightingale_core::eval;
    use knightingale_core::stt::{Provider, build_transcriber};

    let cfg = Config::load().map_err(miette::Report::from)?;
    let provider = Provider::from_env();
    let transcriber = build_transcriber(provider).map_err(miette::Report::from)?;

    let entries: Vec<std::path::PathBuf> = std::fs::read_dir(&corpus)
        .map_err(|e| miette::miette!(e.to_string()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("wav"))
        .collect();

    if entries.is_empty() {
        return Err(miette::miette!("no .wav files in {}", corpus.display()));
    }

    let mut rows = Vec::new();
    for wav in entries {
        let txt = wav.with_extension("txt");
        let reference = match std::fs::read_to_string(&txt) {
            Ok(s) => s.trim().to_string(),
            Err(_) => {
                eprintln!("skipped {} (no matching .txt)", wav.display());
                continue;
            }
        };
        let r = eval::run_clip(&*transcriber, &wav, &reference, &cfg.stt.language)
            .map_err(miette::Report::from)?;
        rows.push(r);
    }

    let agg = eval::aggregate(&rows);

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["clip", "audio s", "proc s", "rtf", "wer %", "ok"]);
    for r in &rows {
        let name = std::path::Path::new(&r.clip)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| r.clip.clone());
        table.add_row(vec![
            name,
            format!("{:.2}", r.audio_secs),
            format!("{:.2}", r.processing_secs),
            format!("{:.2}", r.rtf),
            format!("{:.1}", r.wer_pct),
            (if r.correct { "✓" } else { "✗" }).into(),
        ]);
    }
    println!("{table}");
    println!(
        "\nprovider={} avg WER={:.1}% avg RTF={:.2} SER={:.1}%",
        provider.as_str(),
        agg.avg_wer_pct,
        agg.avg_rtf,
        agg.ser_pct
    );
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
