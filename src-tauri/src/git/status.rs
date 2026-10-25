use serde::Serialize;
use super::cli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub status_code: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoStatus {
    pub branch: String,
    pub files: Vec<FileEntry>,
}

pub fn parse_status_line(line: &str) -> Option<FileEntry> {
    if line.len() < 4 {
        return None;
    }
    let code = line[..2].to_string();
    let path = line[3..].trim().to_string();
    let index_char = code.chars().next().unwrap_or(' ');
    let staged = index_char != ' ' && index_char != '?';
    Some(FileEntry {
        path,
        status_code: code,
        staged,
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
    let files = status_out
        .lines()
        .filter_map(parse_status_line)
        .collect();

    Ok(RepoStatus { branch, files })
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

pub fn commit(repo: &str, message: &str) -> Result<String, String> {
    let msg = message.trim();
    if msg.is_empty() {
        return Err("Pesan commit wajib diisi".into());
    }
    cli::run(repo, &["commit", "-m", msg])
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
    }

    #[test]
    fn parse_staged_file() {
        let entry = parse_status_line("A  staged_file.txt").unwrap();
        assert_eq!(entry.path, "staged_file.txt");
        assert_eq!(entry.status_code, "A ");
        assert!(entry.staged);
    }

    #[test]
    fn parse_unstaged_modified() {
        let entry = parse_status_line(" M modified.txt").unwrap();
        assert_eq!(entry.path, "modified.txt");
        assert_eq!(entry.status_code, " M");
        assert!(!entry.staged);
    }

    #[test]
    fn reject_blank_commit() {
        assert!(commit("/tmp", "   ").is_err());
    }
}
