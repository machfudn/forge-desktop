use super::accounts::{
    normalize_github_host, AccountManager, AccountProfile, AuthMethod,
};
use super::cli;
use super::github::{parse_curl_http_response, IssueInfo, PullRequestInfo};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostingProvider {
    Github,
    Gitlab,
    Bitbucket,
    AzureDevops,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedHostingRepo {
    pub provider: HostingProvider,
    pub host: String,
    pub namespace: String, // GitHub owner, GitLab group/subgroup, Bitbucket workspace, Azure org/project
    pub repo: String,
    pub web_url: String,
}

fn url_encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn split_host_and_path(remote_url: &str) -> Option<(String, String)> {
    let raw = remote_url.trim();
    if raw.is_empty() {
        return None;
    }
    let trimmed = raw.strip_suffix(".git").unwrap_or(raw);

    if let Some(rest) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    {
        let without_auth = rest.split_once('@').map(|(_, r)| r).unwrap_or(rest);
        let (host, path) = without_auth.split_once('/')?;
        let host_clean = host.split(':').next()?.to_ascii_lowercase();
        return Some((host_clean, path.trim_matches('/').to_string()));
    }

    if let Some(rest) = trimmed.strip_prefix("ssh://") {
        let without_user = rest.split_once('@').map(|(_, r)| r).unwrap_or(rest);
        let (host, path) = without_user.split_once('/')?;
        let host_clean = host.split(':').next()?.to_ascii_lowercase();
        // Azure SSH URLs often start with v3/org/project/repo
        return Some((host_clean, path.trim_matches('/').to_string()));
    }

    if let Some(rest) = trimmed.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        return Some((
            host.to_ascii_lowercase(),
            path.trim_matches('/').to_string(),
        ));
    }

    if let Some((host, path)) = trimmed.split_once(':') {
        if !host.contains('/') && !host.contains('\\') && host.contains('.') {
            return Some((
                host.split('@').next_back()?.to_ascii_lowercase(),
                path.trim_matches('/').to_string(),
            ));
        }
    }

    None
}

pub fn parse_hosting_repo(remote_url: &str) -> Option<ParsedHostingRepo> {
    let (host, path) = split_host_and_path(remote_url)?;
    if host.is_empty() || path.is_empty() {
        return None;
    }

    if host == "dev.azure.com" || host.ends_with(".visualstudio.com") || host == "ssh.dev.azure.com"
    {
        let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        // HTTPS: dev.azure.com/{org}/{project}/_git/{repo}
        if let Some(git_idx) = segs.iter().position(|&s| s == "_git") {
            if git_idx >= 1 && git_idx + 1 < segs.len() {
                let repo = segs[git_idx + 1].to_string();
                let namespace = segs[..git_idx].join("/");
                let web_host = if host == "ssh.dev.azure.com" {
                    "dev.azure.com"
                } else {
                    host.as_str()
                };
                return Some(ParsedHostingRepo {
                    provider: HostingProvider::AzureDevops,
                    host: web_host.to_string(),
                    namespace: namespace.clone(),
                    repo: repo.clone(),
                    web_url: format!("https://{web_host}/{namespace}/_git/{repo}"),
                });
            }
        }
        // SSH: ssh.dev.azure.com:v3/{org}/{project}/{repo}
        if segs.len() >= 4 && segs[0] == "v3" {
            let org = segs[1];
            let project = segs[2];
            let repo = segs[3];
            let namespace = format!("{org}/{project}");
            return Some(ParsedHostingRepo {
                provider: HostingProvider::AzureDevops,
                host: "dev.azure.com".into(),
                namespace: namespace.clone(),
                repo: repo.to_string(),
                web_url: format!("https://dev.azure.com/{namespace}/_git/{repo}"),
            });
        }
    }

    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segs.len() < 2 {
        return None;
    }

    if host == "bitbucket.org" || host.contains("bitbucket") {
        let namespace = segs[..segs.len() - 1].join("/");
        let repo = segs[segs.len() - 1].to_string();
        return Some(ParsedHostingRepo {
            provider: HostingProvider::Bitbucket,
            host: host.clone(),
            namespace: namespace.clone(),
            repo: repo.clone(),
            web_url: format!("https://{host}/{namespace}/{repo}"),
        });
    }

    if host == "gitlab.com" || host.contains("gitlab") {
        let namespace = segs[..segs.len() - 1].join("/");
        let repo = segs[segs.len() - 1].to_string();
        return Some(ParsedHostingRepo {
            provider: HostingProvider::Gitlab,
            host: host.clone(),
            namespace: namespace.clone(),
            repo: repo.clone(),
            web_url: format!("https://{host}/{namespace}/{repo}"),
        });
    }

    let namespace = segs[0].to_string();
    let repo = segs[1].to_string();
    Some(ParsedHostingRepo {
        provider: HostingProvider::Github,
        host: host.clone(),
        namespace: namespace.clone(),
        repo: repo.clone(),
        web_url: format!("https://{host}/{namespace}/{repo}"),
    })
}

