use std::path::Path;
use std::process::Command;
use serde::Serialize;
use super::cli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

pub fn validate_remote_url(url: &str) -> Result<(), String> {
    let u = url.trim();
    if u.is_empty() {
        return Err("URL remote tidak boleh kosong".into());
    }
    if u.starts_with('-')
        || u.contains('\n')
        || u.contains('\r')
        || u.contains(' ')
        || u.contains(';')
        || u.contains('&')
        || u.contains('|')
        || u.contains('`')
        || u.contains('$')
    {
        return Err("URL remote tidak valid".into());
    }
    if !(u.starts_with("https://")
        || u.starts_with("http://")
        || u.starts_with("git@")
        || u.starts_with("ssh://")
        || u.starts_with("file://")
        || Path::new(u).exists())
    {
        return Err("Protokol URL remote harus https, ssh, git@, atau path valid".into());
    }
    Ok(())
}

pub fn list_remotes(repo: &str) -> Result<Vec<RemoteInfo>, String> {
    let out = cli::run(repo, &["remote", "-v"])?;
    let mut remotes = Vec::new();

    for line in out.lines() {
        if line.ends_with("(fetch)") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                remotes.push(RemoteInfo {
                    name: parts[0].to_string(),
                    url: parts[1].to_string(),
                });
            }
        }
    }

    Ok(remotes)
}

pub fn fetch(repo: &str, remote: Option<&str>) -> Result<String, String> {
    let rem = remote.unwrap_or("origin");
    cli::run(repo, &["fetch", rem])
}

pub fn pull(repo: &str, remote: Option<&str>, branch: Option<&str>) -> Result<String, String> {
    let rem = remote.unwrap_or("origin");
    let mut args = vec!["pull", rem];
    if let Some(b) = branch {
        if !b.trim().is_empty() {
            args.push(b);
        }
    }
    cli::run(repo, &args)
}

pub fn push(repo: &str, remote: Option<&str>, branch: Option<&str>) -> Result<String, String> {
    let rem = remote.unwrap_or("origin");
    let mut args = vec!["push", rem];
    if let Some(b) = branch {
        if !b.trim().is_empty() {
            args.push(b);
        }
    }
    cli::run(repo, &args)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AheadBehind {
    pub ahead: usize,
    pub behind: usize,
}

pub fn get_ahead_behind(repo: &str, branch: &str) -> Result<AheadBehind, String> {
    let b = branch.trim();
    if b.is_empty() || b.starts_with('-') {
        return Ok(AheadBehind { ahead: 0, behind: 0 });
    }
    let upstream_spec = format!("{b}...{b}@{{upstream}}");
    match cli::run(repo, &["rev-list", "--left-right", "--count", &upstream_spec]) {
        Ok(out) => {
            let parts: Vec<&str> = out.split_whitespace().collect();
            if parts.len() >= 2 {
                let ahead = parts[0].parse::<usize>().unwrap_or(0);
                let behind = parts[1].parse::<usize>().unwrap_or(0);
                return Ok(AheadBehind { ahead, behind });
            }
            Ok(AheadBehind { ahead: 0, behind: 0 })
        }
        Err(_) => Ok(AheadBehind { ahead: 0, behind: 0 }),
    }
}

pub fn clone_repository(url: &str, target_dir: &str) -> Result<String, String> {
    validate_remote_url(url)?;
    let target = target_dir.trim();
    if target.is_empty() {
        return Err("Target direktori tidak boleh kosong".into());
    }

    let output = Command::new("git")
        .args(["clone", "--", url, target])
        .output()
        .map_err(|e| format!("Gagal menjalankan git clone: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if err.is_empty() {
            "Git clone gagal".into()
        } else {
            err
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_remote_urls() {
        assert!(validate_remote_url("https://github.com/foo/bar.git").is_ok());
        assert!(validate_remote_url("git@github.com:foo/bar.git").is_ok());
        assert!(validate_remote_url("ssh://git@github.com/foo/bar.git").is_ok());
    }

    #[test]
    fn test_invalid_remote_urls() {
        assert!(validate_remote_url("  ").is_err());
        assert!(validate_remote_url("-badurl").is_err());
        assert!(validate_remote_url("ftp://foo/bar").is_err());
    }
}
