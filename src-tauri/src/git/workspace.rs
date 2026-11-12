use super::cli;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReflogEntry {
    pub sha: String,
    pub short_sha: String,
    pub selector: String,
    pub action: String,
    pub date_relative: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmoduleInfo {
    pub path: String,
    pub sha: String,
    pub status: String, // "clean" | "modified" | "uninitialized" | "conflict"
    pub describe: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: String,
    pub head: String,
    pub branch: Option<String>,
    pub is_bare: bool,
    pub is_detached: bool,
    pub is_locked: bool,
    pub is_prunable: bool,
    pub is_main: bool,
}

pub fn parse_reflog_output(raw: &str) -> Vec<ReflogEntry> {
    raw.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\x1f').collect();
            if parts.len() < 6 {
                return None;
            }
            Some(ReflogEntry {
                sha: parts[0].trim().to_string(),
                short_sha: parts[1].trim().to_string(),
                selector: parts[2].trim().to_string(),
                action: parts[3].trim().to_string(),
                date_relative: parts[4].trim().to_string(),
                author: parts[5].trim().to_string(),
            })
        })
        .collect()
}

pub fn list_reflog(repo: &str, limit: usize) -> Result<Vec<ReflogEntry>, String> {
    let lim = limit.clamp(1, 500).to_string();
    let res = cli::run(
        repo,
        &[
            "reflog",
            "--date=relative",
            "-n",
            &lim,
            "--pretty=format:%H%x1f%h%x1f%gD%x1f%gs%x1f%gd%x1f%an",
        ],
    );
    match res {
        Ok(out) => Ok(parse_reflog_output(&out)),
        Err(err) => {
            let lower = err.to_ascii_lowercase();
            if lower.contains("does not have any commits")
                || lower.contains("bad default revision")
                || lower.contains("your current branch")
            {
                Ok(Vec::new())
            } else {
                Err(err)
            }
        }
    }
}

pub fn parse_submodule_status_line(line: &str) -> Option<SubmoduleInfo> {
    if line.len() < 2 {
        return None;
    }
    let prefix = line.chars().next()?;
    let status = match prefix {
        '-' => "uninitialized",
        '+' => "modified",
        'U' => "conflict",
        ' ' => "clean",
        _ => "clean",
    }
    .to_string();

    let rest = line[1..].trim();
    let mut parts = rest.splitn(3, ' ');
    let sha = parts.next()?.trim().to_string();
    let path = parts.next()?.trim().to_string();
    let describe = parts
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches(|c| c == '(' || c == ')')
        .to_string();

    if sha.is_empty() || path.is_empty() {
        return None;
    }
    Some(SubmoduleInfo {
        path,
        sha,
        status,
        describe,
        url: None,
    })
}

pub fn list_submodules(repo: &str) -> Result<Vec<SubmoduleInfo>, String> {
    let out = match cli::run(repo, &["submodule", "status"]) {
        Ok(s) => s,
        Err(err) => {
            if err.to_ascii_lowercase().contains("no submodule mapping") {
                return Ok(Vec::new());
            }
            return Err(err);
        }
    };
    let mut items: Vec<SubmoduleInfo> = out
        .lines()
        .filter_map(parse_submodule_status_line)
        .collect();

    for item in &mut items {
        let key = format!("submodule.{}.url", item.path);
        if let Ok(url) = cli::run(repo, &["config", "--file", ".gitmodules", "--get", &key]) {
            let trimmed = url.trim();
            if !trimmed.is_empty() {
                item.url = Some(trimmed.to_string());
            }
        }
    }
    Ok(items)
}

pub fn add_submodule(repo: &str, url: &str, path: &str) -> Result<String, String> {
    let u = url.trim();
    let p = path.trim();
    if u.is_empty() || u.starts_with('-') {
        return Err("URL submodule tidak valid".into());
    }
    if p.is_empty() || p.starts_with('-') || p.contains("..") {
        return Err("Path submodule tidak valid".into());
    }
    cli::run(repo, &["submodule", "add", u, p])
}

pub fn update_submodules(repo: &str, init: bool, recursive: bool) -> Result<String, String> {
    let mut args = vec!["submodule", "update"];
    if init {
        args.push("--init");
    }
    if recursive {
        args.push("--recursive");
    }
    cli::run(repo, &args)
}

