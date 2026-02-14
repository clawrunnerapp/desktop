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
    // 1. Check bundled resources (production)
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Cannot resolve resource dir: {}", e))?;

    let bundled_path = resource_dir
        .join("resources")
        .join("openclaw")
        .join("openclaw.mjs");
    if bundled_path.exists() {
        return Ok(bundled_path);
    }

    // 2. Check DEV_OPENCLAW_PATH env var
    if let Ok(dev_path) = std::env::var("DEV_OPENCLAW_PATH") {
        let p = PathBuf::from(&dev_path);
        if p.exists() {
            return Ok(p);
        }
    }

    // 3. Dev fallback: workspace sibling directory
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_path = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|workspace| workspace.join("openclaw").join("openclaw.mjs"));

    if let Some(path) = dev_path {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(format!(
        "OpenClaw entry not found. Checked bundled: {:?}",
        bundled_path
    ))
}

/// Returns the OpenClaw state directory (~/.openclaw-desktop/openclaw-state/).
fn openclaw_state_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let state_dir = home.join(".openclaw-desktop").join("openclaw-state");
    std::fs::create_dir_all(&state_dir)
        .map_err(|e| format!("Cannot create state dir: {}", e))?;
    Ok(state_dir)
}

/// Checks if OpenClaw is already configured (openclaw.json exists in state dir).
pub fn is_configured() -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };
    let config_path = home
        .join(".openclaw-desktop")
        .join("openclaw-state")
        .join("openclaw.json");
    config_path.exists()
}

/// Builds the CommandBuilder for spawning OpenClaw CLI with given args.
/// Example args: ["onboard"], ["gateway"], ["configure"]
pub fn build_openclaw_command(
    app: &AppHandle,
    settings: &Settings,
    args: &[String],
) -> Result<CommandBuilder, String> {
    let node_path = node_binary_path(app)?;
    let entry_path = openclaw_entry_path(app)?;
    let state_dir = openclaw_state_dir()?;

    let mut cmd = CommandBuilder::new(&node_path);

    // Node.js flags + openclaw entry point
    cmd.arg("--disable-warning=ExperimentalWarning");
    cmd.arg(&entry_path);

    // Append openclaw subcommand args (e.g. "onboard", "gateway")
    for arg in args {
        cmd.arg(arg);
    }

    // Set working directory to user's home
    if let Some(home) = dirs::home_dir() {
        cmd.cwd(home);
    }

    // Core env vars for OpenClaw isolation
    cmd.env("OPENCLAW_NO_RESPAWN", "1");
    cmd.env("OPENCLAW_STATE_DIR", state_dir.to_string_lossy().as_ref());

    // Ensure node is in PATH
    if let Some(node_dir) = node_path.parent() {
        if node_dir != std::path::Path::new("") {
            if let Ok(current_path) = std::env::var("PATH") {
                let new_path = format!("{}:{}", node_dir.to_string_lossy(), current_path);
                cmd.env("PATH", &new_path);
            }
        }
    }

    // Inject API keys from settings as env vars
    for (key, value) in &settings.api_keys {
        if !value.is_empty() {
            cmd.env(key, value);
        }
    }

    // Pass through common env vars
    for var in &["HOME", "USER", "SHELL", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            cmd.env(var, &val);
        }
    }
    cmd.env("TERM", "xterm-256color");

    Ok(cmd)
}