fn provider_http_get(url: &str, auth_header: Option<&str>) -> Result<(u16, String), String> {
    let mut cmd = crate::system::command_no_window("curl");
    cmd.args(["-s", "-i", "--max-time", "30", "-K", "-", url])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Gagal menghubungi server provider Git: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let mut cfg = format!(
            "header = \"User-Agent: ForgeDesktop/{}\"\nheader = \"Accept: application/json\"\n",
            env!("CARGO_PKG_VERSION")
        );
        if let Some(auth) = auth_header {
            if !auth.contains(['\r', '\n', '"']) {
                cfg.push_str(&format!("header = \"{auth}\"\n"));
            }
        }
        let _ = stdin.write_all(cfg.as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Gagal membaca respon server provider Git: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "Gagal terhubung ke provider Git".into()
        } else {
            err
        });
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let parsed = parse_curl_http_response(&raw);
    Ok((parsed.status_code, parsed.body))
}

fn build_auth_header(provider: HostingProvider, token: &str) -> String {
    let t = token.trim();
    match provider {
        HostingProvider::Github => format!("Authorization: Bearer {t}"),
        HostingProvider::Gitlab => format!("Authorization: Bearer {t}"),
        HostingProvider::Bitbucket => format!("Authorization: Bearer {t}"),
        HostingProvider::AzureDevops => {
            let b64 = base64_encode(format!(":{t}").as_bytes());
            format!("Authorization: Basic {b64}")
        }
    }
}

