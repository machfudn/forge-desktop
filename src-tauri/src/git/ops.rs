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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebaseStep {
    pub sha: String,
    pub action: String,
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
    }

    let orig_head = cli::run(repo, &["rev-parse", "HEAD"])?.trim().to_string();

    if let Err(e) = cli::run(repo, &["reset", "--hard", base_sha]) {
        return Err(format!("Gagal reset ke base sha: {e}"));
    }

    for step in &steps {
        if step.action == "drop" {
            continue;
        }
        if step.action == "pick" {
            if let Err(e) = cli::run(repo, &["cherry-pick", &step.sha]) {
                let _ = cli::run(repo, &["cherry-pick", "--abort"]);
                let _ = cli::run(repo, &["reset", "--hard", &orig_head]);
                return Err(format!("Gagal saat rebase pada commit {}: {e}", step.sha));
            }
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
    }
}
