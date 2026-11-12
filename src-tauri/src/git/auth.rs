use super::accounts::{github_web_base, normalize_github_host};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFlowInit {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPollResult {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalSessionSource {
    GhCli,
    GitCredentials,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedSession {
    pub source: LocalSessionSource,
    pub host: String,
    pub username_hint: Option<String>,
    pub display: String,
}

fn post_form_json(url: &str, payload: &str, err_ctx: &str) -> Result<String, String> {
    let output = crate::system::command_no_window("curl")
        .args([
            "-s",
            "-X",
            "POST",
            url,
            "-H",
            "Accept: application/json",
            "-d",
            payload,
        ])
        .output()
        .map_err(|e| format!("{err_ctx}: {e}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn start_device_flow(host: Option<&str>, client_id: &str) -> Result<DeviceFlowInit, String> {
    let norm_host = normalize_github_host(host.unwrap_or("github.com"))?;
    let cid = client_id.trim();
    if cid.is_empty() || cid.chars().any(|c| c.is_whitespace() || c == '&') {
        return Err("GitHub Client ID tidak boleh kosong atau mengandung spasi".into());
    }

    let web_base = github_web_base(&norm_host)?;
    let url = format!("{web_base}/login/device/code");
    let payload = format!("client_id={cid}&scope=repo,read:user,user:email");
    let stdout = post_form_json(&url, &payload, "Gagal memanggil OAuth device endpoint")?;
    serde_json::from_str::<DeviceFlowInit>(&stdout)
        .map_err(|e| format!("Gagal membaca respon device flow: {e} ({stdout})"))
}

pub fn poll_device_token_raw(
    host: Option<&str>,
    client_id: &str,
    device_code: &str,
) -> Result<TokenPollResult, String> {
    let norm_host = normalize_github_host(host.unwrap_or("github.com"))?;
    let cid = client_id.trim();
    let dcode = device_code.trim();
    if cid.is_empty() || dcode.is_empty() || cid.contains('&') || dcode.contains('&') {
        return Err("Client ID dan device code tidak valid".into());
    }

    let web_base = github_web_base(&norm_host)?;
    let url = format!("{web_base}/login/oauth/access_token");
    let payload = format!(
        "client_id={cid}&device_code={dcode}&grant_type=urn:ietf:params:oauth:grant-type:device_code"
    );
    let stdout = post_form_json(&url, &payload, "Gagal memeriksa status otorisasi")?;
    serde_json::from_str::<TokenPollResult>(&stdout)
        .map_err(|e| format!("Gagal membaca status token: {e} ({stdout})"))
}

pub fn parse_github_credentials_for_host(content: &str, target_host: &str) -> Vec<(String, String)> {
    let want_host = target_host.trim().to_ascii_lowercase();
    let mut found = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let without_scheme = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"));
        let Some(rest) = without_scheme else {
            continue;
        };
        let Some((creds_part, host_and_path)) = rest.split_once('@') else {
            continue;
        };
        let entry_host = host_and_path
            .split('/')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if entry_host == want_host {
            if let Some((user, token)) = creds_part.split_once(':') {
                if !token.is_empty() {
                    found.push((user.to_string(), token.to_string()));
                }
            }
        }
    }
    found
}

fn read_gh_cli_token(host: &str) -> Option<String> {
    let out = crate::system::command_no_window("gh")
        .args(["auth", "token", "--hostname", host])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let tok = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if tok.is_empty() { None } else { Some(tok) }
}

fn read_git_credentials_entry(host: &str) -> Option<(String, String)> {
    let home = home_dir()?;
    let cred_path = home.join(".git-credentials");
    let content = fs::read_to_string(&cred_path).ok()?;
    parse_github_credentials_for_host(&content, host).into_iter().next()
}

pub fn detect_local_github_sessions(host: &str) -> Result<Vec<DetectedSession>, String> {
    let norm_host = normalize_github_host(host)?;
    let mut sessions = Vec::new();

    if read_gh_cli_token(&norm_host).is_some() {
        sessions.push(DetectedSession {
            source: LocalSessionSource::GhCli,
            host: norm_host.clone(),
            username_hint: None,
            display: format!("Sesi aktif gh CLI ({norm_host})"),
        });
    }

    if let Some((user, _)) = read_git_credentials_entry(&norm_host) {
        let hint = if user != "oauth" && !user.is_empty() {
            Some(user.clone())
        } else {
            None
        };
        let label = match &hint {
            Some(u) => format!("~/.git-credentials (@{u})"),
            None => format!("~/.git-credentials ({norm_host})"),
        };
        sessions.push(DetectedSession {
            source: LocalSessionSource::GitCredentials,
            host: norm_host,
            username_hint: hint,
            display: label,
        });
    }

    Ok(sessions)
}

pub fn read_local_session_token(
    host: &str,
    source: &LocalSessionSource,
) -> Result<String, String> {
    let norm_host = normalize_github_host(host)?;
    match source {
        LocalSessionSource::GhCli => read_gh_cli_token(&norm_host).ok_or_else(|| {
            format!("Tidak ada sesi aktif `gh` CLI untuk {norm_host}")
        }),
        LocalSessionSource::GitCredentials => read_git_credentials_entry(&norm_host)
            .map(|(_, tok)| tok)
            .ok_or_else(|| {
                format!("Tidak ada kredensial tersimpan di ~/.git-credentials untuk {norm_host}")
            }),
    }
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            let drive = std::env::var_os("HOMEDRIVE")?;
            let path = std::env::var_os("HOMEPATH")?;
            let mut full = PathBuf::from(drive);
            full.push(path);
            Some(full)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_blank_client_id() {
        assert!(start_device_flow(None, "   ").is_err());
    }

    #[test]
    fn parses_github_credentials_by_host() {
        let sample = "https://gitlab.example.com\nhttps://octocat:ghp_pub@github.com\nhttps://budi:ghp_ghe@git.kantor.id/org\n";
        assert_eq!(
            parse_github_credentials_for_host(sample, "github.com"),
            vec![("octocat".to_string(), "ghp_pub".to_string())]
        );
        assert_eq!(
            parse_github_credentials_for_host(sample, "git.kantor.id"),
            vec![("budi".to_string(), "ghp_ghe".to_string())]
        );
        assert!(parse_github_credentials_for_host(sample, "other.com").is_empty());
    }
}
