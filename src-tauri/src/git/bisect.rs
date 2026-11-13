use std::path::Path;
use serde::{Deserialize, Serialize};
use super::cli;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BisectCommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub summary: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BisectState {
    pub is_active: bool,
    pub current_commit: Option<BisectCommitInfo>,
    pub remaining_steps_hint: Option<String>,
    pub first_bad_commit: Option<BisectCommitInfo>,
    pub message: String,
}

fn get_commit_info(repo: &str, rev: &str) -> Option<BisectCommitInfo> {
    let out = cli::run(repo, &["log", "-1", "--format=%H%x00%an%x00%s%x00%cr", rev]).ok()?;
    let parts: Vec<&str> = out.trim().split('\0').collect();
    if parts.len() >= 4 {
        let sha = parts[0].to_string();
        let short_sha = if sha.len() >= 7 { sha[..7].to_string() } else { sha.clone() };
        Some(BisectCommitInfo {
            sha,
            short_sha,
            author: parts[1].to_string(),
            summary: parts[2].to_string(),
            date: parts[3].to_string(),
        })
    } else {
        None
    }
}

pub fn is_bisect_active(repo: &str) -> bool {
    let p = Path::new(repo);
    p.join(".git").join("BISECT_START").exists()
        || p.join(".git").join("BISECT_LOG").exists()
}

fn parse_bisect_output(repo: &str, out: &str) -> BisectState {
    let is_active = is_bisect_active(repo);

    // Check if first bad commit was found
    let mut first_bad_commit = None;
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.contains("is the first bad commit") {
            if let Some(sha_part) = trimmed.split_whitespace().next() {
                if let Some(info) = get_commit_info(repo, sha_part) {
                    first_bad_commit = Some(info);
                    break;
                }
            }
        }
    }

    // Extract remaining steps hint
    let mut remaining_steps_hint = None;
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Bisecting:") || trimmed.contains("steps") || trimmed.contains("revisions left") {
            remaining_steps_hint = Some(trimmed.to_string());
            break;
        }
    }

    let current_commit = if is_active && first_bad_commit.is_none() {
        get_commit_info(repo, "HEAD")
    } else {
        None
    };

    BisectState {
        is_active,
        current_commit,
        remaining_steps_hint,
        first_bad_commit,
        message: out.trim().to_string(),
    }
}

pub fn bisect_status(repo: &str) -> Result<BisectState, String> {
    if !is_bisect_active(repo) {
        return Ok(BisectState {
            is_active: false,
            current_commit: None,
            remaining_steps_hint: None,
            first_bad_commit: None,
            message: "Bisect is not running".to_string(),
        });
    }

    let log_out = cli::run(repo, &["bisect", "log"]).unwrap_or_default();
    let current_commit = get_commit_info(repo, "HEAD");

    Ok(BisectState {
        is_active: true,
        current_commit,
        remaining_steps_hint: None,
        first_bad_commit: None,
        message: log_out.trim().to_string(),
    })
}

pub fn bisect_start(repo: &str, bad_sha: &str, good_sha: &str) -> Result<BisectState, String> {
    let bad = bad_sha.trim();
    let good = good_sha.trim();
    if bad.is_empty() || good.is_empty() {
        return Err("Both bad commit and good commit must be specified".into());
    }

    // Ensure clean state before starting
    if is_bisect_active(repo) {
        let _ = cli::run(repo, &["bisect", "reset"]);
    }

    let out = cli::run(repo, &["bisect", "start", bad, good])?;
    Ok(parse_bisect_output(repo, &out))
}

pub fn bisect_step(repo: &str, action: &str) -> Result<BisectState, String> {
    if !is_bisect_active(repo) {
        return Err("Bisect is not currently running".into());
    }

    let act = action.trim().to_lowercase();
    if act != "good" && act != "bad" && act != "skip" {
        return Err("Invalid bisect step action. Allowed: good, bad, skip".into());
    }

    let out = cli::run(repo, &["bisect", &act])?;
    Ok(parse_bisect_output(repo, &out))
}

pub fn bisect_reset(repo: &str) -> Result<(), String> {
    cli::run(repo, &["bisect", "reset"]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_bisect_active_non_existent() {
        assert!(!is_bisect_active("/non/existent/path"));
    }

    #[test]
    fn test_reject_empty_start_commits() {
        assert!(bisect_start("/tmp", "", "abc").is_err());
        assert!(bisect_start("/tmp", "abc", "").is_err());
    }

    #[test]
    fn test_reject_invalid_step_action() {
        assert!(bisect_step("/non/existent/repo", "invalid").is_err());
    }
}
