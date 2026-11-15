use super::cli;
use super::repo::check_index_lock;
use serde::{Deserialize, Serialize};

fn validate_sha(sha: &str) -> Result<&str, String> {
    let s = sha.trim();
    if s.is_empty()
        || s.len() < 4
        || s.len() > 64
        || !s.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err("SHA commit tidak valid".into());
    }
    Ok(s)
}

fn validate_tag_name(name: &str) -> Result<&str, String> {
    let n = name.trim();
    if n.is_empty()
        || n.starts_with('-')
        || n.contains(' ')
        || n.contains('~')
        || n.contains('^')
        || n.contains(':')
    {
        return Err("Nama tag tidak valid".into());
    }
    Ok(n)
}

pub fn cherry_pick(repo: &str, sha: &str) -> Result<String, String> {
    let valid_sha = validate_sha(sha)?;
    cli::run(repo, &["cherry-pick", valid_sha])
}

pub fn revert_commit(repo: &str, sha: &str) -> Result<String, String> {
    let valid_sha = validate_sha(sha)?;
    cli::run(repo, &["revert", "--no-edit", valid_sha])
}

pub fn reset_to_commit(repo: &str, sha: &str, hard: bool) -> Result<String, String> {
    let valid_sha = validate_sha(sha)?;
    let mode = if hard { "--hard" } else { "--soft" };
    cli::run(repo, &["reset", mode, valid_sha])
}

pub fn rebase_branch(repo: &str, base_branch: &str) -> Result<String, String> {
    let base = base_branch.trim();
    if base.is_empty() || base.starts_with('-') {
        return Err("Nama branch dasar tidak valid".into());
    }
    cli::run(repo, &["rebase", base])
}

