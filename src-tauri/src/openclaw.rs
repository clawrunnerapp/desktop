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

    // 3. Dev fallback: workspace sibling directory (../../openclaw/openclaw.mjs relative to src-tauri)
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

/// Creates a wrapper shell script for the `openclaw` command in a temp directory.
/// Returns the directory containing the wrapper (to be added to PATH).
fn create_openclaw_wrapper(node_path: &PathBuf, entry_path: &PathBuf) -> Result<PathBuf, String> {
    let wrapper_dir = std::env::temp_dir().join("openclaw-desktop-bin");
    std::fs::create_dir_all(&wrapper_dir)
        .map_err(|e| format!("Cannot create wrapper dir: {}", e))?;

    let wrapper_path = wrapper_dir.join("openclaw");

    let node_str = node_path.to_string_lossy();
    let entry_str = entry_path.to_string_lossy();

    let script = format!(
        "#!/bin/sh\nexec \"{}\" --disable-warning=ExperimentalWarning \"{}\" \"$@\"\n",
        node_str, entry_str
    );

    std::fs::write(&wrapper_path, &script)
        .map_err(|e| format!("Cannot write wrapper script: {}", e))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&wrapper_path, perms)
            .map_err(|e| format!("Cannot chmod wrapper: {}", e))?;
    }

    Ok(wrapper_dir)
}

/// Builds the CommandBuilder for spawning an interactive shell with OpenClaw available.
pub fn build_openclaw_command(
    app: &AppHandle,
    settings: &Settings,
) -> Result<CommandBuilder, String> {
    let node_path = node_binary_path(app)?;
    eprintln!("[openclaw] Node path: {:?}", node_path);
    let entry_path = openclaw_entry_path(app)?;
    eprintln!("[openclaw] Entry path: {:?}", entry_path);
    let state_dir = openclaw_state_dir()?;
    eprintln!("[openclaw] State dir: {:?}", state_dir);

    // Create wrapper script so `openclaw` command is available in the shell
    let wrapper_dir = create_openclaw_wrapper(&node_path, &entry_path)?;
    eprintln!("[openclaw] Wrapper dir: {:?}", wrapper_dir);

    // Determine user's shell
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    eprintln!("[openclaw] Shell: {}", shell);

    let mut cmd = CommandBuilder::new(&shell);

    // Set working directory to user's home
    if let Some(home) = dirs::home_dir() {
        cmd.cwd(home);
    }

    // Core env vars for OpenClaw isolation
    cmd.env("OPENCLAW_NO_RESPAWN", "1");
    cmd.env("OPENCLAW_STATE_DIR", state_dir.to_string_lossy().as_ref());

    // Put wrapper dir + node dir at the front of PATH
    let mut path_parts = vec![wrapper_dir.to_string_lossy().to_string()];
    if let Some(node_dir) = node_path.parent() {
        if node_dir != std::path::Path::new("") {
            path_parts.push(node_dir.to_string_lossy().to_string());
        }
    }
    if let Ok(current_path) = std::env::var("PATH") {
        path_parts.push(current_path);
    }
    cmd.env("PATH", &path_parts.join(":"));

    // Inject API keys from settings as env vars
    for (key, value) in &settings.api_keys {
        if !value.is_empty() {
            cmd.env(key, value);
        }
    }

    // Pass through common env vars that CLI might need
    for var in &["HOME", "USER", "SHELL", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            cmd.env(var, &val);
        }
    }
    cmd.env("TERM", "xterm-256color");

    Ok(cmd)
}
