use super::accounts::{
    github_api_base, make_account_id, normalize_github_host, AccountHealth, AccountManager,
    AccountProfile, AuthMethod, GITHUB_PUBLIC_HOST,
};
use super::auth::{poll_device_token_raw, read_local_session_token, LocalSessionSource};
use super::cli;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedGitHubRepo {
    pub host: String,
    pub owner: String,
    pub repo: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceFlowPollOutcome {
    pub status: String,
    pub profile: Option<AccountProfile>,
    pub error: Option<String>,
    pub sso_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubHttpResponse {
    pub status_code: u16,
    pub sso_url: Option<String>,
    pub body: String,
}

pub fn parse_github_repo_with_hosts(
    remote_url: &str,
    registered_hosts: &[String],
) -> Option<ParsedGitHubRepo> {
    let raw = remote_url.trim();
    if raw.is_empty() {
        return None;
    }

    let trimmed = raw.strip_suffix(".git").unwrap_or(raw);
    let mut allowed: Vec<String> = vec![GITHUB_PUBLIC_HOST.to_string()];
    for h in registered_hosts {
        let lower = h.trim().to_ascii_lowercase();
        if !lower.is_empty() && !allowed.contains(&lower) {
            allowed.push(lower);
        }
    }

    if let Some(rest) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    {
        let mut parts = rest.split('/');
        let host = parts.next()?.to_ascii_lowercase();
        let owner = parts.next()?;
        let repo = parts.next()?;
        if allowed.contains(&host) && !owner.is_empty() && !repo.is_empty() {
            return Some(ParsedGitHubRepo {
                host,
                owner: owner.to_string(),
                repo: repo.to_string(),
            });
        }
    }

    if let Some(rest) = trimmed.strip_prefix("ssh://git@") {
        let mut parts = rest.split('/');
        let host = parts.next()?.to_ascii_lowercase();
        let owner = parts.next()?;
        let repo = parts.next()?;
        if allowed.contains(&host) && !owner.is_empty() && !repo.is_empty() {
            return Some(ParsedGitHubRepo {
                host,
                owner: owner.to_string(),
                repo: repo.to_string(),
            });
        }
    }

    if let Some(rest) = trimmed.strip_prefix("git@") {
        let (host_part, path_part) = rest.split_once(':')?;
        let host = host_part.to_ascii_lowercase();
        let mut parts = path_part.split('/');
        let owner = parts.next()?;
        let repo = parts.next()?;
        if allowed.contains(&host) && !owner.is_empty() && !repo.is_empty() {
            return Some(ParsedGitHubRepo {
                host,
                owner: owner.to_string(),
                repo: repo.to_string(),
            });
        }
    }

    None
}

pub fn parse_github_repo(remote_url: &str) -> Option<(String, String)> {
    parse_github_repo_with_hosts(remote_url, &[]).map(|p| (p.owner, p.repo))
}

fn validate_github_ident<'a>(val: &'a str, label: &str) -> Result<&'a str, String> {
    let trimmed = val.trim();
    if trimmed.is_empty()
        || trimmed.contains(['/', '\\', '?', '#', '&', ' ', '\t', '\r', '\n'])
        || trimmed == "."
        || trimmed == ".."
    {
        return Err(format!("{label} GitHub tidak valid"));
    }
    Ok(trimmed)
}

pub fn extract_sso_url_from_header(header_val: &str) -> Option<String> {
    for part in header_val.split(';') {
        let trimmed = part.trim();
        if let Some(url) = trimmed.strip_prefix("url=") {
            let clean = url.trim().trim_matches('"');
            if clean.starts_with("https://") {
                return Some(clean.to_string());
            }
        }
    }
    None
}