pub fn connect_provider_pat(
    accounts: &AccountManager,
    provider: HostingProvider,
    host: &str,
    token: &str,
    label: Option<&str>,
    allow_memory_fallback: bool,
) -> Result<AccountProfile, String> {
    if provider == HostingProvider::Github {
        return super::github::verify_and_connect_pat(
            accounts,
            Some(host),
            token,
            label,
            allow_memory_fallback,
        );
    }

    let norm_host = normalize_github_host(host)?;
    let tok = token.trim();
    if tok.is_empty() {
        return Err("Token akses tidak boleh kosong".into());
    }
    let auth_hdr = build_auth_header(provider, tok);

    let (login, name, email, avatar_url, html_url) = match provider {
        HostingProvider::Gitlab => {
            let url = format!("https://{norm_host}/api/v4/user");
            let (status, body) = provider_http_get(&url, Some(&auth_hdr))?;
            if status == 401 || status == 403 {
                return Err("Token GitLab tidak valid atau kedaluwarsa".into());
            }
            if status >= 400 || status == 0 {
                return Err(format!("GitLab API error HTTP {status}"));
            }
            let v: serde_json::Value =
                serde_json::from_str(&body).map_err(|e| format!("Respon GitLab invalid: {e}"))?;
            let login = v
                .get("username")
                .and_then(|s| s.as_str())
                .ok_or("Username GitLab tidak ditemukan")?
                .to_string();
            (
                login,
                v.get("name").and_then(|s| s.as_str()).map(ToString::to_string),
                v.get("email").and_then(|s| s.as_str()).map(ToString::to_string),
                v.get("avatar_url").and_then(|s| s.as_str()).map(ToString::to_string),
                v.get("web_url").and_then(|s| s.as_str()).map(ToString::to_string),
            )
        }
        HostingProvider::Bitbucket => {
            let url = if norm_host == "bitbucket.org" {
                "https://api.bitbucket.org/2.0/user".to_string()
            } else {
                format!("https://{norm_host}/rest/api/1.0/users")
            };
            let (status, body) = provider_http_get(&url, Some(&auth_hdr))?;
            if status == 401 || status == 403 {
                return Err("Token Bitbucket tidak valid atau kedaluwarsa".into());
            }
            if status >= 400 || status == 0 {
                return Err(format!("Bitbucket API error HTTP {status}"));
            }
            let v: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| format!("Respon Bitbucket invalid: {e}"))?;
            let login = v
                .get("username")
                .or_else(|| v.get("nickname"))
                .and_then(|s| s.as_str())
                .unwrap_or("bitbucket-user")
                .to_string();
            (
                login,
                v.get("display_name")
                    .and_then(|s| s.as_str())
                    .map(ToString::to_string),
                None,
                v.get("links")
                    .and_then(|l| l.get("avatar"))
                    .and_then(|a| a.get("href"))
                    .and_then(|h| h.as_str())
                    .map(ToString::to_string),
                v.get("links")
                    .and_then(|l| l.get("html"))
                    .and_then(|a| a.get("href"))
                    .and_then(|h| h.as_str())
                    .map(ToString::to_string),
            )
        }
        HostingProvider::AzureDevops => {
            // Verify token cleanly against Azure DevOps profile endpoint or allow org-scoped PAT
            let url = "https://app.vssps.visualstudio.com/_apis/profile/profiles/me?api-version=7.0";
            let (status, body) = provider_http_get(url, Some(&auth_hdr))?;
            if status == 401 || status == 403 {
                return Err("PAT Azure DevOps tidak valid atau kedaluwarsa".into());
            }
            let (login, display) = if status >= 200 && status < 300 {
                let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
                let email = v
                    .get("emailAddress")
                    .and_then(|s| s.as_str())
                    .unwrap_or("azure-user");
                let disp = v
                    .get("displayName")
                    .and_then(|s| s.as_str())
                    .map(ToString::to_string);
                (email.to_string(), disp)
            } else {
                ("azure-pat".to_string(), Some("Azure DevOps PAT".to_string()))
            };
            (
                login,
                display,
                None,
                None,
                Some(format!("https://{norm_host}")),
            )
        }
        HostingProvider::Github => unreachable!(),
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let profile = AccountProfile {
        id: format!("{norm_host}/{}", login.to_ascii_lowercase()),
        host: norm_host,
        login,
        name,
        email,
        avatar_url,
        html_url,
        label: label
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string),
        auth_method: AuthMethod::Pat,
        added_at: now,
        memory_only: false,
    };
    accounts.upsert_account(profile, tok, allow_memory_fallback)
}

