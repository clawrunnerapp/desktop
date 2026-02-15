mod openclaw;
mod pty_manager;
mod settings;

use pty_manager::PtyManager;
use settings::Settings;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

/// Allowed OpenClaw subcommands that can be passed from the frontend.
const ALLOWED_ARGS: &[&str] = &[
    "onboard",
    "--skip-daemon",
    "gateway",
    "tui",
];

fn validate_args(args: &[String]) -> Result<(), String> {
    for arg in args {
        if !ALLOWED_ARGS.contains(&arg.as_str()) {
            return Err(format!("Disallowed argument: {}", arg));
        }
    }
    Ok(())
}

struct AppState {
    pty: PtyManager,
    settings: Mutex<Settings>,
}

#[tauri::command]
fn pty_spawn(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: Settings,
    args: Vec<String>,
    cols: u16,
    rows: u16,
) -> Result<u64, String> {
    if cols == 0 || rows == 0 {
        return Err("cols and rows must be non-zero".to_string());
    }
    validate_args(&args)?;

    // Update stored settings
    {
        let mut s = state.settings.lock().map_err(|e| e.to_string())?;
        *s = settings.clone();
    }

    let cmd = openclaw::build_openclaw_command(&app, &settings, &args)?;
    state.pty.spawn(&app, cmd, cols, rows)
}

const MAX_WRITE_SIZE: usize = 1_048_576; // 1 MB

#[tauri::command]
fn pty_write(state: tauri::State<'_, AppState>, session_id: u64, data: String) -> Result<(), String> {
    if data.len() > MAX_WRITE_SIZE {
        return Err(format!("Write data too large: {} bytes", data.len()));
    }
    state.pty.write(session_id, &data)
}

#[tauri::command]
fn pty_resize(state: tauri::State<'_, AppState>, session_id: u64, cols: u16, rows: u16) -> Result<(), String> {
    if cols == 0 || rows == 0 {
        return Err("cols and rows must be non-zero".to_string());
    }
    state.pty.resize(session_id, cols, rows)
}

#[tauri::command]
fn pty_kill(state: tauri::State<'_, AppState>, session_id: u64) -> Result<(), String> {
    if session_id == 0 {
        return Err("Invalid session_id: 0 is reserved".to_string());
    }
    state.pty.kill(session_id)
}

#[tauri::command]
fn save_settings(
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    settings::save_settings_to_disk(&settings)?;
    let mut s = state.settings.lock().map_err(|e| e.to_string())?;
    *s = settings;
    Ok(())
}

#[tauri::command]
fn load_settings_cmd() -> Settings {
    settings::load_settings()
}

#[tauri::command]
fn check_openclaw_configured() -> bool {
    openclaw::is_configured()
}

pub fn run() {
    let initial_settings = settings::load_settings();

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .manage(AppState {
            pty: PtyManager::new(),
            settings: Mutex::new(initial_settings),
        })
        .invoke_handler(tauri::generate_handler![
            pty_spawn,
            pty_write,
            pty_resize,
            pty_kill,
            save_settings,
            load_settings_cmd,
            check_openclaw_configured,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = window.try_state::<AppState>() {
                    let _ = state.pty.kill(0);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