pub fn list_tags(repo: &str) -> Result<Vec<String>, String> {
    let out = cli::run(repo, &["tag", "-l"])?;
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

pub fn create_tag(repo: &str, tag_name: &str, target_sha: Option<&str>) -> Result<String, String> {
    let name = validate_tag_name(tag_name)?;
    let mut args = vec!["tag", name];
    let sha_val;
    if let Some(s) = target_sha {
        if !s.trim().is_empty() {
            sha_val = validate_sha(s)?;
            args.push(sha_val);
        }
    }
    cli::run(repo, &args)
}

pub fn delete_tag(repo: &str, tag_name: &str) -> Result<String, String> {
    let name = validate_tag_name(tag_name)?;
    cli::run(repo, &["tag", "-d", name])
}

pub fn squash_commits(
    repo: &str,
    count: usize,
    new_summary: &str,
    new_description: Option<&str>,
) -> Result<String, String> {
    if count < 2 {
        return Err("Jumlah komit untuk di-squash minimal 2".into());
    }
    let s = new_summary.trim();
    if s.is_empty() {
        return Err("Pesan komit baru tidak boleh kosong".into());
    }

    let reset_target = format!("HEAD~{count}");
    cli::run(repo, &["reset", "--soft", &reset_target])?;

    let mut args = vec!["commit", "-m", s];
    let desc_val;
    if let Some(d) = new_description {
        desc_val = d.trim();
        if !desc_val.is_empty() {
            args.push("-m");
            args.push(desc_val);
        }
    }
    cli::run(repo, &args)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RebaseStep {
    pub sha: String,
    pub action: String,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RebaseCommitItem {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub summary: String,
}

pub fn get_rebase_commits(repo: &str, base_sha: &str) -> Result<Vec<RebaseCommitItem>, String> {
    validate_sha(base_sha)?;
    let rev_range = format!("{base_sha}..HEAD");
    let out = cli::run(repo, &["log", "--reverse", "--format=%H%x00%an%x00%s", &rev_range])?;
    let mut commits = Vec::new();
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('\0').collect();
        if parts.len() >= 3 {
            let sha = parts[0].to_string();
            let short_sha = if sha.len() >= 7 { sha[..7].to_string() } else { sha.clone() };
            commits.push(RebaseCommitItem {
                sha,
                short_sha,
                author: parts[1].to_string(),
                summary: parts[2].to_string(),
            });
        }
    }
    Ok(commits)
}

pub fn get_all_rebase_commits(repo: &str) -> Result<(String, Vec<RebaseCommitItem>), String> {
    let root_sha = cli::run(repo, &["rev-list", "--max-parents=0", "--reverse", "HEAD"])?.trim().to_string();
    if root_sha.is_empty() {
        return Ok((String::new(), Vec::new()));
    }
    get_rebase_commits(repo, &root_sha).map(|commits| (root_sha, commits))
}

pub fn interactive_rebase(
    repo: &str,
    base_sha: &str,
    steps: Vec<RebaseStep>,
) -> Result<String, String> {
    validate_sha(base_sha)?;
    if steps.is_empty() {
        return Err("Daftar langkah rebase tidak boleh kosong".into());
    }
    for s in &steps {
        validate_sha(&s.sha)?;
        match s.action.as_str() {
            "pick" | "reword" | "squash" | "fixup" | "drop" | "edit" => {}
            _ => return Err(format!("Aksi rebase tidak valid: {}", s.action)),
        }
    }

    let status_out = cli::run(repo, &["status", "--porcelain=v1"])?;
    if !status_out.trim().is_empty() {
        return Err("Working tree masih memiliki perubahan belum di-commit. Commit atau stash dahulu sebelum rebase.".into());
    }

    let orig_head = cli::run(repo, &["rev-parse", "HEAD"])?.trim().to_string();
    if let Err(e) = cli::run(repo, &["reset", "--hard", base_sha]) {
        return Err(format!("Gagal reset ke base sha: {e}"));
    }

    for step in &steps {
        match step.action.as_str() {
            "drop" => continue,
            "pick" | "edit" => {
                if let Err(e) = cli::run(repo, &["cherry-pick", &step.sha]) {
                    let _ = cli::run(repo, &["cherry-pick", "--abort"]);
                    let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
                    return Err(format!("Gagal saat rebase pada commit {}: {e}", step.sha));
                }
            }
            "reword" => {
                if let Err(e) = cli::run(repo, &["cherry-pick", &step.sha]) {
                    let _ = cli::run(repo, &["cherry-pick", "--abort"]);
                    let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
                    return Err(format!("Gagal saat rebase pada commit {}: {e}", step.sha));
                }
                if let Some(msg) = &step.message {
                    if !msg.trim().is_empty() {
                        if let Err(e) = cli::run(repo, &["commit", "--amend", "-m", msg.trim()]) {
                            let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
                            return Err(format!("Gagal saat reword commit {}: {e}", step.sha));
                        }
                    }
                }
            }
            "squash" => {
                if let Err(e) = cli::run(repo, &["cherry-pick", "-n", &step.sha]) {
                    let _ = cli::run(repo, &["cherry-pick", "--abort"]);
                    let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
                    return Err(format!("Gagal saat squash commit {}: {e}", step.sha));
                }
                let commit_res = if let Some(msg) = &step.message {
                    if !msg.trim().is_empty() {
                        cli::run(repo, &["commit", "--amend", "-m", msg.trim()])
                    } else {
                        cli::run(repo, &["commit", "--amend", "--no-edit"])
                    }
                } else {
                    cli::run(repo, &["commit", "--amend", "--no-edit"])
                };
                if let Err(e) = commit_res {
                    let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
                    return Err(format!("Gagal saat commit squash {}: {e}", step.sha));
                }
            }
            "fixup" => {
                if let Err(e) = cli::run(repo, &["cherry-pick", "-n", &step.sha]) {
                    let _ = cli::run(repo, &["cherry-pick", "--abort"]);
                    let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
                    return Err(format!("Gagal saat fixup commit {}: {e}", step.sha));
                }
                if let Err(e) = cli::run(repo, &["commit", "--amend", "--no-edit"]) {
                    let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
                    return Err(format!("Gagal saat commit fixup {}: {e}", step.sha));
                }
            }
            _ => {}
        }
    }

    Ok("Interactive rebase berhasil diselesaikan".into())
}

pub fn reword_commit(
    repo: &str,
    target_sha: &str,
    new_message: &str,
) -> Result<String, String> {
    validate_sha(target_sha)?;
    let trimmed_msg = new_message.trim();
    if trimmed_msg.is_empty() {
        return Err("Pesan commit baru tidak boleh kosong".into());
    }

    if check_index_lock(repo)? {
        return Err("File .git/index.lock terdeteksi. Silakan bersihkan file lock terlebih dahulu.".into());
    }

    let status_out = cli::run(repo, &["status", "--porcelain=v1"])?;
    if !status_out.trim().is_empty() {
        return Err("Working tree masih memiliki perubahan belum di-commit. Commit atau stash dahulu sebelum mengedit pesan commit.".into());
    }

    let orig_head = cli::run(repo, &["rev-parse", "HEAD"])?.trim().to_string();
    let full_target = cli::run(repo, &["rev-parse", target_sha])?.trim().to_string();

    if orig_head == full_target {
        cli::run(repo, &["commit", "--amend", "-m", trimmed_msg])?;
        return Ok("Pesan commit HEAD berhasil diperbarui".into());
    }

    if cli::run(repo, &["merge-base", "--is-ancestor", &full_target, &orig_head]).is_err() {
        return Err("Commit yang dipilih bukan bagian dari branch aktif saat ini".into());
    }

    let parent_out = cli::run(repo, &["rev-parse", &format!("{full_target}^")]);
    let base_sha = match parent_out {
        Ok(out) => out.trim().to_string(),
        Err(_) => {
            return Err("Tidak dapat menemukan parent commit (commit root belum didukung untuk reword otomatis)".into());
        }
    };

    let rev_range = format!("{base_sha}..{orig_head}");
    let log_out = cli::run(repo, &["log", "--reverse", "--format=%H", &rev_range])?;
    let commits_to_replay: Vec<String> = log_out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if commits_to_replay.is_empty() {
        return Err("Tidak ada commit yang ditemukan untuk di-replay".into());
    }

    if let Err(e) = cli::run(repo, &["reset", "--hard", &base_sha]) {
        return Err(format!("Gagal reset ke base sha: {e}"));
    }

    for sha in &commits_to_replay {
        if let Err(e) = cli::run(repo, &["cherry-pick", sha]) {
            let _ = cli::run(repo, &["cherry-pick", "--abort"]);
            let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
            return Err(format!("Gagal saat cherry-pick commit {sha}: {e}"));
        }

        if *sha == full_target {
            if let Err(e) = cli::run(repo, &["commit", "--amend", "-m", trimmed_msg]) {
                let _ = cli::run(repo, &["cherry-pick", "--abort"]);
                let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
                return Err(format!("Gagal saat amend pesan commit {sha}: {e}"));
            }
        }
    }

    Ok("Pesan commit berhasil diperbarui dan seluruh commit berhasil di-replay".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_sha_and_tags() {
        assert!(validate_sha("abc").is_err());
        assert!(validate_sha("not-hex").is_err());
        assert!(validate_sha("a1b2c3d").is_ok());

        assert!(validate_tag_name("").is_err());
        assert!(validate_tag_name("-invalid").is_err());
        assert!(validate_tag_name("v1.0.0").is_ok());
    }

    #[test]
    fn test_reject_empty_rebase_steps() {
        assert!(interactive_rebase("/tmp", "abc1234", vec![]).is_err());
        assert!(interactive_rebase("/tmp", "invalid", vec![]).is_err());
        assert!(interactive_rebase(
            "/tmp",
            "abc1234",
            vec![RebaseStep { sha: "abc1234".into(), action: "invalid".into(), message: None }]
        )
        .is_err());
    }

    #[test]
    fn test_reword_commit_preserves_all_subsequent_commits() {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("forge-reword-preserve-test-{nanos}"));
        std::fs::create_dir_all(&root).unwrap();
        let run_git = |args: &[&str]| {
            let out = Command::new("git").args(args).current_dir(&root).output().unwrap();
            assert!(out.status.success(), "git {:?} failed: {}", args, String::from_utf8_lossy(&out.stderr));
        };
        run_git(&["init"]);
        run_git(&["config", "user.name", "Tester"]);
        run_git(&["config", "user.email", "tester@example.com"]);
        std::fs::write(root.join("f1.txt"), "hello").unwrap();
        run_git(&["add", "f1.txt"]);
        run_git(&["commit", "-m", "commit 1 (base)"]);

        std::fs::write(root.join("f2.txt"), "world").unwrap();
        run_git(&["add", "f2.txt"]);
        run_git(&["commit", "-m", "commit 2 (to reword)"]);
        let c2 = cli::run(root.to_str().unwrap(), &["rev-parse", "HEAD"]).unwrap().trim().to_string();

        std::fs::write(root.join("f3.txt"), "foo").unwrap();
        run_git(&["add", "f3.txt"]);
        run_git(&["commit", "-m", "commit 3 (subsequent 1)"]);

        std::fs::write(root.join("f4.txt"), "bar").unwrap();
        run_git(&["add", "f4.txt"]);
        run_git(&["commit", "-m", "commit 4 (subsequent 2 HEAD)"]);

        let res = reword_commit(root.to_str().unwrap(), &c2, "reworded commit 2 summary\n\nextended desc");
        assert!(res.is_ok(), "reword_commit failed: {:?}", res);

        let log = cli::run(root.to_str().unwrap(), &["log", "--oneline"]).unwrap();
        assert!(log.contains("reworded commit 2 summary"));
        assert!(log.contains("commit 3 (subsequent 1)"));
        assert!(log.contains("commit 4 (subsequent 2 HEAD)"));
        assert!(log.contains("commit 1 (base)"));

        let count = cli::run(root.to_str().unwrap(), &["rev-list", "--count", "HEAD"]).unwrap().trim().to_string();
        assert_eq!(count, "4", "Total commits must remain 4 after reword");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_reword_head_commit() {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("forge-reword-head-test-{nanos}"));
        std::fs::create_dir_all(&root).unwrap();
        let run_git = |args: &[&str]| {
            let out = Command::new("git").args(args).current_dir(&root).output().unwrap();
            assert!(out.status.success(), "git {:?} failed: {}", args, String::from_utf8_lossy(&out.stderr));
        };
        run_git(&["init"]);
        run_git(&["config", "user.name", "Tester"]);
        run_git(&["config", "user.email", "tester@example.com"]);
        std::fs::write(root.join("f1.txt"), "hello").unwrap();
        run_git(&["add", "f1.txt"]);
        run_git(&["commit", "-m", "commit 1"]);
        let c1 = cli::run(root.to_str().unwrap(), &["rev-parse", "HEAD"]).unwrap().trim().to_string();

        let res = reword_commit(root.to_str().unwrap(), &c1, "commit 1 updated");
        assert!(res.is_ok());
        let log = cli::run(root.to_str().unwrap(), &["log", "--oneline"]).unwrap();
        assert!(log.contains("commit 1 updated"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
