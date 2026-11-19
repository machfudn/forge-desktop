use std::fs;
use std::path::Path;
use super::cli;

pub fn discard_changes(repo: &str, file: &str) -> Result<(), String> {
    let trimmed = file.trim();
    if trimmed.is_empty() {
        return Err("File path tidak boleh kosong".into());
    }

    let full_path = Path::new(repo).join(trimmed);
    let status_out = cli::run(
        repo,
        &[
            "-c",
            "core.quotepath=false",
            "status",
            "--porcelain=v1",
            "--",
            trimmed,
        ],
    )?;
    let code = if status_out.len() >= 2 { &status_out[..2] } else { "" };

    // If untracked file, remove it directly
    if code == "??" {
        if full_path.is_file() {
            fs::remove_file(&full_path).map_err(|e| format!("Gagal menghapus file: {e}"))?;
            return Ok(());
        } else if full_path.is_dir() {
            fs::remove_dir_all(&full_path).map_err(|e| format!("Gagal menghapus folder: {e}"))?;
            return Ok(());
        }
    }

    let has_head = cli::run(repo, &["rev-parse", "--verify", "HEAD"]).is_ok();
    if has_head {
        // Tracked file: restore worktree and staged index
        cli::run(repo, &["checkout", "HEAD", "--", trimmed]).map(|_| ())
    } else {
        // Unborn repository (no commits yet): unstage and remove
        let _ = cli::run(repo, &["rm", "--cached", "-f", "--", trimmed]);
        if full_path.is_file() {
            let _ = fs::remove_file(&full_path);
        } else if full_path.is_dir() {
            let _ = fs::remove_dir_all(&full_path);
        }
        Ok(())
    }
}

pub fn discard_all_changes(repo: &str) -> Result<(), String> {
    if cli::run(repo, &["rev-parse", "--verify", "HEAD"]).is_ok() {
        // Unstage any staged files
        let _ = cli::run(repo, &["reset", "HEAD"]);
        // Restore tracked modifications
        cli::run(repo, &["checkout", "HEAD", "--", "."])?;
    } else {
        let _ = cli::run(repo, &["rm", "--cached", "-r", "-q", "--", "."]);
    }
    // Remove untracked files and directories
    cli::run(repo, &["clean", "-fd"])?;
    Ok(())
}

pub fn stash_save(repo: &str, message: Option<&str>) -> Result<String, String> {
    let mut args = vec!["stash", "push", "-u"];
    if let Some(msg) = message {
        let trimmed = msg.trim();
        if !trimmed.is_empty() {
            args.push("-m");
            args.push(trimmed);
        }
    }
    cli::run(repo, &args)
}

pub fn stash_pop(repo: &str) -> Result<String, String> {
    cli::run(repo, &["stash", "pop"])
}

pub fn list_stashes(repo: &str) -> Result<Vec<String>, String> {
    let out = cli::run(repo, &["stash", "list"])?;
    Ok(out.lines().map(|s| s.to_string()).collect())
}

fn validate_stash_ref(stash_ref: Option<&str>) -> Result<&str, String> {
    let target = stash_ref.unwrap_or("stash@{0}").trim();
    if target.is_empty()
        || target.starts_with('-')
        || target.contains(';')
        || target.contains('&')
        || target.contains('|')
        || target.contains('`')
        || target.contains('$')
    {
        return Err("Karakter stash ref tidak valid".into());
    }
    Ok(target)
}

pub fn stash_show(repo: &str, stash_ref: Option<&str>) -> Result<String, String> {
    let target = validate_stash_ref(stash_ref)?;
    cli::run(repo, &["stash", "show", "-p", target])
}

pub fn stash_drop(repo: &str, stash_ref: Option<&str>) -> Result<String, String> {
    let target = validate_stash_ref(stash_ref)?;
    cli::run(repo, &["stash", "drop", target])
}

pub fn stash_apply_file(repo: &str, stash_ref: Option<&str>, file_path: &str) -> Result<String, String> {
    let target = validate_stash_ref(stash_ref)?;
    let trimmed = file_path.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return Err("Path file tidak valid".into());
    }
    cli::run(repo, &["checkout", target, "--", trimmed])
}

pub fn stash_apply(repo: &str, stash_ref: Option<&str>) -> Result<String, String> {
    let target = validate_stash_ref(stash_ref)?;
    cli::run(repo, &["stash", "apply", target])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reject_empty_file_discard() {
        assert!(discard_changes("/tmp", "  ").is_err());
    }

    #[test]
    fn test_reject_invalid_stash_ref() {
        assert!(stash_show("/tmp", Some("stash@{0}; rm -rf /")).is_err());
        assert!(stash_drop("/tmp", Some("stash@{0} && echo bad")).is_err());
    }
}