pub fn parse_curl_http_response(raw: &str) -> GitHubHttpResponse {
    let mut rest = raw;
    loop {
        let split_pos = rest
            .find("\r\n\r\n")
            .map(|i| (i, 4))
            .or_else(|| rest.find("\n\n").map(|i| (i, 2)));
        let Some((idx, sep_len)) = split_pos else {
            return GitHubHttpResponse {
                status_code: 0,
                sso_url: None,
                body: rest.to_string(),
            };
        };

        let header_block = &rest[..idx];
        let body_candidate = &rest[idx + sep_len..];

        let mut status_code: u16 = 0;
        let mut sso_url: Option<String> = None;
        for (line_idx, line) in header_block.lines().enumerate() {
            let l = line.trim_end_matches('\r');
            if line_idx == 0 && l.starts_with("HTTP/") {
                if let Some(code_str) = l.split_whitespace().nth(1) {
                    status_code = code_str.parse::<u16>().unwrap_or(0);
                }
            } else if let Some((k, v)) = l.split_once(':') {
                if k.trim().eq_ignore_ascii_case("x-github-sso") {
                    sso_url = extract_sso_url_from_header(v);
                }
            }
        }

        // Skip intermediate 1xx or proxy 200 tunnel headers if another HTTP response follows
        let is_intermediate = (status_code >= 100 && status_code < 200)
            || (status_code == 200 && body_candidate.starts_with("HTTP/"));
        if is_intermediate && body_candidate.starts_with("HTTP/") {
            rest = body_candidate;
            continue;
        }

        return GitHubHttpResponse {
            status_code,
            sso_url,
            body: body_candidate.to_string(),
        };
    }
}

fn github_api_request_detailed(
    method: &str,
    url: &str,
    json_body: Option<&str>,
    token: Option<&str>,
) -> Result<GitHubHttpResponse, String> {
    let mut cmd = crate::system::command_no_window("curl");
    cmd.args(["-s", "-i", "-X", method, "-K", "-", url]);
    if let Some(body) = json_body {
        cmd.args(["-d", body]);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Gagal menghubungi GitHub API: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let mut cfg = format!(
            "header = \"User-Agent: ForgeDesktop/{}\"\nheader = \"Accept: application/vnd.github.v3+json\"\n",
            env!("CARGO_PKG_VERSION")
        );
        if json_body.is_some() {
            cfg.push_str("header = \"Content-Type: application/json\"\n");
        }
        if let Some(t) = token.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            if !t.contains(['\r', '\n', '"']) {
                cfg.push_str(&format!("header = \"Authorization: Bearer {t}\"\n"));
            }
        }
        let _ = stdin.write_all(cfg.as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Gagal membaca respon GitHub API: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "Gagal terhubung ke server GitHub API".into()
        } else {
            err
        });
    }

    let raw = String::from_utf8_lossy(&output.stdout).into_owned();
    let parsed = parse_curl_http_response(&raw);
    if parsed.status_code == 0 {
        return Err("Tidak menerima respon HTTP valid dari GitHub API".into());
    }
    Ok(parsed)
}

fn format_api_http_error(resp: &GitHubHttpResponse) -> String {
    #[derive(Deserialize)]
    struct ApiMsg {
        message: Option<String>,
    }
    let msg = serde_json::from_str::<ApiMsg>(&resp.body)
        .ok()
        .and_then(|m| m.message)
        .unwrap_or_else(|| format!("HTTP {}", resp.status_code));

    if resp.status_code == 401 {
        return format!("[AUTH_EXPIRED] Token tidak valid atau kedaluwarsa ({msg}).");
    }
    if resp.status_code == 403
        && (resp.sso_url.is_some() || msg.to_ascii_lowercase().contains("saml"))
    {
        let prefix = match &resp.sso_url {
            Some(url) => format!("[SSO_REQUIRED:{url}]"),
            None => "[SSO_REQUIRED:]".to_string(),
        };
        return format!(
            "{prefix} Organisasi mewajibkan otorisasi SAML SSO untuk token ini ({msg})."
        );
    }
    format!("GitHub API error ({}): {msg}", resp.status_code)
}

