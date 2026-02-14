use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Maximum leftover buffer size (64 KB). If exceeded, flush with lossy conversion.
const MAX_LEFTOVER_SIZE: usize = 65536;

struct PtyInstance {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    reader_thread: Option<thread::JoinHandle<()>>,
    session_id: u64,
}

/// Safety net: kills child process on drop if not explicitly cleaned up.
/// The explicit kill() already does kill+wait+join; Drop is for unclean exits only.
/// reader_thread is not joined here to avoid blocking in Drop; it will exit
/// once the master PTY fd is closed (which happens when `master` is dropped).
impl Drop for PtyInstance {
    fn drop(&mut self) {
        // Safe to call multiple times; portable-pty handles double-kill gracefully.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Kills and waits a child process. Safe to call multiple times.
fn cleanup_child(child: &mut Box<dyn portable_pty::Child + Send + Sync>) {
    let _ = child.kill();
    let _ = child.wait();
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
    ) -> Result<u64, String> {
        // Hold lock for the entire spawn to prevent races between concurrent calls
        let mut lock = self.instance.lock().map_err(|e| e.to_string())?;

        // Kill existing instance if any
        if let Some(mut inst) = lock.take() {
            cleanup_child(&mut inst.child);
            if let Some(handle) = inst.reader_thread.take() {
                let _ = handle.join();
            }
        }

        // Session IDs start at 1; 0 is reserved as the "kill unconditionally" sentinel.
        let mut session_id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        if session_id == 0 {
            session_id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        }

        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        // Explicitly drop slave after spawning to ensure proper EOF on master
        drop(pair.slave);

        let writer = match pair.master.take_writer() {
            Ok(w) => w,
            Err(e) => {
                cleanup_child(&mut child);
                return Err(format!("Failed to get PTY writer: {}", e));
            }
        };

        let reader = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                cleanup_child(&mut child);
                return Err(format!("Failed to get PTY reader: {}", e));
            }
        };

        let app_handle = app.clone();
        let reader_thread = spawn_reader_thread(reader, app_handle, session_id);

        *lock = Some(PtyInstance {
            writer,
            master: pair.master,
            child,
            reader_thread: Some(reader_thread),
            session_id,
        });

        Ok(session_id)
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
        let lock = self.instance.lock().map_err(|e| e.to_string())?;
        if let Some(ref inst) = *lock {
            inst.master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| format!("Resize error: {}", e))?;
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Kills the current PTY session if its session_id matches.
    /// Pass session_id=0 to kill unconditionally.
    pub fn kill(&self, session_id: u64) -> Result<(), String> {
        let mut lock = self.instance.lock().map_err(|e| e.to_string())?;
        let should_kill = match lock.as_ref() {
            Some(inst) => session_id == 0 || inst.session_id == session_id,
            None => false,
        };
        if should_kill {
            if let Some(mut inst) = lock.take() {
                cleanup_child(&mut inst.child);
                if let Some(handle) = inst.reader_thread.take() {
                    let _ = handle.join();
                }
            }
        }
        Ok(())
    }
}

/// Spawns a reader thread that forwards PTY output to frontend via Tauri events.
/// Handles multi-byte UTF-8 sequences that may be split across reads.
/// Events are tagged with session_id so the frontend can ignore stale events.
fn spawn_reader_thread(
    mut reader: Box<dyn Read + Send>,
    app_handle: AppHandle,
    session_id: u64,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut leftover = Vec::new();
        let mut error_msg: Option<String> = None;

        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    leftover.extend_from_slice(&buf[..n]);

                    // Cap leftover to prevent unbounded growth from binary output
                    if leftover.len() > MAX_LEFTOVER_SIZE {
                        let data = String::from_utf8_lossy(&leftover).to_string();
                        let _ = app_handle.emit("pty:data", serde_json::json!({
                            "sessionId": session_id,
                            "data": data,
                        }));
                        leftover.clear();
                        continue;
                    }

                    // Find the last valid UTF-8 boundary
                    let valid_up_to = match std::str::from_utf8(&leftover) {
                        Ok(_) => leftover.len(),
                        Err(e) => e.valid_up_to(),
                    };

                    if valid_up_to > 0 {
                        // Safety: validated by from_utf8 above
                        let text = unsafe {
                            std::str::from_utf8_unchecked(&leftover[..valid_up_to])
                        };
                        let _ = app_handle.emit("pty:data", serde_json::json!({
                            "sessionId": session_id,
                            "data": text,
                        }));
                    }

                    // Keep incomplete bytes for next read
                    leftover = leftover[valid_up_to..].to_vec();
                }
                Err(e) => {
                    error_msg = Some(e.to_string());
                    break;
                }
            }
        }

        // Flush any remaining bytes
        if !leftover.is_empty() {
            let data = String::from_utf8_lossy(&leftover).to_string();
            let _ = app_handle.emit("pty:data", serde_json::json!({
                "sessionId": session_id,
                "data": data,
            }));
        }

        let status_str = if error_msg.is_some() { "error" } else { "stopped" };
        let mut status = serde_json::json!({
            "sessionId": session_id,
            "status": status_str,
        });
        if let Some(err) = error_msg {
            status["errorMessage"] = serde_json::Value::String(err);
        }
        let _ = app_handle.emit("pty:status", status);
    })
}
