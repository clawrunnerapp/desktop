use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default, rename = "apiKeys")]
    pub api_keys: HashMap<String, String>,
}

/// Returns the path to the settings file (~/.clawrunner/settings.json).
fn settings_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    Ok(home.join(".clawrunner").join("settings.json"))
}

/// Ensures the settings directory exists with restricted permissions.
fn ensure_settings_dir() -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let config_dir = home.join(".clawrunner");

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Cannot create config dir: {}", e))?;
    }

    // Always enforce permissions (handles both fresh and pre-existing directories)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        let _ = std::fs::set_permissions(&config_dir, perms);
    }

    Ok(())
}

/// Loads settings from disk. Returns default if file doesn't exist.
pub fn load_settings() -> Settings {
    let path = match settings_path() {
        Ok(p) => p,
        Err(_) => return Settings::default(),
    };

    if !path.exists() {
        return Settings::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Saves settings to disk atomically with restricted permissions.
/// Writes to a temp file first, then renames to prevent corruption on crash.
pub fn save_settings_to_disk(settings: &Settings) -> Result<(), String> {
    ensure_settings_dir()?;
    let path = settings_path()?;
    let tmp_path = path.with_extension("json.tmp");
    let content =
        serde_json::to_string_pretty(settings).map_err(|e| format!("Serialize error: {}", e))?;

    #[cfg(unix)]
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp_path)
            .map_err(|e| format!("Write error: {}", e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        file.sync_all().map_err(|e| format!("Sync error: {}", e))?;
    }

    #[cfg(not(unix))]
    {
        std::fs::write(&tmp_path, &content).map_err(|e| format!("Write error: {}", e))?;
    }

    std::fs::rename(&tmp_path, &path).map_err(|e| format!("Rename error: {}", e))?;

    // Sync parent directory to ensure rename is durable (important on Linux/ext4)
    #[cfg(unix)]
    {
        if let Some(parent) = path.parent() {
            if let Ok(dir) = std::fs::File::open(parent) {
                let _ = dir.sync_all();
            }
        }
    }

    Ok(())
}
