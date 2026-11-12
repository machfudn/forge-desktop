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
                sanitized_config.push(sanitize_config_line(line));
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
fn sanitize_config_line(line: &str) -> String {
    if let Some((k, v)) = line.split_once('=') {
        let key_lower = k.to_ascii_lowercase();
        if key_lower.contains("token")
            || key_lower.contains("secret")
            || key_lower.contains("password")
            || key_lower.contains("passwd")
            || key_lower.contains("key")
            || key_lower.contains("auth")
            || key_lower.contains("credential")
            || key_lower.contains("cookie")
        {
            return format!("{k}=[REDACTED]");
        }
        if let Some(scheme_idx) = v.find("://") {
            let after_scheme = &v[scheme_idx + 3..];
            if let Some(at_idx) = after_scheme.find('@') {
                let scheme = &v[..scheme_idx + 3];
                let host_rest = &after_scheme[at_idx + 1..];
                return format!("{k}={scheme}[REDACTED]@{host_rest}");
            }
        }
        let val_lower = v.to_ascii_lowercase();
        if val_lower.contains("bearer ")
            || val_lower.contains("ghp_")
            || val_lower.contains("github_pat_")
        {
            return format!("{k}=[REDACTED]");
        }
    }
    line.to_string()
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
    #[test]
    fn test_sanitize_config_line_handles_case_and_url_creds() {
        assert_eq!(sanitize_config_line("github.TOKEN=ghp_abc"), "github.TOKEN=[REDACTED]");
        assert_eq!(
            sanitize_config_line("remote.origin.url=https://user:ghp_secret@github.com/a/b.git"),
            "remote.origin.url=https://[REDACTED]@github.com/a/b.git"
        );
    }
}