pub fn list_provider_pull_requests(
    accounts: &AccountManager,
    remote_url: &str,
    repo_path: Option<&str>,
) -> Result<Vec<PullRequestInfo>, String> {
    let parsed = parse_hosting_repo(remote_url)
        .ok_or_else(|| "URL remote tidak dikenali sebagai GitHub/GitLab/Bitbucket/Azure".to_string())?;
    let token = accounts
        .resolve_account_for_repo(repo_path, &parsed.host)
        .map(|(_, t)| t);

    match parsed.provider {
        HostingProvider::Github => super::github::list_pull_requests(
            &parsed.host,
            &parsed.namespace,
            &parsed.repo,
            token.as_deref(),
        ),
        HostingProvider::Gitlab => {
            let project_path = url_encode_component(&format!("{}/{}", parsed.namespace, parsed.repo));
            let url = format!(
                "https://{}/api/v4/projects/{project_path}/merge_requests?state=opened&per_page=30",
                parsed.host
            );
            let auth = token.as_deref().map(|t| build_auth_header(HostingProvider::Gitlab, t));
            let (status, body) = provider_http_get(&url, auth.as_deref())?;
            if status >= 400 || status == 0 {
                return Err(format!("GitLab MR API error HTTP {status}"));
            }
            let arr: Vec<serde_json::Value> = serde_json::from_str(&body)
                .map_err(|e| format!("Gagal parse GitLab MR: {e}"))?;
            Ok(arr
                .into_iter()
                .map(|m| PullRequestInfo {
                    id: m.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                    number: m.get("iid").and_then(|v| v.as_u64()).unwrap_or(0),
                    title: m.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
                    author: m
                        .get("author")
                        .and_then(|a| a.get("username"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("unknown")
                        .into(),
                    head_branch: m
                        .get("source_branch")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                    base_branch: m
                        .get("target_branch")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                    html_url: m.get("web_url").and_then(|v| v.as_str()).unwrap_or("").into(),
                    created_at: m
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                })
                .collect())
        }
        HostingProvider::Bitbucket => {
            let url = format!(
                "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests?state=OPEN",
                parsed.namespace, parsed.repo
            );
            let auth = token.as_deref().map(|t| build_auth_header(HostingProvider::Bitbucket, t));
            let (status, body) = provider_http_get(&url, auth.as_deref())?;
            if status >= 400 || status == 0 {
                return Err(format!("Bitbucket PR API error HTTP {status}"));
            }
            let root: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| format!("Gagal parse Bitbucket PR: {e}"))?;
            let vals = root
                .get("values")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            Ok(vals
                .into_iter()
                .map(|p| PullRequestInfo {
                    id: p.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                    number: p.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                    title: p.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
                    author: p
                        .get("author")
                        .and_then(|a| a.get("display_name"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("unknown")
                        .into(),
                    head_branch: p
                        .get("source")
                        .and_then(|s| s.get("branch"))
                        .and_then(|b| b.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .into(),
                    base_branch: p
                        .get("destination")
                        .and_then(|s| s.get("branch"))
                        .and_then(|b| b.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .into(),
                    html_url: p
                        .get("links")
                        .and_then(|l| l.get("html"))
                        .and_then(|h| h.get("href"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .into(),
                    created_at: p
                        .get("created_on")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                })
                .collect())
        }
        HostingProvider::AzureDevops => {
            let url = format!(
                "https://{}/{}/_apis/git/repositories/{}/pullrequests?searchCriteria.status=active&api-version=7.0",
                parsed.host, parsed.namespace, parsed.repo
            );
            let auth = token
                .as_deref()
                .map(|t| build_auth_header(HostingProvider::AzureDevops, t));
            let (status, body) = provider_http_get(&url, auth.as_deref())?;
            if status >= 400 || status == 0 {
                return Err(format!("Azure DevOps PR API error HTTP {status}"));
            }
            let root: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| format!("Gagal parse Azure PR: {e}"))?;
            let vals = root
                .get("value")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            Ok(vals
                .into_iter()
                .map(|p| {
                    let id = p.get("pullRequestId").and_then(|v| v.as_u64()).unwrap_or(0);
                    let src = p
                        .get("sourceRefName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .strip_prefix("refs/heads/")
                        .unwrap_or("");
                    let dst = p
                        .get("targetRefName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .strip_prefix("refs/heads/")
                        .unwrap_or("");
                    PullRequestInfo {
                        id,
                        number: id,
                        title: p.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
                        author: p
                            .get("createdBy")
                            .and_then(|c| c.get("displayName"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown")
                            .into(),
                        head_branch: src.to_string(),
                        base_branch: dst.to_string(),
                        html_url: format!("{}/pullrequest/{id}", parsed.web_url),
                        created_at: p
                            .get("creationDate")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .into(),
                    }
                })
                .collect())
        }
    }
}

pub fn list_provider_issues(
    accounts: &AccountManager,
    remote_url: &str,
    repo_path: Option<&str>,
) -> Result<Vec<IssueInfo>, String> {
    let parsed = parse_hosting_repo(remote_url)
        .ok_or_else(|| "URL remote tidak dikenali".to_string())?;
    let token = accounts
        .resolve_account_for_repo(repo_path, &parsed.host)
        .map(|(_, t)| t);

    match parsed.provider {
        HostingProvider::Github => super::github::list_issues(
            &parsed.host,
            &parsed.namespace,
            &parsed.repo,
            token.as_deref(),
        ),
        HostingProvider::Gitlab => {
            let project_path = url_encode_component(&format!("{}/{}", parsed.namespace, parsed.repo));
            let url = format!(
                "https://{}/api/v4/projects/{project_path}/issues?state=opened&per_page=30",
                parsed.host
            );
            let auth = token.as_deref().map(|t| build_auth_header(HostingProvider::Gitlab, t));
            let (status, body) = provider_http_get(&url, auth.as_deref())?;
            if status >= 400 || status == 0 {
                return Err(format!("GitLab Issues API error HTTP {status}"));
            }
            let arr: Vec<serde_json::Value> = serde_json::from_str(&body)
                .map_err(|e| format!("Gagal parse GitLab issues: {e}"))?;
            Ok(arr
                .into_iter()
                .map(|it| IssueInfo {
                    id: it.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                    number: it.get("iid").and_then(|v| v.as_u64()).unwrap_or(0),
                    title: it.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
                    author: it
                        .get("author")
                        .and_then(|a| a.get("username"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("unknown")
                        .into(),
                    html_url: it.get("web_url").and_then(|v| v.as_str()).unwrap_or("").into(),
                    comments_count: it
                        .get("user_notes_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                })
                .collect())
        }
        HostingProvider::Bitbucket => {
            let url = format!(
                "https://api.bitbucket.org/2.0/repositories/{}/{}/issues?q=state%3D%22open%22",
                parsed.namespace, parsed.repo
            );
            let auth = token.as_deref().map(|t| build_auth_header(HostingProvider::Bitbucket, t));
            let (status, body) = provider_http_get(&url, auth.as_deref())?;
            if status == 404 {
                return Ok(Vec::new());
            }
            if status >= 400 || status == 0 {
                return Err(format!("Bitbucket Issues API error HTTP {status}"));
            }
            let root: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| format!("Gagal parse Bitbucket issues: {e}"))?;
            let vals = root
                .get("values")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            Ok(vals
                .into_iter()
                .map(|it| IssueInfo {
                    id: it.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                    number: it.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                    title: it.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
                    author: it
                        .get("reporter")
                        .and_then(|a| a.get("display_name"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("unknown")
                        .into(),
                    html_url: it
                        .get("links")
                        .and_then(|l| l.get("html"))
                        .and_then(|h| h.get("href"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .into(),
                    comments_count: 0,
                })
                .collect())
        }
        HostingProvider::AzureDevops => Ok(Vec::new()),
    }
}

pub fn checkout_provider_pull_request(
    repo_path: &str,
    provider: HostingProvider,
    pr_number: u64,
    head_branch: &str,
    creds: Option<(String, String)>,
) -> Result<String, String> {
    match provider {
        HostingProvider::Github => super::github::checkout_pull_request(
            repo_path,
            pr_number,
            Some("origin"),
            creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
        ),
        HostingProvider::Gitlab => {
            let local_branch = format!("mr-{pr_number}");
            let refspec = format!("merge-requests/{pr_number}/head:{local_branch}");
            cli::run_with_credentials(
                repo_path,
                &["fetch", "origin", &refspec],
                creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
            )?;
            cli::run(repo_path, &["checkout", &local_branch])?;
            Ok(local_branch)
        }
        HostingProvider::Bitbucket | HostingProvider::AzureDevops => {
            let clean_branch = head_branch.trim();
            if clean_branch.is_empty() || clean_branch.starts_with('-') {
                return Err("Nama branch PR tidak valid".into());
            }
            cli::run_with_credentials(
                repo_path,
                &["fetch", "origin", clean_branch],
                creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
            )?;
            let local_branch = format!("pr-{pr_number}");
            let _ = cli::run(
                repo_path,
                &["checkout", "-B", &local_branch, &format!("origin/{clean_branch}")],
            )?;
            Ok(local_branch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_github_gitlab_bitbucket_azure_urls() {
        let gl = parse_hosting_repo("git@gitlab.com:acme/platform/backend.git").unwrap();
        assert_eq!(gl.provider, HostingProvider::Gitlab);
        assert_eq!(gl.namespace, "acme/platform");
        assert_eq!(gl.repo, "backend");

        let bb = parse_hosting_repo("https://bitbucket.org/teamx/webapp.git").unwrap();
        assert_eq!(bb.provider, HostingProvider::Bitbucket);
        assert_eq!(bb.namespace, "teamx");
        assert_eq!(bb.repo, "webapp");

        let az = parse_hosting_repo("https://dev.azure.com/myorg/myproj/_git/service-api").unwrap();
        assert_eq!(az.provider, HostingProvider::AzureDevops);
        assert_eq!(az.namespace, "myorg/myproj");
        assert_eq!(az.repo, "service-api");
        assert_eq!(
            az.web_url,
            "https://dev.azure.com/myorg/myproj/_git/service-api"
        );

        let gh = parse_hosting_repo("https://github.com/octocat/hello-world.git").unwrap();
        assert_eq!(gh.provider, HostingProvider::Github);
        assert_eq!(gh.namespace, "octocat");
        assert_eq!(gh.repo, "hello-world");
    }
}
