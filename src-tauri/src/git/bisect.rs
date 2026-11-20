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
    let git_dir = super::status::resolve_git_dir(repo);
    git_dir.join("BISECT_START").exists() || git_dir.join("BISECT_LOG").exists()
}

pub fn extract_culprit_sha(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();

        // Pattern 1: Output from bisect command:
        // "<sha> is the first 'bad' commit" (modern Git) or "<sha> is the first bad commit" (older Git)
        if (trimmed.contains("is the first 'bad' commit")
            || trimmed.contains("is the first bad commit")
            || (trimmed.contains("is the first ") && trimmed.ends_with("commit")))
            && !trimmed.starts_with('#')
        {
            if let Some(sha_part) = trimmed.split_whitespace().next() {
                let clean_sha = sha_part.trim();
                if clean_sha.len() >= 4 && clean_sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Some(clean_sha.to_string());
                }
            }
        }

        // Pattern 2: Log format from BISECT_LOG or `git bisect log`:
        // "# first 'bad' commit: [sha] ..." or "# first bad commit: [sha] ..."
        if trimmed.starts_with("# first ") && trimmed.contains("commit: [") {
            if let Some(start_idx) = trimmed.find('[') {
                if let Some(end_idx) = trimmed[start_idx + 1..].find(']') {
                    let sha = &trimmed[start_idx + 1..start_idx + 1 + end_idx];
                    if sha.len() >= 4 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Some(sha.to_string());
                    }
                }
            }
        }
    }
    None
}

fn parse_bisect_output(repo: &str, out: &str) -> BisectState {
    let is_active = is_bisect_active(repo);

    // Check if first bad commit was found in command output or log
    let mut first_bad_commit = None;
    if let Some(sha) = extract_culprit_sha(out) {
        first_bad_commit = get_commit_info(repo, &sha);
    } else if is_active {
        let log_out = cli::run(repo, &["bisect", "log"]).unwrap_or_default();
        if let Some(sha) = extract_culprit_sha(&log_out) {
            first_bad_commit = get_commit_info(repo, &sha);
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
    let culprit_sha = extract_culprit_sha(&log_out);

    let first_bad_commit = if let Some(ref sha) = culprit_sha {
        get_commit_info(repo, sha)
    } else {
        None
    };

    let current_commit = if first_bad_commit.is_none() {
        get_commit_info(repo, "HEAD")
    } else {
        None
    };

    Ok(BisectState {
        is_active: true,
        current_commit,
        remaining_steps_hint: None,
        first_bad_commit,
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

    #[test]
    fn test_extract_culprit_sha_modern_git() {
        let out = "94b4f2423910916a1d8bfa914d9f84e867d2d8f1 is the first 'bad' commit\ncommit 94b4f24\nAuthor: Foo";
        assert_eq!(
            extract_culprit_sha(out),
            Some("94b4f2423910916a1d8bfa914d9f84e867d2d8f1".to_string())
        );
    }

    #[test]
    fn test_extract_culprit_sha_legacy_git() {
        let out = "94b4f2423910916a1d8bfa914d9f84e867d2d8f1 is the first bad commit\ncommit 94b4f24";
        assert_eq!(
            extract_culprit_sha(out),
            Some("94b4f2423910916a1d8bfa914d9f84e867d2d8f1".to_string())
        );
    }

    #[test]
    fn test_extract_culprit_sha_from_bisect_log() {
        let log = "# bad: [11faf06] c3\n# good: [942a679] c1\n# first 'bad' commit: [94b4f2423910916a1d8bfa914d9f84e867d2d8f1] c2 mid";
        assert_eq!(
            extract_culprit_sha(log),
            Some("94b4f2423910916a1d8bfa914d9f84e867d2d8f1".to_string())
        );
    }

    #[test]
    fn test_extract_culprit_sha_intermediate_step() {
        let out = "Bisecting: 2 revisions left to test after this (roughly 1 step)\n[abc1234] some commit";
        assert_eq!(extract_culprit_sha(out), None);
    }

    #[test]
    fn test_local_repo_bisect_lifecycle() {
        let tmp = std::env::temp_dir().join(format!("forge-test-bisect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.to_str().unwrap();

        // init repo
        cli::run_raw(path, &["init"]).unwrap();
        cli::run_raw(path, &["config", "user.name", "Tester"]).unwrap();
        cli::run_raw(path, &["config", "user.email", "test@test.com"]).unwrap();

        // c1 (good)
        std::fs::write(tmp.join("file.txt"), "c1").unwrap();
        cli::run_raw(path, &["add", "file.txt"]).unwrap();
        cli::run_raw(path, &["commit", "-m", "c1 good"]).unwrap();
        let c1 = cli::run_raw(path, &["rev-parse", "HEAD"]).unwrap();

        // c2 (bad culprit)
        std::fs::write(tmp.join("file.txt"), "c2 bad").unwrap();
        cli::run_raw(path, &["commit", "-am", "c2 bad"]).unwrap();
        let c2 = cli::run_raw(path, &["rev-parse", "HEAD"]).unwrap();

        // c3 (still bad)
        std::fs::write(tmp.join("file.txt"), "c3 worse").unwrap();
        cli::run_raw(path, &["commit", "-am", "c3 worse"]).unwrap();
        let c3 = cli::run_raw(path, &["rev-parse", "HEAD"]).unwrap();

        // Start bisect: bad = c3, good = c1
        let st1 = bisect_start(path, &c3, &c1).unwrap();
        assert!(st1.is_active);
        assert!(st1.current_commit.is_some());
        assert_eq!(st1.current_commit.unwrap().sha, c2);

        // Step: mark c2 as bad -> this should find c2 as the first bad commit!
        let st2 = bisect_step(path, "bad").unwrap();
        assert!(st2.is_active);
        assert!(st2.first_bad_commit.is_some(), "first_bad_commit must be detected!");
        assert_eq!(st2.first_bad_commit.unwrap().sha, c2);

        // Status check: should preserve the first_bad_commit
        let st_status = bisect_status(path).unwrap();
        assert!(st_status.is_active);
        assert!(st_status.first_bad_commit.is_some(), "bisect_status must preserve culprit!");
        assert_eq!(st_status.first_bad_commit.unwrap().sha, c2);

        // Reset bisect
        bisect_reset(path).unwrap();
        assert!(!is_bisect_active(path));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
