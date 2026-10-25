use std::path::Path;
use serde::Serialize;
use super::cli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub status_code: String,
    pub staged: bool,
    pub is_conflict: bool,
    pub is_lfs: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoStatus {
    pub branch: String,
    pub files: Vec<FileEntry>,
    pub has_conflicts: bool,
    pub in_merge: bool,
    pub in_rebase: bool,
}

pub fn is_conflict_code(code: &str) -> bool {
    matches!(
        code,
        "UU" | "AA" | "DD" | "AU" | "UA" | "UD" | "DU"
    )
}

pub fn parse_status_line(line: &str) -> Option<FileEntry> {
    if line.len() < 4 {
        return None;
    }
    let code = line[..2].to_string();
    let path = line[3..].trim().to_string();
    let is_conflict = is_conflict_code(&code);
    let index_char = code.chars().next().unwrap_or(' ');
    let staged = !is_conflict && index_char != ' ' && index_char != '?';

    Some(FileEntry {
        path,
        status_code: code,
        staged,
        is_conflict,
        is_lfs: false,
    })
}

pub fn inspect(repo: &str) -> Result<RepoStatus, String> {
    let branch_out = cli::run(repo, &["branch", "--show-current"])?;
    let branch = if branch_out.is_empty() {
        "HEAD terlepas".into()
    } else {
        branch_out
    };

    let status_out = cli::run(repo, &["status", "--porcelain=v1"])?;
    let mut files: Vec<FileEntry> = status_out
        .lines()
        .filter_map(parse_status_line)
        .collect();

    // Check Git LFS status via git check-attr
    if !files.is_empty() {
        let mut check_args = vec!["check-attr", "filter", "--"];
        for f in &files {
            check_args.push(&f.path);
        }
        if let Ok(attr_out) = cli::run(repo, &check_args) {
            let lfs_paths: std::collections::HashSet<String> = attr_out
                .lines()
                .filter_map(|line| {
                    if line.ends_with(": filter: lfs") {
                        let path = line.split(": filter: lfs").next()?.trim();
                        Some(path.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            for f in &mut files {
                f.is_lfs = lfs_paths.contains(&f.path);
            }
        }
    }

    let has_conflicts = files.iter().any(|f| f.is_conflict);
    let git_dir = Path::new(repo).join(".git");
    let in_merge = git_dir.join("MERGE_HEAD").exists();
    let in_rebase = git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists();

    Ok(RepoStatus {
        branch,
        files,
        has_conflicts,
        in_merge,
        in_rebase,
    })
}

pub fn stage_file(repo: &str, file_path: &str) -> Result<(), String> {
    if file_path.trim().is_empty() {
        return Err("Path file tidak boleh kosong".into());
    }
    cli::run(repo, &["add", "--", file_path]).map(|_| ())
}

pub fn unstage_file(repo: &str, file_path: &str) -> Result<(), String> {
    if file_path.trim().is_empty() {
        return Err("Path file tidak boleh kosong".into());
    }
    cli::run(repo, &["restore", "--staged", "--", file_path]).map(|_| ())
}

pub fn stage_all(repo: &str) -> Result<(), String> {
    cli::run(repo, &["add", "--all"]).map(|_| ())
}

pub fn unstage_all(repo: &str) -> Result<(), String> {
    cli::run(repo, &["restore", "--staged", "."]).map(|_| ())
}

pub fn commit(
    repo: &str,
    summary: &str,
    description: Option<&str>,
    amend: bool,
) -> Result<String, String> {
    let sum = summary.trim();
    if sum.is_empty() && !amend {
        return Err("Pesan commit wajib diisi".into());
    }

    let mut full_msg = String::new();
    if !sum.is_empty() {
        full_msg.push_str(sum);
        if let Some(desc) = description {
            let d = desc.trim();
            if !d.is_empty() {
                full_msg.push_str("\n\n");
                full_msg.push_str(d);
            }
        }
    }

    let mut args = vec!["commit"];
    if amend {
        args.push("--amend");
    }
    if !full_msg.is_empty() {
        args.push("-m");
        args.push(&full_msg);
    } else if amend {
        args.push("--no-edit");
    }

    cli::run(repo, &args)
}

pub fn undo_last_commit(repo: &str) -> Result<String, String> {
    cli::run(repo, &["reset", "--soft", "HEAD~1"])
}

pub fn abort_merge(repo: &str) -> Result<(), String> {
    cli::run(repo, &["merge", "--abort"]).map(|_| ())
}

pub fn abort_rebase(repo: &str) -> Result<(), String> {
    cli::run(repo, &["rebase", "--abort"]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_untracked_file() {
        let entry = parse_status_line("?? new_file.txt").unwrap();
        assert_eq!(entry.path, "new_file.txt");
        assert_eq!(entry.status_code, "??");
        assert!(!entry.staged);
        assert!(!entry.is_conflict);
    }

    #[test]
    fn parse_staged_file() {
        let entry = parse_status_line("A  staged_file.txt").unwrap();
        assert_eq!(entry.path, "staged_file.txt");
        assert_eq!(entry.status_code, "A ");
        assert!(entry.staged);
        assert!(!entry.is_conflict);
    }

    #[test]
    fn parse_unstaged_modified() {
        let entry = parse_status_line(" M modified.txt").unwrap();
        assert_eq!(entry.path, "modified.txt");
        assert_eq!(entry.status_code, " M");
        assert!(!entry.staged);
        assert!(!entry.is_conflict);
    }

    #[test]
    fn parse_conflict_both_modified() {
        let entry = parse_status_line("UU conflict.txt").unwrap();
        assert_eq!(entry.path, "conflict.txt");
        assert_eq!(entry.status_code, "UU");
        assert!(entry.is_conflict);
        assert!(!entry.staged);
    }

    #[test]
    fn reject_blank_commit() {
        assert!(commit("/tmp", "   ", None, false).is_err());
    }
}
