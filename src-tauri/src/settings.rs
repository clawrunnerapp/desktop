use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default, rename = "apiKeys")]
    pub api_keys: HashMap<String, String>,
}

/// Returns the path to the settings file (~/.openclaw-desktop/settings.json).
fn settings_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let config_dir = home.join(".openclaw-desktop");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Cannot create config dir: {}", e))?;
    Ok(config_dir.join("settings.json"))
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

/// Saves settings to disk with restricted permissions.
pub fn save_settings_to_disk(settings: &Settings) -> Result<(), String> {
    let path = settings_path()?;
    let content =
        serde_json::to_string_pretty(settings).map_err(|e| format!("Serialize error: {}", e))?;

    std::fs::write(&path, &content).map_err(|e| format!("Write error: {}", e))?;

    // Set file permissions to 0600 (owner read/write only) on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .map_err(|e| format!("Permission error: {}", e))?;
    }

    Ok(())
}
