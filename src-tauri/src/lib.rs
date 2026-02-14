mod openclaw;
mod pty_manager;
mod settings;

use pty_manager::PtyManager;
use settings::Settings;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

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
) -> Result<(), String> {
    // Update stored settings
    {
        let mut s = state.settings.lock().map_err(|e| e.to_string())?;
        *s = settings.clone();
    }

    let cmd = openclaw::build_openclaw_command(&app, &settings, &args)?;
    state.pty.spawn(&app, cmd, 120, 40)
}

#[tauri::command]
fn pty_write(state: tauri::State<'_, AppState>, data: String) -> Result<(), String> {
    state.pty.write(&data)
}

#[tauri::command]
fn pty_resize(state: tauri::State<'_, AppState>, cols: u16, rows: u16) -> Result<(), String> {
    state.pty.resize(cols, rows)
}

#[tauri::command]
fn pty_kill(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.pty.kill()
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
        .setup(|app| {
            let settings = settings::load_settings();
            let _ = app.emit("settings:loaded", &settings);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = window.try_state::<AppState>() {
                    let _ = state.pty.kill();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
