use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use lingxi_domain::TerminalId;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use thiserror::Error;

pub const MAX_RING_LINES: usize = 5000;

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("pty error: {0}")]
    Pty(String),
    #[error("terminal not found: {0}")]
    NotFound(String),
}

struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    lines: Arc<Mutex<VecDeque<String>>>,
}

pub struct TerminalManager {
    sessions: Arc<Mutex<HashMap<TerminalId, PtySession>>>,
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn_shell(
        &self,
        cwd: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<TerminalId, TerminalError> {
        let id = TerminalId::new();
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::Pty(e.to_string()))?;

        let shell = detect_default_shell();
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(cwd);

        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::Pty(e.to_string()))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::Pty(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::Pty(e.to_string()))?;

        let lines = Arc::new(Mutex::new(VecDeque::with_capacity(256)));
        let lines_bg = Arc::clone(&lines);

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut partial = String::new();
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let chunk = String::from_utf8_lossy(&buf[..n]);
                partial.push_str(&chunk);
                while let Some(pos) = partial.find('\n') {
                    let line = partial[..pos].trim_end_matches('\r').to_string();
                    partial.drain(..=pos);
                    let mut guard = lines_bg.lock().expect("ring buffer poisoned");
                    if guard.len() >= MAX_RING_LINES {
                        guard.pop_front();
                    }
                    guard.push_back(line);
                }
            }
            if !partial.is_empty() {
                let mut guard = lines_bg.lock().expect("ring buffer poisoned");
                if guard.len() >= MAX_RING_LINES {
                    guard.pop_front();
                }
                guard.push_back(partial);
            }
        });

        self.sessions.lock().expect("sessions poisoned").insert(
            id.clone(),
            PtySession {
                master: pair.master,
                writer,
                lines,
            },
        );
        Ok(id)
    }

    pub fn write_input(&self, id: &TerminalId, data: &[u8]) -> Result<(), TerminalError> {
        let mut guard = self.sessions.lock().expect("sessions poisoned");
        let sess = guard
            .get_mut(id)
            .ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        sess.writer
            .write_all(data)
            .and_then(|_| sess.writer.flush())
            .map_err(|e| TerminalError::Pty(e.to_string()))
    }

    pub fn resize(&self, id: &TerminalId, cols: u16, rows: u16) -> Result<(), TerminalError> {
        let guard = self.sessions.lock().expect("sessions poisoned");
        let sess = guard
            .get(id)
            .ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        sess.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::Pty(e.to_string()))
    }

    pub fn snapshot_lines(&self, id: &TerminalId) -> Result<Vec<String>, TerminalError> {
        let guard = self.sessions.lock().expect("sessions poisoned");
        let sess = guard
            .get(id)
            .ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        let ring = sess.lines.lock().expect("ring poisoned");
        Ok(ring.iter().cloned().collect())
    }

    pub fn close(&self, id: &TerminalId) -> bool {
        self.sessions
            .lock()
            .expect("sessions poisoned")
            .remove(id)
            .is_some()
    }
}

pub fn detect_default_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(comspec) = std::env::var("COMSPEC") {
            return comspec;
        }
        "powershell.exe".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pty_spawns_shell_and_captures_output() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mgr = TerminalManager::new();
        let id = mgr.spawn_shell(dir.path(), 80, 24).expect("spawn pty");
        mgr.write_input(&id, b"echo LINGXI_PTY_OK\nexit\n")
            .expect("write");
        let mut found = false;
        for _ in 0..40 {
            let lines = mgr.snapshot_lines(&id).expect("lines");
            if lines.iter().any(|l| l.contains("LINGXI_PTY_OK")) {
                found = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(found, "PTY output did not contain expected marker");
        assert!(mgr.close(&id));
    }
}
