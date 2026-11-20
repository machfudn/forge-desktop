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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailableShell {
    pub id: String,
    pub label: String,
    pub path: String,
    pub is_installed: bool,
    pub is_default: bool,
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

        let (mut cmd, shell_desc) = build_shell_command(trimmed, shell_kind);
        cmd.current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(ch) => ch,
            Err(err) => {
                let (mut fallback_cmd, fb_desc) = build_fallback_command(trimmed);
                fallback_cmd
                    .current_dir(&cwd)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                match fallback_cmd.spawn() {
                    Ok(ch) => {
                        let _ = app.emit(
                            "terminal-output",
                            TerminalOutputEvent {
                                session_id: session_id.to_string(),
                                stream: "system".into(),
                                text: format!(
                                    "[Info: Shell '{}' gagal ({:?}). Beralih otomatis ke {}]\n",
                                    shell_desc,
                                    err.kind(),
                                    fb_desc
                                ),
                            },
                        );
                        ch
                    }
                    Err(err2) => {
                        return Err(format!(
                            "Gagal menjalankan terminal: shell '{}' gagal ({err}), fallback '{}' juga gagal ({err2})",
                            shell_desc, fb_desc
                        ));
                    }
                }
            }
        };

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

fn find_executable(name_or_path: &str) -> Option<PathBuf> {
    let p = Path::new(name_or_path);
    if p.is_absolute() {
        if p.is_file() {
            return Some(p.to_path_buf());
        }
        return None;
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name_or_path);
            if candidate.is_file() {
                return Some(candidate);
            }
            #[cfg(target_os = "windows")]
            {
                if !name_or_path.to_lowercase().ends_with(".exe") {
                    let with_exe = dir.join(format!("{name_or_path}.exe"));
                    if with_exe.is_file() {
                        return Some(with_exe);
                    }
                    let with_cmd = dir.join(format!("{name_or_path}.cmd"));
                    if with_cmd.is_file() {
                        return Some(with_cmd);
                    }
                }
            }
        }
    }
    None
}

