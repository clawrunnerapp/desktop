use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Maximum leftover buffer size (64 KB). If exceeded, flush with lossy conversion.
const MAX_LEFTOVER_SIZE: usize = 65536;

struct PtyInstance {
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    master: Option<Arc<Mutex<Box<dyn MasterPty + Send>>>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    reader_thread: Option<thread::JoinHandle<()>>,
}

/// Safety net: kills child process on drop if not explicitly cleaned up.
/// The explicit kill() already does kill+wait+join; Drop is for unclean exits only.
/// reader_thread is not joined here to avoid blocking in Drop; it will exit
/// once the master PTY fd is closed (which happens when `master`/`writer` are dropped).
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
    sessions: Arc<Mutex<HashMap<u64, PtyInstance>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn(
        &self,
        app: &AppHandle,
        cmd: CommandBuilder,
        cols: u16,
        rows: u16,
    ) -> Result<u64, String> {
        // Session IDs start at 1; 0 is reserved as the "kill all" sentinel.
        let session_id = loop {
            let id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
            if id != 0 { break id; }
        };

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

        let instance = PtyInstance {
            writer: Some(Arc::new(Mutex::new(writer))),
            master: Some(Arc::new(Mutex::new(pair.master))),
            child,
            reader_thread: Some(reader_thread),
        };

        let mut lock = self.sessions.lock().map_err(|e| e.to_string())?;
        lock.insert(session_id, instance);

        Ok(session_id)
    }

    pub fn write(&self, session_id: u64, data: &str) -> Result<(), String> {
        // Get a clone of the writer Arc, then release the global lock before I/O.
        // This prevents blocking other sessions if write_all blocks.
        let writer = {
            let lock = self.sessions.lock().map_err(|e| e.to_string())?;
            lock.get(&session_id)
                .and_then(|inst| inst.writer.as_ref().map(Arc::clone))
                .ok_or_else(|| format!("No PTY session with id {}", session_id))?
        };
        let mut w = writer.lock().map_err(|e| e.to_string())?;
        w.write_all(data.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        w.flush()
            .map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    pub fn resize(&self, session_id: u64, cols: u16, rows: u16) -> Result<(), String> {
        // Get a clone of the master Arc, then release the global lock before I/O.
        let master = {
            let lock = self.sessions.lock().map_err(|e| e.to_string())?;
            lock.get(&session_id)
                .and_then(|inst| inst.master.as_ref().map(Arc::clone))
                .ok_or_else(|| format!("No PTY session with id {}", session_id))?
        };
        let m = master.lock().map_err(|e| e.to_string())?;
        m.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Resize error: {}", e))?;
        Ok(())
    }

    /// Kills a PTY session by session_id.
    /// Pass session_id=0 to kill all sessions (used for window close).
    pub fn kill(&self, session_id: u64) -> Result<(), String> {
        // Remove from map while holding lock, then clean up outside lock
        // to avoid blocking other operations during process wait/thread join.
        let removed: Vec<PtyInstance> = {
            let mut lock = self.sessions.lock().map_err(|e| e.to_string())?;
            if session_id == 0 {
                let ids: Vec<u64> = lock.keys().copied().collect();
                ids.into_iter().filter_map(|id| lock.remove(&id)).collect()
            } else {
                lock.remove(&session_id).into_iter().collect()
            }
        };
        for mut inst in removed {
            cleanup_child(&mut inst.child);
            // Drop master and writer BEFORE joining reader thread.
            // This closes the PTY fd, which unblocks the reader thread's read()
            // even if grandchild processes still hold the slave fd open.
            drop(inst.writer.take());
            drop(inst.master.take());
            if let Some(handle) = inst.reader_thread.take() {
                let _ = handle.join();
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
                        Ok(s) => s.len(),
                        Err(e) => e.valid_up_to(),
                    };

                    if valid_up_to > 0 {
                        // unwrap is safe: from_utf8 validated [0..valid_up_to] above
                        let text = std::str::from_utf8(&leftover[..valid_up_to]).unwrap();
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
