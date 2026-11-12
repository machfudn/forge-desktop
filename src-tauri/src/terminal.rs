use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutputEvent {
    pub session_id: String,
    pub stream: String, // "stdout" | "stderr" | "system"
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalExitEvent {
    pub session_id: String,
    pub exit_code: i32,
    pub cwd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalExecResult {
    pub session_id: String,
    pub exit_code: i32,
    pub cwd: String,
}

pub struct TerminalManager {
    cwds: Mutex<HashMap<String, PathBuf>>,
    active_children: Mutex<HashMap<String, Arc<Mutex<Option<Child>>>>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            cwds: Mutex::new(HashMap::new()),
            active_children: Mutex::new(HashMap::new()),
        }
    }

    pub fn resolve_cwd(&self, session_id: &str, repo_fallback: &str) -> PathBuf {
        if let Ok(map) = self.cwds.lock() {
            if let Some(p) = map.get(session_id) {
                if p.exists() {
                    return p.clone();
                }
            }
        }
        let p = PathBuf::from(repo_fallback);
        if p.exists() {
            p
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    }

    pub fn kill_session(&self, session_id: &str) -> Result<bool, String> {
        let child_arc = {
            let map = self
                .active_children
                .lock()
                .map_err(|_| "Terminal state terkunci".to_string())?;
            map.get(session_id).cloned()
        };
        if let Some(arc) = child_arc {
            let mut guard = arc.lock().map_err(|_| "Child handle terkunci".to_string())?;
            if let Some(ref mut child) = *guard {
                let _ = child.kill();
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn run_command(
        &self,
        app: &AppHandle,
        session_id: &str,
        repo_path: &str,
        command: &str,
        shell_kind: Option<&str>,
    ) -> Result<TerminalExecResult, String> {
        let trimmed = command.trim();
        let mut cwd = self.resolve_cwd(session_id, repo_path);

        if trimmed.is_empty() {
            return Ok(TerminalExecResult {
                session_id: session_id.to_string(),
                exit_code: 0,
                cwd: cwd.to_string_lossy().to_string(),
            });
        }

        // Handle pure `cd` command directly so stateful directory navigation persists cleanly
        if trimmed == "cd" || trimmed.starts_with("cd ") || trimmed.starts_with("cd\t") {
            let target_raw = trimmed[2..].trim().trim_matches('"').trim_matches('\'');
            let next_dir = if target_raw.is_empty() || target_raw == "~" {
                dirs_home().unwrap_or_else(|| PathBuf::from(repo_path))
            } else {
                let candidate = Path::new(target_raw);
                if candidate.is_absolute() {
                    candidate.to_path_buf()
                } else {
                    cwd.join(candidate)
                }
            };

            if next_dir.is_dir() {
                cwd = next_dir.canonicalize().unwrap_or(next_dir);
                if let Ok(mut map) = self.cwds.lock() {
                    map.insert(session_id.to_string(), cwd.clone());
                }
                let cwd_str = strip_win_verbatim(&cwd);
                let _ = app.emit(
                    "terminal-output",
                    TerminalOutputEvent {
                        session_id: session_id.to_string(),
                        stream: "system".into(),
                        text: format!("cwd -> {cwd_str}\n"),
                    },
                );
                let _ = app.emit(
                    "terminal-exit",
                    TerminalExitEvent {
                        session_id: session_id.to_string(),
                        exit_code: 0,
                        cwd: cwd_str.clone(),
                    },
                );
                return Ok(TerminalExecResult {
                    session_id: session_id.to_string(),
                    exit_code: 0,
                    cwd: cwd_str,
                });
            } else {
                let msg = format!("cd: direktori tidak ditemukan: {target_raw}\n");
                let cwd_str = strip_win_verbatim(&cwd);
                let _ = app.emit(
                    "terminal-output",
                    TerminalOutputEvent {
                        session_id: session_id.to_string(),
                        stream: "stderr".into(),
                        text: msg,
                    },
                );
                let _ = app.emit(
                    "terminal-exit",
                    TerminalExitEvent {
                        session_id: session_id.to_string(),
                        exit_code: 1,
                        cwd: cwd_str.clone(),
                    },
                );
                return Ok(TerminalExecResult {
                    session_id: session_id.to_string(),
                    exit_code: 1,
                    cwd: cwd_str,
                });
            }
        }

        let mut cmd = build_shell_command(trimmed, shell_kind);
        cmd.current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Gagal menjalankan terminal: {e}"))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let child_slot = Arc::new(Mutex::new(Some(child)));
        if let Ok(mut map) = self.active_children.lock() {
            map.insert(session_id.to_string(), Arc::clone(&child_slot));
        }

        let app_out = app.clone();
        let sid_out = session_id.to_string();
        let out_handle = std::thread::spawn(move || {
            if let Some(s) = stdout {
                let mut reader = BufReader::new(s);
                let mut buf = String::new();
                while let Ok(n) = reader.read_line(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let _ = app_out.emit(
                        "terminal-output",
                        TerminalOutputEvent {
                            session_id: sid_out.clone(),
                            stream: "stdout".into(),
                            text: buf.clone(),
                        },
                    );
                    buf.clear();
                }
            }
        });

        let app_err = app.clone();
        let sid_err = session_id.to_string();
        let err_handle = std::thread::spawn(move || {
            if let Some(s) = stderr {
                let mut reader = BufReader::new(s);
                let mut buf = String::new();
                while let Ok(n) = reader.read_line(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let _ = app_err.emit(
                        "terminal-output",
                        TerminalOutputEvent {
                            session_id: sid_err.clone(),
                            stream: "stderr".into(),
                            text: buf.clone(),
                        },
                    );
                    buf.clear();
                }
            }
        });

        let status_code = loop {
            let exited = {
                let mut guard = child_slot
                    .lock()
                    .map_err(|_| "Child terminal terkunci".to_string())?;
                if let Some(ref mut ch) = *guard {
                    match ch.try_wait() {
                        Ok(Some(st)) => {
                            *guard = None;
                            Some(st.code().unwrap_or(-1))
                        }
                        Ok(None) => None,
                        Err(e) => {
                            *guard = None;
                            return Err(format!("Gagal menunggu proses terminal: {e}"));
                        }
                    }
                } else {
                    Some(-1)
                }
            };
            if let Some(code) = exited {
                break code;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        };

        let _ = out_handle.join();
        let _ = err_handle.join();

        if let Ok(mut map) = self.active_children.lock() {
            map.remove(session_id);
        }

        let cwd_str = strip_win_verbatim(&cwd);
        let _ = app.emit(
            "terminal-exit",
            TerminalExitEvent {
                session_id: session_id.to_string(),
                exit_code: status_code,
                cwd: cwd_str.clone(),
            },
        );

        Ok(TerminalExecResult {
            session_id: session_id.to_string(),
            exit_code: status_code,
            cwd: cwd_str,
        })
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

fn strip_win_verbatim(p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

fn build_shell_command(command: &str, shell_kind: Option<&str>) -> std::process::Command {
    let kind = shell_kind.unwrap_or("auto");
    #[cfg(target_os = "windows")]
    {
        match kind {
            "cmd" => {
                let mut c = crate::system::command_no_window("cmd");
                c.args(["/C", command]);
                c
            }
            "bash" => {
                let mut c = crate::system::command_no_window("bash");
                c.args(["-lc", command]);
                c
            }
            _ => {
                let mut c = crate::system::command_no_window("powershell");
                c.args(["-NoLogo", "-NoProfile", "-Command", command]);
                c
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let bin = match kind {
            "zsh" => "zsh",
            "sh" => "sh",
            _ => "bash",
        };
        let mut c = crate::system::command_no_window(bin);
        c.args(["-c", command]);
        c
    }
}
