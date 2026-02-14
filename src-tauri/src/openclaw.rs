use portable_pty::CommandBuilder;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

use crate::settings::Settings;

/// Resolves the path to the bundled Node.js binary inside Tauri resources.
fn node_binary_path(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Cannot resolve resource dir: {}", e))?;

    let node_name = if cfg!(target_os = "windows") {
        "node.exe"
    } else {
        "node"
    };

    let path = resource_dir.join("resources").join(node_name);
    if path.exists() {
        Ok(path)
    } else {
        // Fallback: try system node (for development)
        Ok(PathBuf::from(node_name))
    }
}

/// Resolves the path to the bundled OpenClaw entry point.
fn openclaw_entry_path(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Cannot resolve resource dir: {}", e))?;

    let path = resource_dir
        .join("resources")
        .join("openclaw")
        .join("openclaw.mjs");
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("OpenClaw entry not found at: {:?}", path))
    }
}

/// Returns the OpenClaw state directory (~/.openclaw-desktop/openclaw-state/).
fn openclaw_state_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let state_dir = home.join(".openclaw-desktop").join("openclaw-state");
    std::fs::create_dir_all(&state_dir)
        .map_err(|e| format!("Cannot create state dir: {}", e))?;
    Ok(state_dir)
}

/// Builds the CommandBuilder for spawning OpenClaw CLI via PTY.
pub fn build_openclaw_command(
    app: &AppHandle,
    settings: &Settings,
) -> Result<CommandBuilder, String> {
    let node_path = node_binary_path(app)?;
    let entry_path = openclaw_entry_path(app)?;
    let state_dir = openclaw_state_dir()?;

    let mut cmd = CommandBuilder::new(&node_path);

    // Node.js args: suppress experimental warnings, run openclaw entry
    cmd.arg("--disable-warning=ExperimentalWarning");
    cmd.arg(&entry_path);

    // Set working directory to user's home
    if let Some(home) = dirs::home_dir() {
        cmd.cwd(home);
    }

    // Core env vars for OpenClaw isolation
    cmd.env("OPENCLAW_NO_RESPAWN", "1");
    cmd.env("OPENCLAW_STATE_DIR", state_dir.to_string_lossy().as_ref());

    // Ensure bundled node's directory is in PATH
    if let Some(node_dir) = node_path.parent() {
        if let Ok(current_path) = std::env::var("PATH") {
            let new_path = format!(
                "{}:{}",
                node_dir.to_string_lossy(),
                current_path
            );
            cmd.env("PATH", &new_path);
        }
    }

    // Inject API keys from settings as env vars
    for (key, value) in &settings.api_keys {
        if !value.is_empty() {
            cmd.env(key, value);
        }
    }

    // Pass through common env vars that CLI might need
    for var in &["HOME", "USER", "SHELL", "TERM", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            cmd.env(var, &val);
        }
    }
    cmd.env("TERM", "xterm-256color");

    Ok(cmd)
}
