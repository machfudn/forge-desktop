use super::cli;
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
}