pub fn sync_submodules(repo: &str) -> Result<String, String> {
    cli::run(repo, &["submodule", "sync", "--recursive"])
}

pub fn parse_worktree_porcelain(raw: &str) -> Vec<WorktreeInfo> {
    let mut result = Vec::new();
    let normalized = raw.replace("\r\n", "\n");
    for (idx, block) in normalized.split("\n\n").enumerate() {
        let trimmed = block.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut path = String::new();
        let mut head = String::new();
        let mut branch = None;
        let mut is_bare = false;
        let mut is_detached = false;
        let mut is_locked = false;
        let mut is_prunable = false;

        for line in trimmed.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                path = p.trim().to_string();
            } else if let Some(h) = line.strip_prefix("HEAD ") {
                head = h.trim().to_string();
            } else if let Some(b) = line.strip_prefix("branch ") {
                let b_trim = b.trim();
                branch = Some(
                    b_trim
                        .strip_prefix("refs/heads/")
                        .unwrap_or(b_trim)
                        .to_string(),
                );
            } else if line == "bare" {
                is_bare = true;
            } else if line == "detached" {
                is_detached = true;
            } else if line == "locked" || line.starts_with("locked ") {
                is_locked = true;
            } else if line == "prunable" || line.starts_with("prunable ") {
                is_prunable = true;
            }
        }
        if !path.is_empty() {
            result.push(WorktreeInfo {
                path,
                head,
                branch,
                is_bare,
                is_detached,
                is_locked,
                is_prunable,
                is_main: idx == 0,
            });
        }
    }
    result
}

pub fn list_worktrees(repo: &str) -> Result<Vec<WorktreeInfo>, String> {
    let out = cli::run(repo, &["worktree", "list", "--porcelain"])?;
    Ok(parse_worktree_porcelain(&out))
}

pub fn add_worktree(
    repo: &str,
    path: &str,
    branch: &str,
    create_new_branch: bool,
) -> Result<String, String> {
    let p = path.trim();
    let b = branch.trim();
    if p.is_empty() || p.starts_with('-') {
        return Err("Path worktree tidak valid".into());
    }
    if b.is_empty() || b.starts_with('-') {
        return Err("Nama branch worktree tidak valid".into());
    }
    if Path::new(p).exists() && std::fs::read_dir(p).map(|mut d| d.next().is_some()).unwrap_or(false)
    {
        return Err("Folder target worktree sudah berisi file".into());
    }
    if create_new_branch {
        cli::run(repo, &["worktree", "add", "-b", b, p])
    } else {
        cli::run(repo, &["worktree", "add", p, b])
    }
}

pub fn remove_worktree(repo: &str, path: &str, force: bool) -> Result<String, String> {
    let p = path.trim();
    if p.is_empty() || p.starts_with('-') {
        return Err("Path worktree tidak valid".into());
    }
    if force {
        cli::run(repo, &["worktree", "remove", "--force", p])
    } else {
        cli::run(repo, &["worktree", "remove", p])
    }
}

pub fn prune_worktrees(repo: &str) -> Result<String, String> {
    cli::run(repo, &["worktree", "prune"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reflog_and_worktree_and_submodule() {
        let reflog_raw = "0123456789abcdef\x1f0123456\x1fHEAD@{0}\x1fcheckout: moving from main to feat\x1f5 minutes ago\x1fAlice\n";
        let entries = parse_reflog_output(reflog_raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].selector, "HEAD@{0}");
        assert_eq!(entries[0].action, "checkout: moving from main to feat");

        let wt_raw = "worktree C:/repo\nHEAD 1111111\nbranch refs/heads/main\n\nworktree C:/repo-wt\nHEAD 2222222\nbranch refs/heads/feat\nlocked\n";
        let wts = parse_worktree_porcelain(wt_raw);
        assert_eq!(wts.len(), 2);
        assert!(wts[0].is_main);
        assert_eq!(wts[0].branch.as_deref(), Some("main"));
        assert!(!wts[1].is_main);
        assert!(wts[1].is_locked);
        assert_eq!(wts[1].branch.as_deref(), Some("feat"));

        let sub = parse_submodule_status_line("+abcdef1234567890 libs/core (v1.2.0)").unwrap();
        assert_eq!(sub.status, "modified");
        assert_eq!(sub.path, "libs/core");
        assert_eq!(sub.describe, "v1.2.0");
    }
}
