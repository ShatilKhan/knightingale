use std::fs;
use std::path::Path;

pub use secrecy::{ExposeSecret, SecretString};

use crate::config::env_file;
use crate::error::{KnightError, Result};

#[cfg(unix)]
const ENV_FILE_MODE: u32 = 0o600;
#[cfg(unix)]
const CONFIG_DIR_MODE: u32 = 0o700;

pub fn load_env_file() -> Result<()> {
    let path = env_file()?;
    if path.exists() {
        check_permissions(&path)?;
        dotenvy::from_path(&path).map_err(|e| KnightError::Config(format!("load .env: {e}")))?;
    }
    Ok(())
}

pub fn get_secret(var: &str) -> Option<SecretString> {
    std::env::var(var).ok().map(SecretString::from)
}

pub fn set_in_env_file(var: &str, value: &SecretString) -> Result<()> {
    let path = env_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        tighten_dir_mode(parent)?;
    }
    let existing = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };
    let mut updated = String::new();
    let mut replaced = false;
    let prefix = format!("{var}=");
    for line in existing.lines() {
        if line.starts_with(&prefix) {
            updated.push_str(&format!("{var}={}\n", value.expose_secret()));
            replaced = true;
        } else {
            updated.push_str(line);
            updated.push('\n');
        }
    }
    if !replaced {
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&format!("{var}={}\n", value.expose_secret()));
    }
    write_secret_file(&path, updated.as_bytes())?;
    Ok(())
}

pub fn redact(secret: &str) -> String {
    let len = secret.len();
    if len <= 6 {
        return "•".repeat(len.max(4));
    }
    let prefix: String = secret.chars().take(4).collect();
    let suffix: String = secret
        .chars()
        .rev()
        .take(2)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{prefix}{}{suffix}", "•".repeat(8))
}

#[cfg(unix)]
fn check_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path)?;
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(KnightError::Permission(format!(
            ".env file mode {mode:o} is world/group-readable; chmod 600 {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn write_secret_file(path: &Path, body: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(ENV_FILE_MODE)
        .open(path)?;
    file.write_all(body)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_secret_file(path: &Path, body: &[u8]) -> Result<()> {
    fs::write(path, body)?;
    Ok(())
}

#[cfg(unix)]
fn tighten_dir_mode(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = fs::metadata(path)?.permissions();
    perm.set_mode(CONFIG_DIR_MODE);
    fs::set_permissions(path, perm)?;
    Ok(())
}

#[cfg(not(unix))]
fn tighten_dir_mode(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_short() {
        assert_eq!(redact("abc"), "••••");
        assert_eq!(redact("abcdef"), "••••••");
    }

    #[test]
    fn redact_long() {
        assert_eq!(redact("gsk_1234567890abcdef42"), "gsk_••••••••42");
    }
}
