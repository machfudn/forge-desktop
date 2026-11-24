use serde::Serialize;
use std::collections::HashMap;
use super::cli;
use super::history::{parse_log_line, CommitInfo};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CompareFileItem {
    pub path: String,
    pub old_path: Option<String>,
    pub status_code: String,
    pub insertions: usize,
    pub deletions: usize,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CompareSummary {
    pub base: String,
    pub target: String,
    pub range_spec: String,
    pub is_three_dot: bool,
    pub merge_base: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub commits: Vec<CommitInfo>,
    pub files: Vec<CompareFileItem>,
    pub total_files: usize,
    pub total_insertions: usize,
    pub total_deletions: usize,
}

pub fn validate_git_ref(ref_str: &str) -> Result<&str, String> {
    let r = ref_str.trim();
    if r.is_empty() || r.starts_with('-') || r.contains('\0') || r.contains("..") {
        return Err("Referensi Git tidak valid".into());
    }
    // Prevent command injection or argument injection
    if r.chars().any(|c| {
        c.is_whitespace()
            || c == '"'
            || c == '\''
            || c == '`'
            || c == ';'
            || c == '|'
            || c == '&'
            || c == '$'
            || c == '<'
            || c == '>'
            || c == '\\'
    }) {
        return Err("Karakter tidak diizinkan dalam referensi Git".into());
    }
    Ok(r)
}

fn validate_file_path(path: &str) -> Result<&str, String> {
    let p = path.trim();
    if p.is_empty() || p.starts_with('-') || p.contains("..") {
        return Err("Path file tidak valid".into());
    }
    Ok(p)
}

fn parse_ahead_behind(out: &str) -> (usize, usize) {
    let trimmed = out.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() >= 2 {
        let behind = parts[0].parse::<usize>().unwrap_or(0);
        let ahead = parts[1].parse::<usize>().unwrap_or(0);
        (ahead, behind)
    } else {
        (0, 0)
    }
}

pub fn compare_revisions(
    repo: &str,
    base: &str,
    target: &str,
    three_dot: bool,
) -> Result<CompareSummary, String> {
    let valid_base = validate_git_ref(base)?;
    let valid_target = validate_git_ref(target)?;

    let range_spec = if three_dot {
        format!("{valid_base}...{valid_target}")
    } else {
        format!("{valid_base}..{valid_target}")
    };

    // 1. Find merge base
    let merge_base = match cli::run(repo, &["merge-base", valid_base, valid_target]) {
        Ok(out) => {
            let sha = out.trim().to_string();
            if !sha.is_empty() {
                Some(sha)
            } else {
                None
            }
        }
        Err(_) => None,
    };

    // 2. Compute ahead / behind
    let (ahead, behind) = match cli::run(
        repo,
        &[
            "rev-list",
            "--left-right",
            "--count",
            &format!("{valid_base}...{valid_target}"),
        ],
    ) {
        Ok(out) => parse_ahead_behind(&out),
        Err(_) => (0, 0),
    };

    // 3. Collect commits in range
    let log_out = cli::run(
        repo,
        &[
            "log",
            "--topo-order",
            "--shortstat",
            "--decorate=full",
            "--pretty=format:%x1e%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%s%x1f%D%x1f%P",
            "--date=format:%Y-%m-%d %H:%M",
            &range_spec,
        ],
    )
    .unwrap_or_default();

    let mut commits = Vec::new();
    if !log_out.trim().is_empty() {
        let entries: Vec<&str> = if log_out.contains('\x1e') {
            log_out.split('\x1e').collect()
        } else {
            log_out.lines().collect()
        };
        for entry in entries {
            if let Some(c) = parse_log_line(entry) {
                commits.push(c);
            }
        }
    }

    // 4. Collect file name status
    // Format: status\tpath or status\told_path\tnew_path
    let name_status_out = cli::run(
        repo,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
            "--name-status",
            &range_spec,
        ],
    )
    .unwrap_or_default();

    let mut status_map: HashMap<String, (String, Option<String>)> = HashMap::new();
    for line in name_status_out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = trimmed.split('\t').collect();
        if cols.is_empty() {
            continue;
        }
        let raw_status = cols[0].trim();
        let status_code = raw_status.chars().next().unwrap_or('M').to_string();

        if (raw_status.starts_with('R') || raw_status.starts_with('C')) && cols.len() >= 3 {
            let old_path = cols[1].trim().to_string();
            let new_path = cols[2].trim().to_string();
            status_map.insert(new_path, (status_code, Some(old_path)));
        } else if cols.len() >= 2 {
            let path = cols[1].trim().to_string();
            status_map.insert(path, (status_code, None));
        }
    }

    // 5. Collect file numstat (insertions / deletions)
    // Format: insertions\tdeletions\tpath
    let numstat_out = cli::run(
        repo,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
            "--numstat",
            &range_spec,
        ],
    )
    .unwrap_or_default();

    let mut numstat_map: HashMap<String, (usize, usize, bool)> = HashMap::new();
    for line in numstat_out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = trimmed.split('\t').collect();
        if cols.len() < 3 {
            continue;
        }
        let ins_str = cols[0].trim();
        let del_str = cols[1].trim();
        let raw_path = cols[2].trim();

        // If rename, numstat can output "{old => new}" or "old => new" or if tab-separated:
        // We resolve actual target path
        let file_path = if raw_path.contains(" => ") {
            if let Some((_, right)) = raw_path.split_once(" => ") {
                let clean_right = right.trim_end_matches('}');
                clean_right.trim().to_string()
            } else {
                raw_path.to_string()
            }
        } else {
            raw_path.to_string()
        };

        if ins_str == "-" && del_str == "-" {
            numstat_map.insert(file_path, (0, 0, true));
        } else {
            let ins = ins_str.parse::<usize>().unwrap_or(0);
            let del = del_str.parse::<usize>().unwrap_or(0);
            numstat_map.insert(file_path, (ins, del, false));
        }
    }

    // Combine into files list
    let mut files = Vec::new();
    let mut total_insertions = 0;
    let mut total_deletions = 0;

    for (path, (status_code, old_path)) in status_map {
        let (insertions, deletions, is_binary) = numstat_map
            .get(&path)
            .copied()
            .unwrap_or((0, 0, false));

        total_insertions += insertions;
        total_deletions += deletions;

        files.push(CompareFileItem {
            path,
            old_path,
            status_code,
            insertions,
            deletions,
            is_binary,
        });
    }

    // Sort files alphabetically by path
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let total_files = files.len();

    Ok(CompareSummary {
        base: valid_base.to_string(),
        target: valid_target.to_string(),
        range_spec,
        is_three_dot: three_dot,
        merge_base,
        ahead,
        behind,
        commits,
        files,
        total_files,
        total_insertions,
        total_deletions,
    })
}

