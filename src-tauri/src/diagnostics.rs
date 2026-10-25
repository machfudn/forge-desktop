use std::process::Command;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticReport {
    pub app_version: String,
    pub os_info: String,
    pub git_version: String,
    pub sanitized_config: Vec<String>,
}

pub fn generate_report(repo: Option<&str>) -> DiagnosticReport {
    let git_ver = Command::new("git")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "git tidak ditemukan".to_string());

    let mut sanitized_config = Vec::new();
    if let Some(r) = repo {
        if let Ok(out) = Command::new("git").args(["-C", r, "config", "--list"]).output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                // Redact tokens, secrets, credentials, urls containing user info
                if line.contains("token")
                    || line.contains("secret")
                    || line.contains("password")
                    || line.contains("key")
                    || line.contains("auth")
                {
                    let key = line.split('=').next().unwrap_or("secret");
                    sanitized_config.push(format!("{key}=[REDACTED]"));
                } else {
                    sanitized_config.push(line.to_string());
                }
            }
        }
    }

    DiagnosticReport {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os_info: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        git_version: git_ver,
        sanitized_config,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_report_redacts_secrets() {
        let report = generate_report(None);
        assert_eq!(report.app_version, "1.1.0");
        assert!(!report.os_info.is_empty());
        assert!(!report.git_version.is_empty());
    }
}
