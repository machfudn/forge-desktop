use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};

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

pub fn start_device_flow(client_id: &str) -> Result<DeviceFlowInit, String> {
    let cid = client_id.trim();
    if cid.is_empty() {
        return Err("GitHub Client ID tidak boleh kosong".into());
    }

    let payload = format!("client_id={cid}&scope=repo,read:user");
    let output = Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "https://github.com/login/device/code",
            "-H",
            "Accept: application/json",
            "-d",
            &payload,
        ])
        .output()
        .map_err(|e| format!("Gagal memanggil OAuth device endpoint: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<DeviceFlowInit>(&stdout)
        .map_err(|e| format!("Gagal membaca respon device flow: {e} ({stdout})"))
}

pub fn poll_device_token(client_id: &str, device_code: &str) -> Result<TokenPollResult, String> {
    let cid = client_id.trim();
    let dcode = device_code.trim();

    let payload = format!(
        "client_id={cid}&device_code={dcode}&grant_type=urn:ietf:params:oauth:grant-type:device_code"
    );

    let output = Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "https://github.com/login/oauth/access_token",
            "-H",
            "Accept: application/json",
            "-d",
            &payload,
        ])
        .output()
        .map_err(|e| format!("Gagal memeriksa status otorisasi: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<TokenPollResult>(&stdout)
        .map_err(|e| format!("Gagal membaca status token: {e} ({stdout})"))
}

pub fn configure_git_credentials(username: &str, token: &str) -> Result<(), String> {
    let user = username.trim();
    let tok = token.trim();
    if user.is_empty() || tok.is_empty() {
        return Err("Username dan token tidak boleh kosong".into());
    }

    // Configure global credential.helper to store if not set
    let _ = Command::new("git")
        .args(["config", "--global", "credential.helper", "store"])
        .status();

    // Append to ~/.git-credentials
    if let Some(home) = dirs_home() {
        let cred_file = home.join(".git-credentials");
        let line = format!("https://{user}:{tok}@github.com\n");

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&cred_file)
            .map_err(|e| format!("Gagal membuka file credentials: {e}"))?;

        file.write_all(line.as_bytes())
            .map_err(|e| format!("Gagal menyimpan credentials: {e}"))?;
    }

    Ok(())
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_blank_client_id() {
        assert!(start_device_flow("   ").is_err());
    }

    #[test]
    fn reject_blank_credentials() {
        assert!(configure_git_credentials("", "token").is_err());
        assert!(configure_git_credentials("user", "").is_err());
    }
}
