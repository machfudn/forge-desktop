use serde::Serialize;
use super::cli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub date: String,
    pub summary: String,
}

pub fn parse_log_line(line: &str) -> Option<CommitInfo> {
    let parts: Vec<&str> = line.split('\x1f').collect();
    if parts.len() < 5 {
        return None;
    }
    Some(CommitInfo {
        sha: parts[0].to_string(),
        short_sha: parts[1].to_string(),
        author: parts[2].to_string(),
        date: parts[3].to_string(),
        summary: parts[4].to_string(),
    })
}

pub fn get_history(repo: &str, limit: usize) -> Result<Vec<CommitInfo>, String> {
    let limit_str = limit.to_string();
    let out = cli::run(
        repo,
        &[
            "log",
            "-n",
            &limit_str,
            "--pretty=format:%H%x1f%h%x1f%an%x1f%ad%x1f%s",
            "--date=short",
        ],
    )?;

    if out.is_empty() {
        return Ok(Vec::new());
    }

    Ok(out.lines().filter_map(parse_log_line).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_log_line() {
        let line = "abc123456789\x1fabc123\x1fUser Name\x1f2026-10-25\x1fInitial commit";
        let commit = parse_log_line(line).unwrap();
        assert_eq!(commit.short_sha, "abc123");
        assert_eq!(commit.author, "User Name");
        assert_eq!(commit.summary, "Initial commit");
    }

    #[test]
    fn parse_invalid_log_line() {
        assert!(parse_log_line("invalid line").is_none());
    }
}
