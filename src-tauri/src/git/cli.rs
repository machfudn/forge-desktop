use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static ASKPASS_COUNTER: AtomicU64 = AtomicU64::new(1);

pub struct AskpassGuard {
    path: PathBuf,
}

impl AskpassGuard {
    pub fn new() -> Result<Self, String> {
        let id = ASKPASS_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let ext = if cfg!(windows) { "bat" } else { "sh" };
        let path = std::env::temp_dir().join(format!("forge-askpass-{pid}-{id}.{ext}"));

        let script = Self::script_content();
        fs::write(&path, script.as_bytes())
            .map_err(|e| format!("Gagal menyiapkan helper autentikasi Git: {e}"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o700);
            fs::set_permissions(&path, perms)
                .map_err(|e| format!("Gagal mengatur izin helper autentikasi Git: {e}"))?;
        }

        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn script_content() -> &'static str {
        if cfg!(windows) {
            "@echo off\r\nsetlocal EnableDelayedExpansion\r\nset \"p=%~1\"\r\nif /I not \"!p:username=!\"==\"!p!\" (\r\n  echo(!FORGE_GIT_ASKPASS_USER!\r\n) else (\r\n  echo(!FORGE_GIT_ASKPASS_TOKEN!\r\n)\r\n"
        } else {
            "#!/bin/sh\ncase \"$1\" in\n  *Username*|*username*) printf '%s\\n' \"$FORGE_GIT_ASKPASS_USER\" ;;\n  *) printf '%s\\n' \"$FORGE_GIT_ASKPASS_TOKEN\" ;;\nesac\n"
        }
    }
}

impl Drop for AskpassGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn apply_git_askpass(
    cmd: &mut Command,
    creds: Option<(&str, &str)>,
) -> Result<Option<AskpassGuard>, String> {
    let Some((username, token)) = creds else {
        return Ok(None);
    };
    let guard = AskpassGuard::new()?;
    cmd.env("GIT_ASKPASS", guard.path())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .env("FORGE_GIT_ASKPASS_USER", username)
        .env("FORGE_GIT_ASKPASS_TOKEN", token);
    Ok(Some(guard))
}

pub fn run_raw_with_credentials(
    dir: &str,
    args: &[&str],
    creds: Option<(&str, &str)>,
) -> Result<String, String> {
    let p = Path::new(dir);
    if !p.exists() {
        return Err("Direktori tidak ditemukan".into());
    }
    let mut cmd = crate::system::command_no_window("git");
    cmd.args(["-C", dir]).args(args);
    let _guard = apply_git_askpass(&mut cmd, creds)?;
    let output = cmd
        .output()
        .map_err(|error| format!("git tidak tersedia: {error}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if err.is_empty() {
            "Perintah git gagal".into()
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(&['\r', '\n'][..])
        .to_owned())
}

pub fn run_raw(dir: &str, args: &[&str]) -> Result<String, String> {
    run_raw_with_credentials(dir, args, None)
}

pub fn run_with_credentials(
    repo: &str,
    args: &[&str],
    creds: Option<(&str, &str)>,
) -> Result<String, String> {
    if !Path::new(repo).join(".git").exists() {
        return Err("Folder bukan repository Git".into());
    }
    run_raw_with_credentials(repo, args, creds)
}

pub fn run(repo: &str, args: &[&str]) -> Result<String, String> {
    run_with_credentials(repo, args, None)
}

pub fn run_bytes(repo: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    if !Path::new(repo).join(".git").exists() {
        return Err("Folder bukan repository Git".into());
    }
    let output = crate::system::command_no_window("git")
        .args(["-C", repo])
        .args(args)
        .output()
        .map_err(|error| format!("git tidak tersedia: {error}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if err.is_empty() {
            "Perintah git gagal".into()
        } else {
            err
        });
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_non_git_dir() {
        let res = run("/tmp/nonexistent-git-repo-test-12345", &["status"]);
        assert!(res.is_err());
    }

    #[test]
    fn askpass_guard_cleans_up_on_drop_and_contains_no_secret() {
        let path_copy: PathBuf;
        {
            let guard = AskpassGuard::new().unwrap();
            path_copy = guard.path().to_path_buf();
            assert!(path_copy.exists());
            let content = fs::read_to_string(&path_copy).unwrap();
            assert!(content.contains("FORGE_GIT_ASKPASS_USER"));
            assert!(content.contains("FORGE_GIT_ASKPASS_TOKEN"));
        }
        assert!(!path_copy.exists());
    }
}