pub fn detect_available_shells() -> Vec<AvailableShell> {
    let mut detected = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // 1. PowerShell 7 (Core) - pwsh
        let pwsh_candidates = [
            r"C:\Program Files\PowerShell\7\pwsh.exe",
            r"C:\Program Files\PowerShell\6\pwsh.exe",
        ];
        let pwsh_found = find_executable("pwsh.exe").or_else(|| {
            pwsh_candidates.iter().find_map(|p| {
                let pb = PathBuf::from(p);
                pb.is_file().then_some(pb)
            })
        });
        if let Some(p) = pwsh_found {
            detected.push(AvailableShell {
                id: "pwsh".into(),
                label: "PowerShell 7 (pwsh)".into(),
                path: p.to_string_lossy().to_string(),
                is_installed: true,
                is_default: true,
            });
        }

        // 2. Windows PowerShell (powershell.exe)
        let ps_default_path = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";
        let ps_found = find_executable("powershell.exe").or_else(|| {
            let pb = PathBuf::from(ps_default_path);
            pb.is_file().then_some(pb)
        });
        if let Some(p) = ps_found {
            let is_def = detected.is_empty();
            detected.push(AvailableShell {
                id: "powershell".into(),
                label: "Windows PowerShell".into(),
                path: p.to_string_lossy().to_string(),
                is_installed: true,
                is_default: is_def,
            });
        }

        // 3. Command Prompt (cmd.exe)
        let cmd_default_path = r"C:\Windows\System32\cmd.exe";
        let cmd_found = find_executable("cmd.exe").or_else(|| {
            let pb = PathBuf::from(cmd_default_path);
            pb.is_file().then_some(pb)
        });
        if let Some(p) = cmd_found {
            let is_def = detected.is_empty();
            detected.push(AvailableShell {
                id: "cmd".into(),
                label: "Command Prompt (CMD)".into(),
                path: p.to_string_lossy().to_string(),
                is_installed: true,
                is_default: is_def,
            });
        }

        // 4. Git Bash (bash.exe)
        let git_bash_candidates = [
            r"C:\Program Files\Git\bin\bash.exe",
            r"C:\Program Files\Git\usr\bin\bash.exe",
            r"C:\Program Files (x86)\Git\bin\bash.exe",
        ];
        let mut git_bash_found = git_bash_candidates.iter().find_map(|p| {
            let pb = PathBuf::from(p);
            pb.is_file().then_some(pb)
        });
        if git_bash_found.is_none() {
            if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
                let user_git = PathBuf::from(local_app_data).join(r"Programs\Git\bin\bash.exe");
                if user_git.is_file() {
                    git_bash_found = Some(user_git);
                }
            }
        }
        if git_bash_found.is_none() {
            if let Some(found) = find_executable("bash.exe") {
                if !found
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("windowsapps")
                {
                    git_bash_found = Some(found);
                }
            }
        }
        if let Some(p) = git_bash_found {
            detected.push(AvailableShell {
                id: "bash".into(),
                label: "Git Bash".into(),
                path: p.to_string_lossy().to_string(),
                is_installed: true,
                is_default: false,
            });
        }

        // 5. WSL (wsl.exe)
        let wsl_path = r"C:\Windows\System32\wsl.exe";
        let wsl_found = find_executable("wsl.exe").or_else(|| {
            let pb = PathBuf::from(wsl_path);
            pb.is_file().then_some(pb)
        });
        if let Some(p) = wsl_found {
            detected.push(AvailableShell {
                id: "wsl".into(),
                label: "WSL (Linux)".into(),
                path: p.to_string_lossy().to_string(),
                is_installed: true,
                is_default: false,
            });
        }

        // 6. Nushell (nu.exe)
        if let Some(p) = find_executable("nu.exe") {
            detected.push(AvailableShell {
                id: "nu".into(),
                label: "Nushell".into(),
                path: p.to_string_lossy().to_string(),
                is_installed: true,
                is_default: false,
            });
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let unix_shells = [
            ("zsh", "Zsh", &["/bin/zsh", "/usr/bin/zsh", "/usr/local/bin/zsh", "/opt/homebrew/bin/zsh"][..]),
            ("bash", "Bash", &["/bin/bash", "/usr/bin/bash", "/usr/local/bin/bash", "/opt/homebrew/bin/bash"][..]),
            ("fish", "Fish", &["/usr/bin/fish", "/usr/local/bin/fish", "/opt/homebrew/bin/fish"][..]),
            ("pwsh", "PowerShell", &["/usr/bin/pwsh", "/usr/local/bin/pwsh", "/opt/homebrew/bin/pwsh"][..]),
            ("nu", "Nushell", &["/usr/bin/nu", "/usr/local/bin/nu", "/opt/homebrew/bin/nu"][..]),
            ("sh", "POSIX sh", &["/bin/sh", "/usr/bin/sh"][..]),
        ];

        let default_env_shell = std::env::var("SHELL").ok();

        for (id, label, paths) in unix_shells {
            let found = find_executable(id).or_else(|| {
                paths.iter().find_map(|p| {
                    let pb = PathBuf::from(p);
                    pb.is_file().then_some(pb)
                })
            });
            if let Some(p) = found {
                let path_str = p.to_string_lossy().to_string();
                let is_def = default_env_shell.as_deref() == Some(&path_str) || detected.is_empty();
                detected.push(AvailableShell {
                    id: id.into(),
                    label: label.into(),
                    path: path_str,
                    is_installed: true,
                    is_default: is_def,
                });
            }
        }
    }

    let best_label = detected
        .iter()
        .find(|s| s.is_default)
        .or_else(|| detected.first())
        .map(|s| s.label.clone())
        .unwrap_or_else(|| {
            #[cfg(target_os = "windows")]
            {
                "PowerShell".to_string()
            }
            #[cfg(not(target_os = "windows"))]
            {
                "bash".to_string()
            }
        });

    let mut result = vec![AvailableShell {
        id: "auto".into(),
        label: format!("Auto ({best_label})"),
        path: "auto".into(),
        is_installed: true,
        is_default: true,
    }];
    result.extend(detected);
    result
}

fn resolve_shell_path(kind: &str) -> Option<String> {
    let shells = detect_available_shells();
    shells
        .into_iter()
        .find(|s| s.id == kind && s.id != "auto")
        .map(|s| s.path)
}

