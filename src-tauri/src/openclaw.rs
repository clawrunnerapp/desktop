use portable_pty::CommandBuilder;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

use crate::settings::Settings;

/// Allowlist of env var names that may be set from user settings.
/// Prevents injection of dangerous vars like PATH, LD_PRELOAD, etc.
fn is_allowed_env_key(key: &str) -> bool {
    key.len() <= 64
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && key.ends_with("_API_KEY")
}

/// Environment variables safe to pass through from the parent process.
/// This prevents leaking sensitive credentials (AWS_SECRET_ACCESS_KEY,
/// DATABASE_URL, etc.) to the child Node.js process.
const PASSTHROUGH_ENV_VARS: &[&str] = &[
    // System identity
    "HOME", "USER", "LOGNAME", "SHELL",
    // Locale
    "LANG", "LC_ALL", "LC_CTYPE", "LC_MESSAGES", "LC_COLLATE",
    "LC_MONETARY", "LC_NUMERIC", "LC_TIME", "LANGUAGE",
    // Temp directories
    "TMPDIR", "TMP", "TEMP",
    // Linux display (needed if OpenClaw spawns GUI tools)
    "DISPLAY", "WAYLAND_DISPLAY", "XDG_RUNTIME_DIR",
    // macOS specific
    "__CF_USER_TEXT_ENCODING",
    // SSH agent (needed for git operations within OpenClaw)
    "SSH_AUTH_SOCK", "SSH_AGENT_PID",
    // Proxy settings (needed for network access / API calls)
    "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY",
    "http_proxy", "https_proxy", "no_proxy",
    // Node.js TLS
    "NODE_EXTRA_CA_CERTS",
];

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
        return Ok(path);
    }

    // Dev-only fallback: use system node
    if cfg!(debug_assertions) {
        Ok(PathBuf::from(node_name))
    } else {
        Err(format!("Bundled Node.js not found at {:?}", path))
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

    // Dev-only fallbacks (disabled in release builds)
    #[cfg(debug_assertions)]
    {
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
    }

    Err(format!(
        "OpenClaw entry not found. Checked bundled: {:?}",
        bundled_path
    ))
}

/// Returns the OpenClaw state directory (~/.openclaw-desktop/openclaw-state/).
fn openclaw_state_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let base_dir = home.join(".openclaw-desktop");
    let state_dir = base_dir.join("openclaw-state");

    if !state_dir.exists() {
        std::fs::create_dir_all(&state_dir)
            .map_err(|e| format!("Cannot create state dir: {}", e))?;
    }

    // Always enforce permissions (handles both fresh and pre-existing directories)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for dir in &[&base_dir, &state_dir] {
            let perms = std::fs::Permissions::from_mode(0o700);
            let _ = std::fs::set_permissions(dir, perms);
        }
    }

    Ok(state_dir)
}

/// Checks if OpenClaw is already configured (openclaw.json exists in state dir).
pub fn is_configured() -> bool {
    match openclaw_state_dir() {
        Ok(state_dir) => state_dir.join("openclaw.json").exists(),
        Err(_) => false,
    }
}

/// Builds the CommandBuilder for spawning OpenClaw CLI with given args.
/// Example args: ["onboard", "--skip-daemon"], ["gateway"]
pub fn build_openclaw_command(
    app: &AppHandle,
    settings: &Settings,
    args: &[String],
) -> Result<CommandBuilder, String> {
    let node_path = node_binary_path(app)?;
    let entry_path = openclaw_entry_path(app)?;
    let state_dir = openclaw_state_dir()?;

    let mut cmd = CommandBuilder::new(&node_path);

    // Clear inherited environment to prevent leaking sensitive vars
    // (AWS_SECRET_ACCESS_KEY, DATABASE_URL, etc.) to the child process.
    cmd.env_clear();

    // Pass through only safe system env vars from parent process
    for var in PASSTHROUGH_ENV_VARS {
        if let Ok(val) = std::env::var(var) {
            cmd.env(var, &val);
        }
    }

    // Terminal type
    cmd.env("TERM", "xterm-256color");

    // PATH: start with parent's PATH, prepend bundled node dir if available
    let mut path_val = std::env::var("PATH").unwrap_or_default();
    if let Some(node_dir) = node_path.parent() {
        if node_dir != std::path::Path::new("") {
            let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
            path_val = if path_val.is_empty() {
                node_dir.to_string_lossy().to_string()
            } else {
                format!("{}{}{}", node_dir.to_string_lossy(), sep, path_val)
            };
        }
    }
    if !path_val.is_empty() {
        cmd.env("PATH", &path_val);
    }

    // Core env vars for OpenClaw isolation
    cmd.env("OPENCLAW_NO_RESPAWN", "1");
    cmd.env("OPENCLAW_STATE_DIR", state_dir.to_string_lossy().as_ref());

    // Inject API keys from settings as env vars (only known safe key names)
    for (key, value) in &settings.api_keys {
        if !value.is_empty() && is_allowed_env_key(key) {
            cmd.env(key, value);
        }
    }

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

    Ok(cmd)
}
