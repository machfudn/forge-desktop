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
    pub is_remote: bool,
    pub remote_name: Option<String>,
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
            "--format=%(refname)\x1f%(HEAD)\x1f%(refname:short)\x1f%(objectname)\x1f%(upstream:short)\x1f%(upstream:track,nobracket)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;

    struct LocalBranchRaw {
        name: String,
        is_current: bool,
        oid: String,
        upstream: Option<String>,
        ahead: usize,
        behind: usize,
        upstream_gone: bool,
    }

    let mut local_raw = Vec::new();
    let mut remote_refs: Vec<(String, String)> = Vec::new();

    for line in out.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\x1f').collect();
        if cols.len() < 4 {
            continue;
        }
        let refname = cols[0].trim();
        let short = cols[2].trim();
        let oid = cols[3].trim();

        if refname.starts_with("refs/remotes/") {
            if refname.ends_with("/HEAD") || short.ends_with("/HEAD") || !short.contains('/') {
                continue;
            }
            if !short.is_empty() {
                remote_refs.push((short.to_string(), oid.to_string()));
            }
            continue;
        }

        if !refname.starts_with("refs/heads/") || short.is_empty() {
            continue;
        }

        let is_current = cols[1].trim() == "*";
        let upstream = cols
            .get(4)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        let track = cols.get(5).map(|s| s.trim()).unwrap_or("");
        let (ahead, behind, upstream_gone) = parse_track_counts(track);

        local_raw.push(LocalBranchRaw {
            name: short.to_string(),
            is_current,
            oid: oid.to_string(),
            upstream,
            ahead,
            behind,
            upstream_gone,
        });
    }

    let mut branches = Vec::with_capacity(local_raw.len());
    let mut pending_rev_list: Vec<(usize, String, String)> = Vec::new();

    for item in local_raw {
        if item.upstream.is_some() {
            let is_pushed = !item.upstream_gone && item.ahead == 0;
            branches.push(BranchInfo {
                name: item.name,
                is_current: item.is_current,
                upstream: item.upstream,
                ahead: item.ahead,
                behind: item.behind,
                is_pushed,
                upstream_gone: item.upstream_gone,
                is_remote: false,
                remote_name: None,
            });
            continue;
        }

        let suffix = format!("/{}", item.name);
        if let Some((rem_ref, rem_oid)) = remote_refs.iter().find(|(r, _)| r.ends_with(&suffix)) {
            let idx = branches.len();
            if !item.oid.is_empty() && item.oid == *rem_oid {
                branches.push(BranchInfo {
                    name: item.name,
                    is_current: item.is_current,
                    upstream: None,
                    ahead: 0,
                    behind: 0,
                    is_pushed: true,
                    upstream_gone: false,
                    is_remote: false,
                    remote_name: None,
                });
            } else {
                let spec = format!("{}...{}", item.name, rem_ref);
                branches.push(BranchInfo {
                    name: item.name,
                    is_current: item.is_current,
                    upstream: None,
                    ahead: 0,
                    behind: 0,
                    is_pushed: false,
                    upstream_gone: false,
                    is_remote: false,
                    remote_name: None,
                });
                pending_rev_list.push((idx, spec, rem_ref.clone()));
            }
        } else {
            branches.push(BranchInfo {
                name: item.name,
                is_current: item.is_current,
                upstream: None,
                ahead: item.ahead,
                behind: item.behind,
                is_pushed: false,
                upstream_gone: item.upstream_gone,
                is_remote: false,
                remote_name: None,
            });
        }
    }

    if !pending_rev_list.is_empty() {
        let results: Vec<(usize, Option<AheadBehind>)> = std::thread::scope(|s| {
            let handles: Vec<_> = pending_rev_list
                .iter()
                .map(|(idx, spec, _)| {
                    let idx = *idx;
                    let spec = spec.as_str();
                    s.spawn(move || {
                        let ab = cli::run(repo, &["rev-list", "--left-right", "--count", spec])
                            .ok()
                            .map(|out| parse_ahead_behind_output(&out));
                        (idx, ab)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap_or((0, None))).collect()
        });
        for (idx, ab_opt) in results {
            if let Some(ab) = ab_opt {
                if let Some(b) = branches.get_mut(idx) {
                    b.ahead = ab.ahead;
                    b.behind = ab.behind;
                    b.is_pushed = ab.ahead == 0;
                }
            } else if let Some(b) = branches.get_mut(idx) {
                b.is_pushed = true;
            }
        }
    }

    for (rem_short, _) in remote_refs {
        let remote_name = rem_short.split('/').next().map(ToString::to_string);
        branches.push(BranchInfo {
            name: rem_short.clone(),
            is_current: false,
            upstream: Some(rem_short),
            ahead: 0,
            behind: 0,
            is_pushed: true,
            upstream_gone: false,
            is_remote: true,
            remote_name,
        });
    }

    Ok(branches)
}

pub fn switch_branch(repo: &str, branch_name: &str) -> Result<(), String> {
    let branch = branch_name.trim();
    if branch.is_empty() || branch.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    let local_ref = format!("refs/heads/{branch}");
    if cli::run(repo, &["show-ref", "--verify", "--quiet", &local_ref]).is_ok() {
        return cli::run(repo, &["checkout", branch]).map(|_| ());
    }
    let remote_ref = format!("refs/remotes/{branch}");
    if cli::run(repo, &["show-ref", "--verify", "--quiet", &remote_ref]).is_ok() {
        if let Some((_, local_name)) = branch.split_once('/') {
            if !local_name.is_empty() && !local_name.starts_with('-') {
                let target_local_ref = format!("refs/heads/{local_name}");
                if cli::run(repo, &["show-ref", "--verify", "--quiet", &target_local_ref]).is_ok() {
                    return cli::run(repo, &["checkout", local_name]).map(|_| ());
                }
                return cli::run(repo, &["checkout", "-b", local_name, "--track", branch])
                    .map(|_| ());
            }
        }
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
        assert!(branches.iter().all(|b| b.name != "origin" && !b.name.ends_with("/HEAD")));
        let feat_b = branches.iter().find(|b| b.name == "feat/unpushed").unwrap();
        assert!(!feat_b.is_pushed);
        assert!(feat_b.upstream.is_none());
        assert!(feat_b.is_current);

        run_git(&local, &["checkout", "-b", "remote-only"]);
        std::fs::write(local.join("c.txt"), "three\n").unwrap();
        run_git(&local, &["add", "c.txt"]);
        run_git(&local, &["commit", "-m", "remote branch commit"]);
        run_git(&local, &["push", "origin", "remote-only"]);
        run_git(&local, &["checkout", "main"]);
        run_git(&local, &["branch", "-D", "remote-only"]);

        let all_branches = list_branches(local.to_str().unwrap()).unwrap();
        let remote_only = all_branches
            .iter()
            .find(|b| b.is_remote && b.name == "origin/remote-only")
            .expect("remote-only branch must be returned by list_branches");
        assert_eq!(remote_only.remote_name.as_deref(), Some("origin"));

        switch_branch(local.to_str().unwrap(), "origin/remote-only").unwrap();
        let after_switch = list_branches(local.to_str().unwrap()).unwrap();
        let checked_out = after_switch
            .iter()
            .find(|b| !b.is_remote && b.name == "remote-only")
            .expect("checking out remote branch should create local tracking branch");
        assert!(checked_out.is_current);
        assert_eq!(checked_out.upstream.as_deref(), Some("origin/remote-only"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
