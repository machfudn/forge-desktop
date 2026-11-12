use std::fs;
use std::path::{Path, PathBuf};
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
    pub has_commits: bool,
}

pub fn is_conflict_code(code: &str) -> bool {
    matches!(
        code,
        "UU" | "AA" | "DD" | "AU" | "UA" | "UD" | "DU"
    )
}

fn clean_porcelain_path(raw: &str) -> String {
    let trimmed = raw.trim();
    let target = if let Some((_, rhs)) = trimmed.split_once(" -> ") {
        rhs.trim()
    } else {
        trimmed
    };
    if target.starts_with('"') && target.ends_with('"') && target.len() >= 2 {
        target[1..target.len() - 1].to_string()
    } else {
        target.to_string()
    }
}

pub fn parse_status_line(line: &str) -> Option<FileEntry> {
    if line.len() < 4 || !line.is_char_boundary(2) || !line.is_char_boundary(3) {
        return None;
    }
    let code = line[..2].to_string();
    let path = clean_porcelain_path(&line[3..]);
    if path.is_empty() {
        return None;
    }
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
pub fn parse_status_branch_header(header_line: &str) -> (String, bool) {
    let rest = header_line.strip_prefix("## ").unwrap_or(header_line).trim();
    if rest == "HEAD (no branch)" {
        return ("HEAD terlepas".into(), true);
    }
    if let Some(branch) = rest
        .strip_prefix("No commits yet on ")
        .or_else(|| rest.strip_prefix("Initial commit on "))
    {
        let name = branch.trim();
        return (
            if name.is_empty() {
                "HEAD terlepas".into()
            } else {
                name.to_string()
            },
            false,
        );
    }
    let branch_part = rest.split("...").next().unwrap_or(rest).trim();
    if branch_part.is_empty() {
        ("HEAD terlepas".into(), true)
    } else {
        (branch_part.to_string(), true)
    }
}

fn resolve_git_dir(repo: &str) -> PathBuf {
    let dot_git = Path::new(repo).join(".git");
    if dot_git.is_file() {
        if let Ok(content) = fs::read_to_string(&dot_git) {
            if let Some(raw) = content.trim().strip_prefix("gitdir:") {
                let target = Path::new(raw.trim());
                return if target.is_absolute() {
                    target.to_path_buf()
                } else {
                    Path::new(repo).join(target)
                };
            }
        }
    }
    dot_git
}

fn file_contains_lfs(path: &Path) -> bool {
    path.is_file()
        && fs::read_to_string(path)
            .map(|text| text.contains("lfs"))
            .unwrap_or(true)
}

fn repo_may_use_lfs(repo: &str, git_dir: &Path) -> bool {
    git_dir.join("lfs").exists()
        || file_contains_lfs(&git_dir.join("info").join("attributes"))
        || file_contains_lfs(&Path::new(repo).join(".gitattributes"))
}

pub fn inspect(repo: &str) -> Result<RepoStatus, String> {
    let status_out = cli::run(repo, &["status", "--porcelain=v1", "-b"])?;
    let mut lines = status_out.lines();
    let (branch, has_commits) = match lines.next() {
        Some(first) if first.starts_with("## ") => parse_status_branch_header(first),
        Some(other) => {
            let has_c = cli::run(repo, &["rev-parse", "--verify", "HEAD"]).is_ok();
            let br = cli::run(repo, &["branch", "--show-current"])
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "HEAD terlepas".into());
            let mut files: Vec<FileEntry> = std::iter::once(other)
                .chain(lines)
                .filter_map(parse_status_line)
                .collect();
            let git_dir = resolve_git_dir(repo);
            if !files.is_empty() && repo_may_use_lfs(repo, &git_dir) {
                mark_lfs_files(repo, &mut files);
            }
            let has_conflicts = files.iter().any(|f| f.is_conflict);
            let in_merge = git_dir.join("MERGE_HEAD").exists();
            let in_rebase =
                git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists();
            return Ok(RepoStatus {
                branch: br.trim().to_string(),
                files,
                has_conflicts,
                in_merge,
                in_rebase,
                has_commits: has_c,
            });
        }
        None => ("HEAD terlepas".into(), false),
    };

    let mut files: Vec<FileEntry> = lines.filter_map(parse_status_line).collect();
    let git_dir = resolve_git_dir(repo);

    // Check Git LFS status only when repo has LFS configuration/state
    if !files.is_empty() && repo_may_use_lfs(repo, &git_dir) {
        mark_lfs_files(repo, &mut files);
    }

    let has_conflicts = files.iter().any(|f| f.is_conflict);
    let in_merge = git_dir.join("MERGE_HEAD").exists();
    let in_rebase = git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists();

    Ok(RepoStatus {
        branch,
        files,
        has_conflicts,
        in_merge,
        in_rebase,
        has_commits,
    })
}

fn mark_lfs_files(repo: &str, files: &mut [FileEntry]) {
    let mut lfs_paths = std::collections::HashSet::new();
    for chunk in files.chunks(80) {
        let mut check_args = vec!["check-attr", "filter", "--"];
        for f in chunk {
            check_args.push(&f.path);
        }
        if let Ok(attr_out) = cli::run(repo, &check_args) {
            for line in attr_out.lines() {
                if line.ends_with(": filter: lfs") {
                    if let Some(path) = line.split(": filter: lfs").next() {
                        lfs_paths.insert(path.trim().to_string());
                    }
                }
            }
        }
    }
    for f in files {
        f.is_lfs = lfs_paths.contains(&f.path);
    }
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
    if cli::run(repo, &["rev-parse", "--verify", "HEAD"]).is_ok() {
        cli::run(repo, &["restore", "--staged", "--", file_path]).map(|_| ())
    } else {
        cli::run(repo, &["rm", "--cached", "-q", "--", file_path]).map(|_| ())
    }
}

pub fn stage_all(repo: &str) -> Result<(), String> {
    cli::run(repo, &["add", "--all"]).map(|_| ())
}

pub fn unstage_all(repo: &str) -> Result<(), String> {
    if cli::run(repo, &["rev-parse", "--verify", "HEAD"]).is_ok() {
        cli::run(repo, &["restore", "--staged", "."]).map(|_| ())
    } else {
        cli::run(repo, &["rm", "--cached", "-r", "-q", "--", "."]).map(|_| ())
    }
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
    if cli::run(repo, &["rev-parse", "--verify", "HEAD"]).is_err() {
        return Err("Belum ada komit untuk dibatalkan.".into());
    }
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
    #[test]
    fn parse_renamed_and_quoted_paths() {
        let renamed = parse_status_line("R  old.txt -> \"new file.txt\"").unwrap();
        assert_eq!(renamed.path, "new file.txt");
        assert!(renamed.staged);
    }

    #[test]
    fn parses_status_branch_headers() {
        assert_eq!(
            parse_status_branch_header("## No commits yet on main"),
            ("main".to_string(), false)
        );
        assert_eq!(
            parse_status_branch_header("## HEAD (no branch)"),
            ("HEAD terlepas".to_string(), true)
        );
        assert_eq!(
            parse_status_branch_header("## feat/perf...origin/feat/perf [ahead 2, behind 1]"),
            ("feat/perf".to_string(), true)
        );
        assert_eq!(
            parse_status_branch_header("## main"),
            ("main".to_string(), true)
        );
    }
}
