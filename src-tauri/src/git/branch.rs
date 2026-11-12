use super::cli;
use super::remote::{parse_ahead_behind_output, AheadBehind};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub is_pushed: bool,
    pub upstream_gone: bool,
}

fn parse_track_counts(track: &str) -> (usize, usize, bool) {
    let mut ahead = 0;
    let mut behind = 0;
    let mut gone = false;
    for part in track.split(',') {
        let p = part.trim();
        if p == "gone" {
            gone = true;
        } else if let Some(rest) = p.strip_prefix("ahead ") {
            ahead = rest.trim().parse::<usize>().unwrap_or(0);
        } else if let Some(rest) = p.strip_prefix("behind ") {
            behind = rest.trim().parse::<usize>().unwrap_or(0);
        }
    }
    (ahead, behind, gone)
}

pub fn list_branches(repo: &str) -> Result<Vec<BranchInfo>, String> {
    let out = cli::run(
        repo,
        &[
            "for-each-ref",
            "--format=%(HEAD)\x1f%(refname:short)\x1f%(upstream:short)\x1f%(upstream:track,nobracket)",
            "refs/heads",
        ],
    )?;

    let remote_refs: Vec<String> = cli::run(
        repo,
        &["for-each-ref", "--format=%(refname:short)", "refs/remotes"],
    )
    .map(|r| {
        r.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.ends_with("/HEAD"))
            .map(ToString::to_string)
            .collect()
    })
    .unwrap_or_default();

    let mut branches = Vec::new();
    for line in out.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\x1f').collect();
        if cols.len() < 2 {
            continue;
        }
        let is_current = cols[0].trim() == "*";
        let name = cols[1].trim().to_string();
        if name.is_empty() {
            continue;
        }
        let upstream = cols
            .get(2)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        let track = cols.get(3).map(|s| s.trim()).unwrap_or("");
        let (mut ahead, mut behind, upstream_gone) = parse_track_counts(track);

        let is_pushed = if upstream.is_some() {
            !upstream_gone && ahead == 0
        } else {
            let suffix = format!("/{name}");
            if let Some(rem_ref) = remote_refs.iter().find(|r| r.ends_with(&suffix)) {
                let spec = format!("{name}...{rem_ref}");
                if let Ok(ab_out) = cli::run(repo, &["rev-list", "--left-right", "--count", &spec]) {
                    let ab = parse_ahead_behind_output(&ab_out);
                    ahead = ab.ahead;
                    behind = ab.behind;
                }
                ahead == 0
            } else {
                false
            }
        };

        branches.push(BranchInfo {
            name,
            is_current,
            upstream,
            ahead,
            behind,
            is_pushed,
            upstream_gone,
        });
    }

    Ok(branches)
}

pub fn switch_branch(repo: &str, branch_name: &str) -> Result<(), String> {
    let branch = branch_name.trim();
    if branch.is_empty() || branch.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    cli::run(repo, &["checkout", branch]).map(|_| ())
}

pub fn create_branch(repo: &str, branch_name: &str) -> Result<(), String> {
    let branch = branch_name.trim();
    if branch.is_empty() || branch.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    cli::run(repo, &["checkout", "-b", branch]).map(|_| ())
}

pub fn delete_branch(repo: &str, branch_name: &str, force: bool) -> Result<String, String> {
    let branch = branch_name.trim();
    if branch.is_empty() || branch.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    let flag = if force { "-D" } else { "-d" };
    cli::run(repo, &["branch", flag, branch])
}

pub fn rename_branch(repo: &str, old_name: &str, new_name: &str) -> Result<String, String> {
    let old_b = old_name.trim();
    let new_b = new_name.trim();
    if old_b.is_empty() || old_b.starts_with('-') || new_b.is_empty() || new_b.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    cli::run(repo, &["branch", "-m", old_b, new_b])
}

pub fn merge_branch(repo: &str, target_branch: &str) -> Result<String, String> {
    let target = target_branch.trim();
    if target.is_empty() || target.starts_with('-') {
        return Err("Nama branch target tidak valid".into());
    }
    cli::run(repo, &["merge", target])
}

pub fn compare_branches(repo: &str, base: &str, target: &str) -> Result<AheadBehind, String> {
    let b = base.trim();
    let t = target.trim();
    if b.is_empty() || b.starts_with('-') || t.is_empty() || t.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    let spec = format!("{b}...{t}");
    let out = cli::run(repo, &["rev-list", "--left-right", "--count", &spec])?;
    Ok(parse_ahead_behind_output(&out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_invalid_branch_names() {
        assert!(switch_branch("/tmp", "  ").is_err());
        assert!(switch_branch("/tmp", "-invalid").is_err());
        assert!(create_branch("/tmp", " ").is_err());
        assert!(create_branch("/tmp", "--force").is_err());
        assert!(delete_branch("/tmp", " ", false).is_err());
        assert!(delete_branch("/tmp", "-D", true).is_err());
        assert!(rename_branch("/tmp", " ", "new", ).is_err());
        assert!(rename_branch("/tmp", "old", "-invalid").is_err());
        assert!(merge_branch("/tmp", " ").is_err());
        assert!(merge_branch("/tmp", "-m").is_err());
        assert!(compare_branches("/tmp", " -b", "main").is_err());
        assert!(compare_branches("/tmp", "main", "").is_err());
    }

    #[test]
    fn list_branches_reports_push_status_per_branch() {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("forge-branch-push-test-{nanos}"));
        let bare = root.join("bare.git");
        let local = root.join("local");
        std::fs::create_dir_all(&bare).unwrap();
        std::fs::create_dir_all(&local).unwrap();

        let run_git = |cwd: &std::path::Path, args: &[&str]| {
            let out = Command::new("git").args(args).current_dir(cwd).output().unwrap();
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };

        run_git(&bare, &["init", "--bare"]);
        run_git(&local, &["init", "-b", "main"]);
        run_git(&local, &["config", "user.name", "Tester"]);
        run_git(&local, &["config", "user.email", "tester@example.com"]);
        run_git(&local, &["remote", "add", "origin", bare.to_str().unwrap()]);

        std::fs::write(local.join("a.txt"), "one\n").unwrap();
        run_git(&local, &["add", "a.txt"]);
        run_git(&local, &["commit", "-m", "init"]);
        run_git(&local, &["push", "-u", "origin", "main"]);
        run_git(&local, &["checkout", "-b", "feat/unpushed"]);
        std::fs::write(local.join("b.txt"), "two\n").unwrap();
        run_git(&local, &["add", "b.txt"]);
        run_git(&local, &["commit", "-m", "feat"]);

        let branches = list_branches(local.to_str().unwrap()).unwrap();
        let main_b = branches.iter().find(|b| b.name == "main").unwrap();
        assert!(main_b.is_pushed);
        assert_eq!(main_b.upstream.as_deref(), Some("origin/main"));
        assert!(!main_b.is_current);

        let feat_b = branches.iter().find(|b| b.name == "feat/unpushed").unwrap();
        assert!(!feat_b.is_pushed);
        assert!(feat_b.upstream.is_none());
        assert!(feat_b.is_current);

        let _ = std::fs::remove_dir_all(&root);
    }
}
