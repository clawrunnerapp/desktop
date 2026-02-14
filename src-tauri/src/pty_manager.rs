use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

pub struct PtyInstance {
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _reader_thread: thread::JoinHandle<()>,
}

pub struct PtyManager {
    instance: Arc<Mutex<Option<PtyInstance>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            instance: Arc::new(Mutex::new(None)),
        }
    }

    pub fn spawn(
        &self,
        app: &AppHandle,
        cmd: CommandBuilder,
        cols: u16,
        rows: u16,
    ) -> Result<(), String> {
        // Kill existing instance if any
        self.kill()?;

        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to get PTY writer: {}", e))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to get PTY reader: {}", e))?;

        // Spawn reader thread that forwards PTY output to frontend via Tauri events
        let app_handle = app.clone();
        let reader_thread = thread::spawn(move || {
            eprintln!("[pty_reader] Reader thread started");
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        eprintln!("[pty_reader] EOF");
                        break;
                    }
                    Ok(n) => {
                        // Convert bytes to string (lossy for non-UTF8 terminal output)
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        eprintln!("[pty_reader] Read {} bytes", n);
                        let _ = app_handle.emit("pty:data", &data);
                    }
                    Err(e) => {
                        eprintln!("[pty_reader] Error: {}", e);
                        break;
                    }
                }
            }
            // Process ended - emit status
            eprintln!("[pty_reader] Thread ending, emitting stopped");
            let _ = app_handle.emit(
                "pty:status",
                serde_json::json!({ "status": "stopped" }),
            );
        });

        let mut lock = self.instance.lock().map_err(|e| e.to_string())?;
        *lock = Some(PtyInstance {
            writer,
            child,
            _reader_thread: reader_thread,
        });

        Ok(())
    }

    pub fn write(&self, data: &str) -> Result<(), String> {
        let mut lock = self.instance.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut inst) = *lock {
            inst.writer
                .write_all(data.as_bytes())
                .map_err(|e| format!("Write error: {}", e))?;
            inst.writer
                .flush()
                .map_err(|e| format!("Flush error: {}", e))?;
            Ok(())
        } else {
            Err("No PTY instance".to_string())
        }
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        // portable-pty resize requires access to the master PTY
        // For now, we store the size and it will be applied on next spawn
        // TODO: Store master handle for resize support
        let _ = cols;
        let _ = rows;
        Ok(())
    }

    pub fn kill(&self) -> Result<(), String> {
        let mut lock = self.instance.lock().map_err(|e| e.to_string())?;
        if let Some(mut inst) = lock.take() {
            let _ = inst.child.kill();
            let _ = inst.child.wait();
        }
        Ok(())
    }
}