fn build_shell_command(command: &str, shell_kind: Option<&str>) -> (std::process::Command, String) {
    let kind = shell_kind.unwrap_or("auto");
    #[cfg(target_os = "windows")]
    {
        match kind {
            "pwsh" => {
                let bin = resolve_shell_path("pwsh").unwrap_or_else(|| "pwsh.exe".into());
                let mut c = crate::system::command_no_window(&bin);
                c.args(["-NoLogo", "-NoProfile", "-Command", command]);
                (c, "PowerShell 7 (pwsh)".into())
            }
            "powershell" => {
                let bin =
                    resolve_shell_path("powershell").unwrap_or_else(|| "powershell.exe".into());
                let mut c = crate::system::command_no_window(&bin);
                c.args(["-NoLogo", "-NoProfile", "-Command", command]);
                (c, "Windows PowerShell".into())
            }
            "cmd" => {
                let bin = resolve_shell_path("cmd").unwrap_or_else(|| "cmd.exe".into());
                let mut c = crate::system::command_no_window(&bin);
                c.args(["/C", command]);
                (c, "Command Prompt (CMD)".into())
            }
            "bash" | "git-bash" => {
                let bin = resolve_shell_path("bash").unwrap_or_else(|| "bash.exe".into());
                let mut c = crate::system::command_no_window(&bin);
                c.args(["-lc", command]);
                (c, "Git Bash".into())
            }
            "wsl" => {
                let bin = resolve_shell_path("wsl").unwrap_or_else(|| "wsl.exe".into());
                let mut c = crate::system::command_no_window(&bin);
                c.args(["-e", "sh", "-c", command]);
                (c, "WSL".into())
            }
            "nu" => {
                let bin = resolve_shell_path("nu").unwrap_or_else(|| "nu.exe".into());
                let mut c = crate::system::command_no_window(&bin);
                c.args(["-c", command]);
                (c, "Nushell".into())
            }
            _ => {
                if let Some(pwsh_p) = resolve_shell_path("pwsh") {
                    let mut c = crate::system::command_no_window(&pwsh_p);
                    c.args(["-NoLogo", "-NoProfile", "-Command", command]);
                    (c, "PowerShell 7 (pwsh)".into())
                } else if let Some(ps_p) = resolve_shell_path("powershell") {
                    let mut c = crate::system::command_no_window(&ps_p);
                    c.args(["-NoLogo", "-NoProfile", "-Command", command]);
                    (c, "Windows PowerShell".into())
                } else {
                    let mut c = crate::system::command_no_window("cmd.exe");
                    c.args(["/C", command]);
                    (c, "Command Prompt (CMD)".into())
                }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let bin = match kind {
            "zsh" => resolve_shell_path("zsh").unwrap_or_else(|| "zsh".into()),
            "fish" => resolve_shell_path("fish").unwrap_or_else(|| "fish".into()),
            "pwsh" => resolve_shell_path("pwsh").unwrap_or_else(|| "pwsh".into()),
            "nu" => resolve_shell_path("nu").unwrap_or_else(|| "nu".into()),
            "sh" => resolve_shell_path("sh").unwrap_or_else(|| "sh".into()),
            _ => resolve_shell_path("bash").unwrap_or_else(|| "bash".into()),
        };
        let mut c = crate::system::command_no_window(&bin);
        c.args(["-c", command]);
        let desc = kind.to_string();
        (c, desc)
    }
}

fn build_fallback_command(command: &str) -> (std::process::Command, String) {
    #[cfg(target_os = "windows")]
    {
        let ps_path = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";
        if Path::new(ps_path).is_file() {
            let mut c = crate::system::command_no_window(ps_path);
            c.args(["-NoLogo", "-NoProfile", "-Command", command]);
            (c, "Windows PowerShell".into())
        } else {
            let mut c = crate::system::command_no_window("cmd.exe");
            c.args(["/C", command]);
            (c, "Command Prompt (CMD)".into())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut c = crate::system::command_no_window("sh");
        c.args(["-c", command]);
        (c, "POSIX sh".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_available_shells_always_returns_auto_and_installed() {
        let shells = detect_available_shells();
        assert!(!shells.is_empty(), "Daftar shell tidak boleh kosong");
        assert_eq!(shells[0].id, "auto", "Elemen pertama harus selalu auto");
        assert!(
            shells.iter().any(|s| s.is_installed && s.id != "auto"),
            "Harus ada minimal satu shell terinstall selain auto"
        );
    }

    #[test]
    fn test_build_shell_command_fallback_never_panics() {
        let (cmd, desc) = build_fallback_command("git status");
        assert!(!desc.is_empty());
        let program = cmd.get_program().to_string_lossy().to_string();
        assert!(!program.is_empty());
    }

    #[test]
    fn test_build_shell_command_auto_resolves_successfully() {
        let (cmd, desc) = build_shell_command("echo test", None);
        assert!(!desc.is_empty());
        let program = cmd.get_program().to_string_lossy().to_string();
        assert!(!program.is_empty());
    }
}