pub(crate) fn github_api_request(
    method: &str,
    url: &str,
    json_body: Option<&str>,
    token: Option<&str>,
) -> Result<String, String> {
    let resp = github_api_request_detailed(method, url, json_body, token)?;
    if resp.status_code >= 400 {
        return Err(format_api_http_error(&resp));
    }
    Ok(resp.body)
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn verify_account_on_host(
    host: &str,
    token: &str,
    label: Option<&str>,
    auth_method: AuthMethod,
) -> Result<AccountProfile, String> {
    let norm_host = normalize_github_host(host)?;
    let tok = token.trim();
    if tok.is_empty() || tok.contains(['\r', '\n', '"']) {
        return Err("Token GitHub tidak boleh kosong atau mengandung karakter invalid".into());
    }

    let api_base = github_api_base(&norm_host)?;
    let user_url = format!("{api_base}/user");
    let resp = github_api_request_detailed("GET", &user_url, None, Some(tok))?;
    if resp.status_code >= 400 {
        return Err(format_api_http_error(&resp));
    }

    #[derive(Deserialize)]
    struct RawUserResp {
        login: Option<String>,
        name: Option<String>,
        email: Option<String>,
        avatar_url: Option<String>,
        html_url: Option<String>,
    }

    let parsed: RawUserResp = serde_json::from_str(&resp.body)
        .map_err(|e| format!("Gagal membaca profil akun dari {norm_host}: {e}"))?;
    let login = parsed
        .login
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Respon GitHub API tidak memuat username login".to_string())?;

    let mut email = parsed.email.filter(|e| !e.trim().is_empty());
    if email.is_none() {
        let emails_url = format!("{api_base}/user/emails");
        if let Ok(email_body) = github_api_request("GET", &emails_url, None, Some(tok)) {
            #[derive(Deserialize)]
            struct RawEmail {
                email: String,
                primary: Option<bool>,
                verified: Option<bool>,
            }
            if let Ok(list) = serde_json::from_str::<Vec<RawEmail>>(&email_body) {
                email = list
                    .iter()
                    .find(|e| e.primary.unwrap_or(false) && e.verified.unwrap_or(false))
                    .or_else(|| list.iter().find(|e| e.primary.unwrap_or(false)))
                    .or_else(|| list.first())
                    .map(|e| e.email.clone());
            }
        }
    }

    let clean_label = label
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    Ok(AccountProfile {
        id: make_account_id(&norm_host, &login),
        host: norm_host,
        login,
        name: parsed.name.filter(|s| !s.trim().is_empty()),
        email,
        avatar_url: parsed.avatar_url.filter(|s| !s.trim().is_empty()),
        html_url: parsed.html_url.filter(|s| !s.trim().is_empty()),
        label: clean_label,
        auth_method,
        added_at: unix_now_secs(),
        memory_only: false,
    })
}

pub fn verify_and_connect_pat(
    mgr: &AccountManager,
    host: Option<&str>,
    token: &str,
    label: Option<&str>,
    allow_memory_fallback: bool,
) -> Result<AccountProfile, String> {
    let target_host = host.unwrap_or(GITHUB_PUBLIC_HOST);
    let profile = verify_account_on_host(target_host, token, label, AuthMethod::Pat)?;
    mgr.upsert_account(profile, token.trim(), allow_memory_fallback)
}

pub fn poll_and_connect_device_flow(
    mgr: &AccountManager,
    host: Option<&str>,
    client_id: &str,
    device_code: &str,
    label: Option<&str>,
    allow_memory_fallback: bool,
) -> Result<DeviceFlowPollOutcome, String> {
    let res = poll_device_token_raw(host, client_id, device_code)?;
    if let Some(access_token) = res.access_token {
        let target_host = host.unwrap_or(GITHUB_PUBLIC_HOST);
        match verify_account_on_host(target_host, &access_token, label, AuthMethod::DeviceFlow) {
            Ok(profile) => match mgr.upsert_account(profile, &access_token, allow_memory_fallback) {
                Ok(saved) => Ok(DeviceFlowPollOutcome {
                    status: "authorized".into(),
                    profile: Some(saved),
                    error: None,
                    sso_url: None,
                }),
                Err(err) => Ok(DeviceFlowPollOutcome {
                    status: "error".into(),
                    profile: None,
                    error: Some(err),
                    sso_url: None,
                }),
            },
            Err(err) => Ok(DeviceFlowPollOutcome {
                status: "error".into(),
                profile: None,
                error: Some(err),
                sso_url: None,
            }),
        }
    } else if let Some(err_code) = res.error {
        if err_code == "authorization_pending" {
            Ok(DeviceFlowPollOutcome {
                status: "pending".into(),
                profile: None,
                error: None,
                sso_url: None,
            })
        } else if err_code == "slow_down" {
            Ok(DeviceFlowPollOutcome {
                status: "slow_down".into(),
                profile: None,
                error: None,
                sso_url: None,
            })
        } else {
            Ok(DeviceFlowPollOutcome {
                status: "error".into(),
                profile: None,
                error: Some(res.error_description.unwrap_or(err_code)),
                sso_url: None,
            })
        }
    } else {
        Ok(DeviceFlowPollOutcome {
            status: "pending".into(),
            profile: None,
            error: None,
            sso_url: None,
        })
    }
}

pub fn import_local_github_session(
    mgr: &AccountManager,
    host: &str,
    source: &LocalSessionSource,
    label: Option<&str>,
    allow_memory_fallback: bool,
) -> Result<AccountProfile, String> {
    let tok = read_local_session_token(host, source)?;
    let auth_method = match source {
        LocalSessionSource::GhCli => AuthMethod::GhCliImport,
        LocalSessionSource::GitCredentials => AuthMethod::GitCredentialsImport,
    };
    let profile = verify_account_on_host(host, &tok, label, auth_method)?;
    mgr.upsert_account(profile, &tok, allow_memory_fallback)
}

pub fn check_account_health(mgr: &AccountManager, account_id: &str) -> Result<AccountHealth, String> {
    let profile = mgr
        .get_profile(account_id)
        .ok_or_else(|| "Akun GitHub tidak ditemukan".to_string())?;
    if !profile.memory_only {
        mgr.invalidate_token_cache(account_id);
    }
    let Some(token) = mgr.get_token(account_id)? else {
        return Ok(AccountHealth::Expired {
            message: "Token tidak ditemukan di penyimpanan kredensial. Perbarui token untuk melanjutkan.".into(),
        });
    };

    let api_base = github_api_base(&profile.host)?;
    let url = format!("{api_base}/user");
    let resp = match github_api_request_detailed("GET", &url, None, Some(&token)) {
        Ok(r) => r,
        Err(err) => {
            return Ok(AccountHealth::Offline { message: err });
        }
    };

    if resp.status_code == 401 {
        return Ok(AccountHealth::Expired {
            message: "Token ditolak atau sudah kedaluwarsa (HTTP 401).".into(),
        });
    }
    if resp.status_code == 403
        && (resp.sso_url.is_some() || resp.body.to_ascii_lowercase().contains("saml"))
    {
        return Ok(AccountHealth::SsoRequired {
            message: "Token membutuhkan otorisasi SAML SSO oleh organisasi.".into(),
            sso_url: resp.sso_url,
        });
    }
    if resp.status_code >= 400 {
        return Ok(AccountHealth::Offline {
            message: format_api_http_error(&resp),
        });
    }

    if let Ok(fresh) = verify_account_on_host(
        &profile.host,
        &token,
        profile.label.as_deref(),
        profile.auth_method.clone(),
    ) {
        let mut merged = profile.clone();
        merged.name = fresh.name;
        merged.email = fresh.email;
        merged.avatar_url = fresh.avatar_url;
        merged.html_url = fresh.html_url;
        let _ = mgr.update_profile_metadata(&merged);
        return Ok(AccountHealth::Valid { profile: merged });
    }

    Ok(AccountHealth::Valid { profile })
}

pub fn list_pull_requests(
    host: &str,
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<Vec<PullRequestInfo>, String> {
    let o = validate_github_ident(owner, "Owner")?;
    let r = validate_github_ident(repo, "Nama repository")?;
    let api_base = github_api_base(host)?;

    let url = format!("{api_base}/repos/{o}/{r}/pulls?state=open&per_page=30");
    let stdout = github_api_request("GET", &url, None, token)?;
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
        .map_err(|_| "Respon GitHub API tidak valid atau limit rate tercapai".to_string())?;

    Ok(parsed
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
        .collect())
}

pub fn list_issues(
    host: &str,
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<Vec<IssueInfo>, String> {
    let o = validate_github_ident(owner, "Owner")?;
    let r = validate_github_ident(repo, "Nama repository")?;
    let api_base = github_api_base(host)?;

    let url = format!("{api_base}/repos/{o}/{r}/issues?state=open&per_page=30");
    let stdout = github_api_request("GET", &url, None, token)?;
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
        .map_err(|_| "Respon GitHub API tidak valid atau limit rate tercapai".to_string())?;

    Ok(parsed
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
        .collect())
}

pub fn checkout_pull_request(
    repo_path: &str,
    pr_number: u64,
    remote: Option<&str>,
    creds: Option<(&str, &str)>,
) -> Result<String, String> {
    let rem = remote.unwrap_or("origin");
    let refspec = format!("pull/{pr_number}/head:pr-{pr_number}");
    let target_branch = format!("pr-{pr_number}");

    cli::run_with_credentials(repo_path, &["fetch", rem, &refspec], creds)?;
    cli::run(repo_path, &["checkout", &target_branch])
}

pub fn get_commit_check_runs(
    host: &str,
    owner: &str,
    repo: &str,
    commit_sha: &str,
    token: Option<&str>,
) -> Result<Vec<CheckRunInfo>, String> {
    let o = validate_github_ident(owner, "Owner")?;
    let r = validate_github_ident(repo, "Nama repository")?;
    let sha = validate_github_ident(commit_sha, "Commit SHA")?;
    let api_base = github_api_base(host)?;

    let url = format!("{api_base}/repos/{o}/{r}/commits/{sha}/check-runs");
    let resp = github_api_request_detailed("GET", &url, None, token)?;
    parse_check_runs_response(&resp)
}

fn parse_check_runs_response(resp: &GitHubHttpResponse) -> Result<Vec<CheckRunInfo>, String> {
    if resp.status_code == 404 || resp.status_code == 422 {
        return Ok(vec![]);
    }
    if resp.status_code >= 400 {
        return Err(format_api_http_error(resp));
    }
    if resp.body.trim().is_empty() {
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

    if let Ok(parsed) = serde_json::from_str::<RawCheckResponse>(&resp.body) {
        if let Some(runs) = parsed.check_runs {
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
    mgr: &AccountManager,
    repo_path: &str,
    params: PublishRepoParams,
    account_id: Option<&str>,
    host: Option<&str>,
) -> Result<String, String> {
    let target_host = host.unwrap_or(GITHUB_PUBLIC_HOST);
    let (profile, tok) = mgr
        .resolve_account(account_id, Some(repo_path), target_host)
        .ok_or_else(|| {
            "Tidak ada akun GitHub aktif untuk publikasi. Hubungkan akun terlebih dahulu."
                .to_string()
        })?;

    let name = params.name.trim();
    if name.is_empty() {
        return Err("Nama repository GitHub tidak boleh kosong".into());
    }

    let api_base = github_api_base(&profile.host)?;
    let url = format!("{api_base}/user/repos");

    let payload = serde_json::json!({
        "name": name,
        "description": params.description.as_deref().unwrap_or(""),
        "private": params.private,
        "auto_init": false
    });
    let payload_str = payload.to_string();
    let stdout = github_api_request("POST", &url, Some(&payload_str), Some(&tok))?;

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

    let has_remote_origin = cli::run(repo_path, &["remote", "get-url", "origin"]).is_ok();
    if has_remote_origin {
        cli::run(repo_path, &["remote", "set-url", "origin", &clone_url])?;
    } else {
        cli::run(repo_path, &["remote", "add", "origin", &clone_url])?;
    }

    let current_branch_raw = cli::run(repo_path, &["branch", "--show-current"])
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| cli::run(repo_path, &["symbolic-ref", "--short", "HEAD"]).ok())
        .unwrap_or_default();
    let branch = current_branch_raw.trim();
    let branch_name = if branch.is_empty() { "main" } else { branch };

    let has_commits = cli::run(repo_path, &["rev-parse", "--verify", "HEAD"]).is_ok();
    if has_commits {
        cli::run_with_credentials(
            repo_path,
            &["push", "-u", "origin", branch_name],
            Some((&profile.login, &tok)),
        )?;
        let _ = mgr.bind_repo(repo_path, &profile.id);
        Ok(clone_url)
    } else {
        let _ = mgr.bind_repo(repo_path, &profile.id);
        Ok(format!(
            "{clone_url} (remote ditautkan; buat komit pertama lalu push)"
        ))
    }
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
    fn test_parse_enterprise_hosts() {
        let hosts = vec!["github.com".to_string(), "git.kantor.id".to_string()];
        let https = parse_github_repo_with_hosts("https://git.kantor.id/tim/api.git", &hosts).unwrap();
        assert_eq!(https.host, "git.kantor.id");
        assert_eq!(https.owner, "tim");
        assert_eq!(https.repo, "api");

        let ssh = parse_github_repo_with_hosts("git@git.kantor.id:tim/web.git", &hosts).unwrap();
        assert_eq!(ssh.host, "git.kantor.id");
        assert_eq!(ssh.owner, "tim");
        assert_eq!(ssh.repo, "web");

        assert!(parse_github_repo_with_hosts("https://unregistered.org/a/b.git", &hosts).is_none());
    }

    #[test]
    fn test_parse_invalid_repo_url() {
        assert!(parse_github_repo("https://gitlab.com/owner/repo.git").is_none());
        assert!(parse_github_repo("not-a-url").is_none());
        assert!(parse_github_repo("").is_none());
    }

    #[test]
    fn test_semver_update_comparison() {
        assert!(crate::updater::is_newer_version("1.2.0", "1.1.0"));
        assert!(crate::updater::is_newer_version("2.0.0", "1.9.9"));
        assert!(!crate::updater::is_newer_version("1.0.0", "1.1.0"));
        assert!(!crate::updater::is_newer_version("1.1.0", "1.1.0"));
    }

    #[test]
    fn test_validate_github_ident() {
        assert!(validate_github_ident("sveltejs", "Owner").is_ok());
        assert!(validate_github_ident("owner/evil", "Owner").is_err());
        assert!(validate_github_ident("repo?x=1", "Repo").is_err());
    }

    #[test]
    fn test_parse_curl_http_response_401_and_sso() {
        let raw_sso = "HTTP/1.1 200 Connection established\r\n\r\nHTTP/2 403 \r\nX-GitHub-SSO: required; url=https://github.com/orgs/acme/sso?authorization_request=123\r\nContent-Type: application/json\r\n\r\n{\"message\":\"Resource protected by organization SAML enforcement.\"}";
        let resp = parse_curl_http_response(raw_sso);
        assert_eq!(resp.status_code, 403);
        assert_eq!(
            resp.sso_url.as_deref(),
            Some("https://github.com/orgs/acme/sso?authorization_request=123")
        );
        let formatted = format_api_http_error(&resp);
        assert!(formatted.starts_with("[SSO_REQUIRED:https://github.com/orgs/acme/sso"));

        let raw_401 = "HTTP/2 401 \r\n\r\n{\"message\":\"Bad credentials\"}";
        let resp_401 = parse_curl_http_response(raw_401);
        assert_eq!(resp_401.status_code, 401);
        assert!(format_api_http_error(&resp_401).starts_with("[AUTH_EXPIRED]"));
    }

    #[test]
    fn test_parse_check_runs_response_handles_missing_commit_422_and_404() {
        let resp_422 = GitHubHttpResponse {
            status_code: 422,
            sso_url: None,
            body: "{\"message\":\"No commit found for SHA: abc\"}".into(),
        };
        assert!(parse_check_runs_response(&resp_422).unwrap().is_empty());

        let resp_404 = GitHubHttpResponse {
            status_code: 404,
            sso_url: None,
            body: "{\"message\":\"Not Found\"}".into(),
        };
        assert!(parse_check_runs_response(&resp_404).unwrap().is_empty());

        let resp_401 = GitHubHttpResponse {
            status_code: 401,
            sso_url: None,
            body: "{\"message\":\"Bad credentials\"}".into(),
        };
        assert!(parse_check_runs_response(&resp_401).is_err());

        let resp_200 = GitHubHttpResponse {
            status_code: 200,
            sso_url: None,
            body: "{\"check_runs\":[{\"id\":1,\"name\":\"build\",\"status\":\"completed\",\"conclusion\":\"success\"}]}".into(),
        };
        let runs = parse_check_runs_response(&resp_200).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].name, "build");
        assert_eq!(runs[0].conclusion.as_deref(), Some("success"));
    }
}
