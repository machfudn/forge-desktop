use std::fs;
use std::path::Path;
use super::cli;

pub fn discard_changes(repo: &str, file: &str) -> Result<(), String> {
    let trimmed = file.trim();
    if trimmed.is_empty() {
        return Err("File path tidak boleh kosong".into());
    }

    let full_path = Path::new(repo).join(trimmed);
    // If untracked file, remove it directly
    let status_out = cli::run(repo, &["status", "--porcelain=v1", "--", trimmed])?;
    if status_out.starts_with("??") {
        if full_path.is_file() {
            fs::remove_file(&full_path).map_err(|e| format!("Gagal menghapus file: {e}"))?;
            return Ok(());
        } else if full_path.is_dir() {
            fs::remove_dir_all(&full_path).map_err(|e| format!("Gagal menghapus folder: {e}"))?;
            return Ok(());
        }
    }

    // Tracked file: restore worktree and staged index
    cli::run(repo, &["checkout", "HEAD", "--", trimmed]).map(|_| ())
}

pub fn discard_all_changes(repo: &str) -> Result<(), String> {
    // Unstage any staged files
    let _ = cli::run(repo, &["reset", "HEAD"]);
    // Restore tracked modifications
    cli::run(repo, &["checkout", "HEAD", "--", "."])?;
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

pub fn stash_show(repo: &str, stash_ref: Option<&str>) -> Result<String, String> {
    let target = stash_ref.unwrap_or("stash@{0}").trim();
    if target.contains(';')
        || target.contains('&')
        || target.contains('|')
        || target.contains('`')
        || target.contains('$')
    {
        return Err("Karakter stash ref tidak valid".into());
    }
    cli::run(repo, &["stash", "show", "-p", target])
}

pub fn stash_drop(repo: &str, stash_ref: Option<&str>) -> Result<String, String> {
    let target = stash_ref.unwrap_or("stash@{0}").trim();
    if target.contains(';')
        || target.contains('&')
        || target.contains('|')
        || target.contains('`')
        || target.contains('$')
    {
        return Err("Karakter stash ref tidak valid".into());
    }
    cli::run(repo, &["stash", "drop", target])
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
