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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRunInfo {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRepoParams {
    pub name: String,
    pub description: Option<String>,
    pub private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_notes: Option<String>,
    pub html_url: Option<String>,
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

pub fn get_commit_check_runs(
    owner: &str,
    repo: &str,
    commit_sha: &str,
    token: Option<&str>,
) -> Result<Vec<CheckRunInfo>, String> {
    let o = owner.trim();
    let r = repo.trim();
    let sha = commit_sha.trim();
    if o.is_empty() || r.is_empty() || sha.is_empty() {
        return Err("Owner, repo, dan commit sha tidak boleh kosong".into());
    }

    let url = format!("https://api.github.com/repos/{o}/{r}/commits/{sha}/check-runs");
    let mut cmd = Command::new("curl");
    cmd.args([
        "-s",
        "-H",
        "User-Agent: ForgeDesktop/1.1.0",
        "-H",
        "Accept: application/vnd.github.v3+json",
    ]);

    let tok = token.map(|t| t.to_string()).or_else(get_stored_github_token);
    if let Some(ref t) = tok {
        cmd.args(["-H", &format!("Authorization: Bearer {t}")]);
    }
    cmd.arg(&url);

    let output = cmd
        .output()
        .map_err(|e| format!("Gagal menghubungi GitHub Check Runs API: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    #[derive(Deserialize)]
    struct RawCheckResponse {
        check_runs: Option<Vec<RawCheckRun>>,
    }
    #[derive(Deserialize)]
    struct RawCheckRun {
        id: Option<u64>,
        name: Option<String>,
        status: Option<String>,
        conclusion: Option<String>,
        html_url: Option<String>,
    }

    if let Ok(resp) = serde_json::from_str::<RawCheckResponse>(&stdout) {
        if let Some(runs) = resp.check_runs {
            return Ok(runs
                .into_iter()
                .filter_map(|c| {
                    Some(CheckRunInfo {
                        id: c.id?,
                        name: c.name.unwrap_or_else(|| "check".into()),
                        status: c.status.unwrap_or_else(|| "unknown".into()),
                        conclusion: c.conclusion,
                        html_url: c.html_url,
                    })
                })
                .collect());
        }
    }
    Ok(vec![])
}

pub fn publish_repository_to_github(
    repo_path: &str,
    params: PublishRepoParams,
    token: Option<&str>,
) -> Result<String, String> {
    let tok = token
        .map(|s| s.to_string())
        .or_else(get_stored_github_token)
        .ok_or_else(|| {
            "Token autentikasi GitHub tidak ditemukan. Silakan login ke GitHub terlebih dahulu."
                .to_string()
        })?;

    let name = params.name.trim();
    if name.is_empty() {
        return Err("Nama repository GitHub tidak boleh kosong".into());
    }

    let payload = serde_json::json!({
        "name": name,
        "description": params.description.as_deref().unwrap_or(""),
        "private": params.private,
        "auto_init": false
    });
    let payload_str = payload.to_string();

    let mut cmd = Command::new("curl");
    cmd.args([
        "-s",
        "-X",
        "POST",
        "https://api.github.com/user/repos",
        "-H",
        "User-Agent: ForgeDesktop/1.1.0",
        "-H",
        "Accept: application/vnd.github.v3+json",
        "-H",
        &format!("Authorization: Bearer {tok}"),
        "-H",
        "Content-Type: application/json",
        "-d",
        &payload_str,
    ]);

    let output = cmd
        .output()
        .map_err(|e| format!("Gagal menghubungi GitHub API: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    #[derive(Deserialize)]
    struct RepoCreateResponse {
        clone_url: Option<String>,
        message: Option<String>,
    }

    let parsed: RepoCreateResponse = serde_json::from_str(&stdout)
        .map_err(|e| format!("Gagal membaca response GitHub API: {e} ({stdout})"))?;

    if let Some(err_msg) = parsed.message {
        return Err(format!("GitHub API Error: {err_msg}"));
    }

    let clone_url = parsed
        .clone_url
        .ok_or_else(|| "Gagal mendapatkan clone URL dari repositori yang dibuat".to_string())?;

    // Add remote 'origin'
    let _ = cli::run(repo_path, &["remote", "add", "origin", &clone_url]);

    // Push current branch
    let current_branch_raw = cli::run(repo_path, &["branch", "--show-current"])?;
    let branch = current_branch_raw.trim();
    let branch_name = if branch.is_empty() { "main" } else { branch };

    cli::run(repo_path, &["push", "-u", "origin", branch_name])?;

    Ok(clone_url)
}

pub fn check_app_update(token: Option<&str>) -> Result<AppUpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let tok = token.map(|t| t.to_string()).or_else(get_stored_github_token);

    let mut cmd = Command::new("curl");
    cmd.args([
        "-s",
        "https://api.github.com/repos/machfudin/github-desktop/releases/latest",
        "-H", "User-Agent: ForgeDesktop/1.1.0",
        "-H", "Accept: application/vnd.github.v3+json",
    ]);
    if let Some(ref t) = tok {
        cmd.args(["-H", &format!("Authorization: Bearer {t}")]);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Gagal menghubungi GitHub API: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    #[derive(Deserialize)]
    struct ReleaseResp {
        tag_name: Option<String>,
        body: Option<String>,
        html_url: Option<String>,
    }

    if let Ok(rel) = serde_json::from_str::<ReleaseResp>(&stdout) {
        if let Some(tag) = rel.tag_name {
            let clean_tag = tag.trim_start_matches('v').trim();
            let has_update = clean_tag != current_version && !clean_tag.is_empty();
            return Ok(AppUpdateInfo {
                current_version,
                latest_version: clean_tag.to_string(),
                has_update,
                release_notes: rel.body,
                html_url: rel.html_url,
            });
        }
    }

    Ok(AppUpdateInfo {
        current_version: current_version.clone(),
        latest_version: current_version,
        has_update: false,
        release_notes: None,
        html_url: None,
    })
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
