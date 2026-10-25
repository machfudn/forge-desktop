use super::cli;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub head_branch: String,
    pub base_branch: String,
    pub html_url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueInfo {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub html_url: String,
    pub comments_count: u64,
}

pub fn parse_github_repo(remote_url: &str) -> Option<(String, String)> {
    let raw = remote_url.trim();
    if raw.is_empty() {
        return None;
    }

    let trimmed = raw.strip_suffix(".git").unwrap_or(raw);

    if let Some(rest) = trimmed.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    None
}

pub fn get_stored_github_token() -> Option<String> {
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let cred_path = home.join(".git-credentials");
        if cred_path.exists() {
            if let Ok(content) = fs::read_to_string(&cred_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.contains("github.com") {
                        if let Some(idx_at) = trimmed.find('@') {
                            let creds_part = &trimmed[..idx_at];
                            if let Some(idx_colon) = creds_part.rfind(':') {
                                let token = &creds_part[idx_colon + 1..];
                                if !token.is_empty() {
                                    return Some(token.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn list_pull_requests(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<Vec<PullRequestInfo>, String> {
    let o = owner.trim();
    let r = repo.trim();
    if o.is_empty() || r.is_empty() {
        return Err("Owner dan nama repository GitHub tidak boleh kosong".into());
    }

    let url = format!("https://api.github.com/repos/{o}/{r}/pulls?state=open&per_page=30");
    let mut cmd = Command::new("curl");
    cmd.args(["-s", "-H", "User-Agent: ForgeDesktop/1.1.0", "-H", "Accept: application/vnd.github.v3+json"]);

    let tok = token.map(|t| t.to_string()).or_else(get_stored_github_token);
    if let Some(ref t) = tok {
        cmd.args(["-H", &format!("Authorization: Bearer {t}")]);
    }

    cmd.arg(&url);

    let output = cmd
        .output()
        .map_err(|e| format!("Gagal menghubungi GitHub API: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    #[derive(Deserialize)]
    struct RawUser {
        login: Option<String>,
    }
    #[derive(Deserialize)]
    struct RawBranch {
        #[serde(rename = "ref")]
        ref_name: Option<String>,
    }
    #[derive(Deserialize)]
    struct RawPR {
        id: Option<u64>,
        number: Option<u64>,
        title: Option<String>,
        html_url: Option<String>,
        created_at: Option<String>,
        user: Option<RawUser>,
        head: Option<RawBranch>,
        base: Option<RawBranch>,
    }

    let parsed: Vec<RawPR> = serde_json::from_str(&stdout)
        .map_err(|_| format!("Respon GitHub API tidak valid atau limit rate tercapai"))?;

    let list = parsed
        .into_iter()
        .filter_map(|p| {
            Some(PullRequestInfo {
                id: p.id?,
                number: p.number?,
                title: p.title.unwrap_or_default(),
                author: p.user.and_then(|u| u.login).unwrap_or_else(|| "unknown".into()),
                head_branch: p.head.and_then(|h| h.ref_name).unwrap_or_default(),
                base_branch: p.base.and_then(|b| b.ref_name).unwrap_or_default(),
                html_url: p.html_url.unwrap_or_default(),
                created_at: p.created_at.unwrap_or_default(),
            })
        })
        .collect();

    Ok(list)
}

pub fn list_issues(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<Vec<IssueInfo>, String> {
    let o = owner.trim();
    let r = repo.trim();
    if o.is_empty() || r.is_empty() {
        return Err("Owner dan nama repository GitHub tidak boleh kosong".into());
    }

    let url = format!("https://api.github.com/repos/{o}/{r}/issues?state=open&per_page=30");
    let mut cmd = Command::new("curl");
    cmd.args(["-s", "-H", "User-Agent: ForgeDesktop/1.1.0", "-H", "Accept: application/vnd.github.v3+json"]);

    let tok = token.map(|t| t.to_string()).or_else(get_stored_github_token);
    if let Some(ref t) = tok {
        cmd.args(["-H", &format!("Authorization: Bearer {t}")]);
    }

    cmd.arg(&url);

    let output = cmd
        .output()
        .map_err(|e| format!("Gagal menghubungi GitHub API: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    #[derive(Deserialize)]
    struct RawUser {
        login: Option<String>,
    }
    #[derive(Deserialize)]
    struct RawIssue {
        id: Option<u64>,
        number: Option<u64>,
        title: Option<String>,
        html_url: Option<String>,
        comments: Option<u64>,
        pull_request: Option<serde_json::Value>,
        user: Option<RawUser>,
    }

    let parsed: Vec<RawIssue> = serde_json::from_str(&stdout)
        .map_err(|_| format!("Respon GitHub API tidak valid atau limit rate tercapai"))?;

    // GitHub /issues endpoint includes PRs unless filtered out
    let list = parsed
        .into_iter()
        .filter(|i| i.pull_request.is_none())
        .filter_map(|i| {
            Some(IssueInfo {
                id: i.id?,
                number: i.number?,
                title: i.title.unwrap_or_default(),
                author: i.user.and_then(|u| u.login).unwrap_or_else(|| "unknown".into()),
                html_url: i.html_url.unwrap_or_default(),
                comments_count: i.comments.unwrap_or(0),
            })
        })
        .collect();

    Ok(list)
}

pub fn checkout_pull_request(
    repo_path: &str,
    pr_number: u64,
    remote: Option<&str>,
) -> Result<String, String> {
    let rem = remote.unwrap_or("origin");
    let refspec = format!("pull/{pr_number}/head:pr-{pr_number}");
    let target_branch = format!("pr-{pr_number}");

    // Fetch the PR ref
    cli::run(repo_path, &["fetch", rem, &refspec])?;

    // Checkout the created local branch
    cli::run(repo_path, &["checkout", &target_branch])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_repo_https() {
        let (owner, repo) = parse_github_repo("https://github.com/rust-lang/rust.git").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");

        let (o2, r2) = parse_github_repo("https://github.com/machfudin/forge-desktop").unwrap();
        assert_eq!(o2, "machfudin");
        assert_eq!(r2, "forge-desktop");
    }

    #[test]
    fn test_parse_github_repo_ssh() {
        let (owner, repo) = parse_github_repo("git@github.com:sveltejs/svelte.git").unwrap();
        assert_eq!(owner, "sveltejs");
        assert_eq!(repo, "svelte");
    }

    #[test]
    fn test_parse_invalid_repo_url() {
        assert!(parse_github_repo("https://gitlab.com/owner/repo.git").is_none());
        assert!(parse_github_repo("not-a-url").is_none());
        assert!(parse_github_repo("").is_none());
    }
}