pub fn get_compare_file_diff(
    repo: &str,
    base: &str,
    target: &str,
    file_path: &str,
    old_path: Option<&str>,
    three_dot: bool,
    ignore_whitespace: bool,
) -> Result<String, String> {
    let valid_base = validate_git_ref(base)?;
    let valid_target = validate_git_ref(target)?;
    let valid_path = validate_file_path(file_path)?;
    let valid_old = match old_path {
        Some(op) if !op.trim().is_empty() => Some(validate_file_path(op)?),
        _ => None,
    };

    let range_spec = if three_dot {
        format!("{valid_base}...{valid_target}")
    } else {
        format!("{valid_base}..{valid_target}")
    };

    let mut args = vec![
        "-c",
        "core.quotepath=false",
        "diff",
        "--no-color",
    ];

    if ignore_whitespace {
        args.push("-w");
    }

    args.push(&range_spec);
    args.push("--");

    if let Some(op) = valid_old {
        args.push(op);
    }
    args.push(valid_path);

    cli::run(repo, &args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_validate_git_ref() {
        assert!(validate_git_ref("main").is_ok());
        assert!(validate_git_ref("feature/login").is_ok());
        assert!(validate_git_ref("origin/main").is_ok());
        assert!(validate_git_ref("v1.2.3").is_ok());
        assert!(validate_git_ref("HEAD~1").is_ok());
        assert!(validate_git_ref("abc1234").is_ok());

        assert!(validate_git_ref("").is_err());
        assert!(validate_git_ref("-invalid").is_err());
        assert!(validate_git_ref("--upload-pack").is_err());
        assert!(validate_git_ref("foo..bar").is_err());
        assert!(validate_git_ref("foo;rm -rf /").is_err());
        assert!(validate_git_ref("foo | cat").is_err());
        assert!(validate_git_ref("foo\0bar").is_err());
    }

    #[test]
    fn test_compare_revisions_on_real_repo() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("forge-compare-test-{nanos}"));
        fs::create_dir_all(&root).unwrap();
        let root_str = root.to_str().unwrap();

        cli::run_raw(root_str, &["init", "-b", "main"]).unwrap();
        cli::run(root_str, &["config", "user.name", "Tester"]).unwrap();
        cli::run(root_str, &["config", "user.email", "test@example.com"]).unwrap();

        // Commit 1 on main
        fs::write(root.join("file1.txt"), "hello\nworld\n").unwrap();
        cli::run(root_str, &["add", "file1.txt"]).unwrap();
        cli::run(root_str, &["commit", "-m", "initial commit"]).unwrap();

        // Create feature branch
        cli::run(root_str, &["checkout", "-b", "feature"]).unwrap();
        fs::write(root.join("file2.txt"), "feature line\n").unwrap();
        fs::write(root.join("file1.txt"), "hello\nworld\nmodified\n").unwrap();
        cli::run(root_str, &["add", "."]).unwrap();
        cli::run(root_str, &["commit", "-m", "feature commit"]).unwrap();

        // Compare main and feature
        let summary = compare_revisions(root_str, "main", "feature", true).unwrap();
        assert_eq!(summary.base, "main");
        assert_eq!(summary.target, "feature");
        assert_eq!(summary.ahead, 1);
        assert_eq!(summary.behind, 0);
        assert_eq!(summary.commits.len(), 1);
        assert_eq!(summary.commits[0].summary, "feature commit");
        assert_eq!(summary.total_files, 2);

        // Diff file1.txt
        let diff1 = get_compare_file_diff(root_str, "main", "feature", "file1.txt", None, true, false).unwrap();
        assert!(diff1.contains("+modified"));

        // Cleanup
        let _ = fs::remove_dir_all(&root);
    }
}
