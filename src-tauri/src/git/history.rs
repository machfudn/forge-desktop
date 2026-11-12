use std::collections::HashSet;
use serde::Serialize;
use super::cli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommitRef {
    pub kind: String,
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub date: String,
    pub summary: String,
    pub is_pushed: bool,
    pub tags: Vec<String>,
    pub parents: Vec<String>,
    pub refs: Vec<CommitRef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BlameLine {
    pub line_no: usize,
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub author_email: String,
    pub date: String,
    pub summary: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommitChangedFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status_code: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommitDetail {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub author_email: String,
    pub date: String,
    pub summary: String,
    pub description: String,
    pub files: Vec<CommitChangedFile>,
}

fn parse_refs_from_decorate(dec: &str) -> (Vec<String>, Vec<CommitRef>) {
    let mut tags = Vec::new();
    let mut refs = Vec::new();
    for raw_item in dec.split(", ") {
        let item = raw_item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some(tag) = item.strip_prefix("tag: refs/tags/") {
            let t = tag.trim();
            if !t.is_empty() {
                tags.push(t.to_string());
                refs.push(CommitRef {
                    kind: "tag".into(),
                    name: t.to_string(),
                    is_current: false,
                });
            }
        } else if let Some(target) = item.strip_prefix("HEAD -> refs/heads/") {
            let name = target.trim();
            if !name.is_empty() {
                refs.push(CommitRef {
                    kind: "branch".into(),
                    name: name.to_string(),
                    is_current: true,
                });
            }
        } else if item == "HEAD" {
            refs.push(CommitRef {
                kind: "head".into(),
                name: "HEAD".into(),
                is_current: true,
            });
        } else if let Some(branch) = item.strip_prefix("refs/heads/") {
            let name = branch.trim();
            if !name.is_empty() {
                refs.push(CommitRef {
                    kind: "branch".into(),
                    name: name.to_string(),
                    is_current: false,
                });
            }
        } else if let Some(remote) = item.strip_prefix("refs/remotes/") {
            let name = remote.trim();
            if !name.is_empty() && !name.ends_with("/HEAD") {
                refs.push(CommitRef {
                    kind: "remote".into(),
                    name: name.to_string(),
                    is_current: false,
                });
            }
        }
    }
    (tags, refs)
}

pub fn parse_log_line_with_status(
    line: &str,
    unpushed: &HashSet<String>,
) -> Option<CommitInfo> {
    let parts: Vec<&str> = line.split('\x1f').collect();
    if parts.len() < 5 {
        return None;
    }
    let sha = parts[0].to_string();
    let is_pushed = !unpushed.contains(&sha);
    let (tags, refs) = parts
        .get(5)
        .map(|dec| parse_refs_from_decorate(dec))
        .unwrap_or_default();
    let parents = parts
        .get(6)
        .map(|p| {
            p.split_whitespace()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    Some(CommitInfo {
        sha,
        short_sha: parts[1].to_string(),
        author: parts[2].to_string(),
        date: parts[3].to_string(),
        summary: parts[4].to_string(),
        is_pushed,
        tags,
        parents,
        refs,
    })
}

pub fn parse_log_line(line: &str) -> Option<CommitInfo> {
    parse_log_line_with_status(line, &HashSet::new())
}

fn get_unpushed_shas(repo: &str, limit: usize) -> HashSet<String> {
    let limit_str = limit.max(1).saturating_mul(2).to_string();
    let out = cli::run(repo, &["rev-list", "-n", &limit_str, "@{u}..HEAD"])
        .or_else(|_| cli::run(repo, &["rev-list", "-n", &limit_str, "HEAD", "--not", "--remotes"]));
    match out {
        Ok(text) => text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect(),
        Err(_) => HashSet::new(),
    }
}

pub fn get_history_with_options(
    repo: &str,
    limit: usize,
    all_branches: bool,
) -> Result<Vec<CommitInfo>, String> {
    let limit_str = limit.to_string();
    let (log_res, unpushed) = std::thread::scope(|s| {
        let log_h = s.spawn(|| {
            let mut args = vec![
                "log",
                "-n",
                &limit_str,
                "--topo-order",
                "--decorate=full",
                "--pretty=format:%H%x1f%h%x1f%an%x1f%ad%x1f%s%x1f%D%x1f%P",
                "--date=short",
            ];
            if all_branches {
                args.push("--all");
            }
            cli::run(repo, &args)
        });
        let unpushed_h = s.spawn(|| get_unpushed_shas(repo, limit));
        (
            log_h.join().unwrap_or_else(|_| Err("Thread log panik".into())),
            unpushed_h.join().unwrap_or_default(),
        )
    });

    let out = match log_res {
        Ok(text) => text,
        Err(err) => {
            let lower = err.to_ascii_lowercase();
            if lower.contains("does not have any commits yet") || lower.contains("bad default revision") {
                return Ok(Vec::new());
            }
            return Err(err);
        }
    };
    if out.is_empty() {
        return Ok(Vec::new());
    }

    Ok(out
        .lines()
        .filter_map(|line| parse_log_line_with_status(line, &unpushed))
        .collect())
}

pub fn get_history(repo: &str, limit: usize) -> Result<Vec<CommitInfo>, String> {
    get_history_with_options(repo, limit, false)
}

fn validate_commit_sha(sha: &str) -> Result<&str, String> {
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

fn validate_commit_file_path(path: &str) -> Result<&str, String> {
    let p = path.trim();
    if p.is_empty() || p.starts_with('-') || p.contains("..") {
        return Err("Path file tidak valid".into());
    }
    Ok(p)
}

pub fn parse_commit_detail(raw: &str) -> Result<CommitDetail, String> {
    let (meta_raw, files_raw) = raw.split_once('\x1e').unwrap_or((raw, ""));
    let parts: Vec<&str> = meta_raw.split('\x1f').collect();
    if parts.len() < 7 {
        return Err("Format detail commit tidak valid".into());
    }

    let mut files = Vec::new();
    for line in files_raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = trimmed.split('\t').collect();
        let status_raw = cols[0].trim();
        let status_code = status_raw
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "M".to_string());

        if (status_raw.starts_with('R') || status_raw.starts_with('C')) && cols.len() >= 3 {
            files.push(CommitChangedFile {
                old_path: Some(cols[1].to_string()),
                path: cols[2].to_string(),
                status_code,
            });
        } else if cols.len() >= 2 {
            files.push(CommitChangedFile {
                old_path: None,
                path: cols[1].to_string(),
                status_code,
            });
        }
    }

    Ok(CommitDetail {
        sha: parts[0].trim().to_string(),
        short_sha: parts[1].trim().to_string(),
        author: parts[2].trim().to_string(),
        author_email: parts[3].trim().to_string(),
        date: parts[4].trim().to_string(),
        summary: parts[5].trim().to_string(),
        description: parts[6].replace("\r\n", "\n").trim().to_string(),
        files,
    })
}

pub fn get_commit_detail(repo: &str, sha: &str) -> Result<CommitDetail, String> {
    let valid_sha = validate_commit_sha(sha)?;
    let out = cli::run(
        repo,
        &[
            "-c",
            "core.quotepath=false",
            "show",
            "-m",
            "--first-parent",
            "--root",
            "--no-color",
            "--date=short",
            "--name-status",
            "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%s%x1f%b%x1e",
            valid_sha,
        ],
    )?;
    parse_commit_detail(&out)
}

pub fn get_commit_file_diff(
    repo: &str,
    sha: &str,
    file_path: &str,
    old_path: Option<&str>,
    ignore_whitespace: bool,
) -> Result<String, String> {
    let valid_sha = validate_commit_sha(sha)?;
    let valid_path = validate_commit_file_path(file_path)?;
    let valid_old = match old_path {
        Some(op) if !op.trim().is_empty() => Some(validate_commit_file_path(op)?),
        _ => None,
    };

    let mut args = vec![
        "-c",
        "core.quotepath=false",
        "show",
        "-m",
        "--first-parent",
        "--root",
        "--no-color",
        "--format=",
    ];
    if ignore_whitespace {
        args.push("-w");
    }
    args.push(valid_sha);
    args.push("--");
    if let Some(op) = valid_old {
        args.push(op);
    }
    args.push(valid_path);

    cli::run(repo, &args)
}

pub fn get_file_history(
    repo: &str,
    file_path: &str,
    limit: usize,
) -> Result<Vec<CommitInfo>, String> {
    let valid_path = validate_commit_file_path(file_path)?;
    let limit_str = limit.max(1).to_string();
    let out = cli::run(
        repo,
        &[
            "-c",
            "core.quotepath=false",
            "log",
            "-n",
            &limit_str,
            "--follow",
            "--decorate=full",
            "--pretty=format:%H%x1f%h%x1f%an%x1f%ad%x1f%s%x1f%D%x1f%P",
            "--date=short",
            "--",
            valid_path,
        ],
    )?;
    if out.is_empty() {
        return Ok(Vec::new());
    }
    Ok(out.lines().filter_map(parse_log_line).collect())
}

fn format_unix_date(secs: i64) -> String {
    // Civil-from-days (Howard Hinnant algorithm) so zero external date crate needed
    let z = secs.div_euclid(86_400) + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    format!("{year:04}-{m:02}-{d:02}")
}

pub fn parse_blame_porcelain(raw: &str) -> Vec<BlameLine> {
    use std::collections::HashMap;
    struct CommitMeta {
        author: String,
        author_email: String,
        date: String,
        summary: String,
    }
    let mut cache: HashMap<String, CommitMeta> = HashMap::new();
    let mut result = Vec::new();
    let mut current_sha = String::new();
    let mut current_line_no: usize = 0;
    let mut author = String::new();
    let mut author_email = String::new();
    let mut date = String::new();
    let mut summary = String::new();

    for line in raw.lines() {
        if let Some(content) = line.strip_prefix('\t') {
            if !current_sha.is_empty() {
                let entry = cache.entry(current_sha.clone()).or_insert_with(|| CommitMeta {
                    author: author.clone(),
                    author_email: author_email.clone(),
                    date: date.clone(),
                    summary: summary.clone(),
                });
                let short_sha: String = current_sha.chars().take(7).collect();
                result.push(BlameLine {
                    line_no: current_line_no,
                    sha: current_sha.clone(),
                    short_sha,
                    author: entry.author.clone(),
                    author_email: entry.author_email.clone(),
                    date: entry.date.clone(),
                    summary: entry.summary.clone(),
                    content: content.to_string(),
                });
            }
            current_sha.clear();
            author.clear();
            author_email.clear();
            date.clear();
            summary.clear();
            continue;
        }
        if current_sha.is_empty() {
            let mut parts = line.split_whitespace();
            if let (Some(sha), Some(_orig_ln), Some(final_ln)) =
                (parts.next(), parts.next(), parts.next())
            {
                if sha.len() >= 7 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    current_sha = sha.to_string();
                    current_line_no = final_ln.parse::<usize>().unwrap_or(result.len() + 1);
                    if let Some(cached) = cache.get(&current_sha) {
                        author = cached.author.clone();
                        author_email = cached.author_email.clone();
                        date = cached.date.clone();
                        summary = cached.summary.clone();
                    }
                }
            }
        } else if let Some(val) = line.strip_prefix("author ") {
            author = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("author-mail ") {
            author_email = val.trim().trim_matches(|c| c == '<' || c == '>').to_string();
        } else if let Some(val) = line.strip_prefix("author-time ") {
            if let Ok(ts) = val.trim().parse::<i64>() {
                date = format_unix_date(ts);
            }
        } else if let Some(val) = line.strip_prefix("summary ") {
            summary = val.trim().to_string();
        }
    }
    result
}

pub fn get_file_blame(
    repo: &str,
    file_path: &str,
    rev: Option<&str>,
) -> Result<Vec<BlameLine>, String> {
    let valid_path = validate_commit_file_path(file_path)?;
    let mut args = vec![
        "-c",
        "core.quotepath=false",
        "blame",
        "--line-porcelain",
    ];
    let valid_rev;
    if let Some(r) = rev.map(str::trim).filter(|s| !s.is_empty()) {
        valid_rev = validate_commit_sha(r)?;
        args.push(valid_rev);
    }
    args.push("--");
    args.push(valid_path);
    let out = cli::run(repo, &args)?;
    Ok(parse_blame_porcelain(&out))
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
        assert!(commit.is_pushed);
    }

    #[test]
    fn parse_log_line_marks_unpushed_commit() {
        let unpushed = HashSet::from(["local999".to_string()]);
        let local = parse_log_line_with_status(
            "local999\x1flocal9\x1fDev\x1f2026-11-10\x1flocal commit",
            &unpushed,
        )
        .unwrap();
        let remote = parse_log_line_with_status(
            "remote111\x1fremote1\x1fDev\x1f2026-11-09\x1fpushed commit",
            &unpushed,
        )
        .unwrap();
        assert!(!local.is_pushed);
        assert!(remote.is_pushed);
    }

    #[test]
    fn parse_invalid_log_line() {
        assert!(parse_log_line("invalid line").is_none());
    }

    #[test]
    fn parse_log_line_extracts_multiple_tags_and_ignores_other_refs() {
        let line = "abc123456789\x1fabc123\x1fUser Name\x1f2026-11-12\x1fRelease commit\x1fHEAD -> refs/heads/main, tag: refs/tags/v1.1.1, tag: refs/tags/release/v1,2, refs/remotes/origin/main";
        let commit = parse_log_line(line).unwrap();
        assert_eq!(commit.tags, vec!["v1.1.1", "release/v1,2"]);
    }

    #[test]
    fn get_history_includes_lightweight_and_annotated_tags() {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("forge-history-tags-test-{nanos}"));
        std::fs::create_dir_all(&root).unwrap();

        let run_git = |args: &[&str]| {
            let out = Command::new("git").args(args).current_dir(&root).output().unwrap();
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };

        run_git(&["init"]);
        run_git(&["config", "user.name", "Tester"]);
        run_git(&["config", "user.email", "tester@example.com"]);

        std::fs::write(root.join("a.txt"), "one\n").unwrap();
        run_git(&["add", "a.txt"]);
        run_git(&["commit", "-m", "tagged commit"]);
        run_git(&["tag", "v1.0.0"]);
        run_git(&["tag", "-a", "v1.0.1", "-m", "annotated release"]);

        std::fs::write(root.join("b.txt"), "two\n").unwrap();
        run_git(&["add", "b.txt"]);
        run_git(&["commit", "-m", "untagged commit"]);

        let history = get_history(root.to_str().unwrap(), 10).unwrap();
        assert_eq!(history.len(), 2);
        assert!(history[0].tags.is_empty());
        assert_eq!(history[1].tags.len(), 2);
        assert!(history[1].tags.contains(&"v1.0.0".to_string()));
        assert!(history[1].tags.contains(&"v1.0.1".to_string()));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn get_history_distinguishes_pushed_and_unpushed_commits() {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("forge-history-push-test-{nanos}"));
        let bare = root.join("remote.git");
        let local = root.join("local");
        std::fs::create_dir_all(&bare).unwrap();
        std::fs::create_dir_all(&local).unwrap();

        let run_git = |cwd: &std::path::Path, args: &[&str]| {
            let out = Command::new("git").args(args).current_dir(cwd).output().unwrap();
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };

        run_git(&bare, &["init", "--bare"]);
        run_git(&local, &["init"]);
        run_git(&local, &["config", "user.name", "Tester"]);
        run_git(&local, &["config", "user.email", "tester@example.com"]);
        run_git(&local, &["remote", "add", "origin", bare.to_str().unwrap()]);

        std::fs::write(local.join("a.txt"), "one\n").unwrap();
        run_git(&local, &["add", "a.txt"]);
        run_git(&local, &["commit", "-m", "initial pushed"]);
        run_git(&local, &["push", "-u", "origin", "HEAD:main"]);
        run_git(&local, &["fetch", "origin"]);
        let cur_branch = cli::run(local.to_str().unwrap(), &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        let _ = cli::run(
            local.to_str().unwrap(),
            &["branch", "--set-upstream-to=origin/main", cur_branch.trim()],
        );

        std::fs::write(local.join("b.txt"), "two\n").unwrap();
        run_git(&local, &["add", "b.txt"]);
        run_git(&local, &["commit", "-m", "local unpushed"]);

        let history = get_history(local.to_str().unwrap(), 10).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].summary, "local unpushed");
        assert!(!history[0].is_pushed);
        assert_eq!(history[1].summary, "initial pushed");
        assert!(history[1].is_pushed);

        // New branch without upstream falls back to --not --remotes
        run_git(&local, &["checkout", "-b", "feat/no-upstream"]);
        std::fs::write(local.join("c.txt"), "three\n").unwrap();
        run_git(&local, &["add", "c.txt"]);
        run_git(&local, &["commit", "-m", "branch unpushed"]);

        let feat_history = get_history(local.to_str().unwrap(), 10).unwrap();
        assert_eq!(feat_history.len(), 3);
        assert_eq!(feat_history[0].summary, "branch unpushed");
        assert!(!feat_history[0].is_pushed);
        assert_eq!(feat_history[2].summary, "initial pushed");
        assert!(feat_history[2].is_pushed);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn get_history_returns_empty_on_unborn_repo() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("forge-unborn-hist-{nanos}"));
        std::fs::create_dir_all(&root).unwrap();
        cli::run_raw(root.to_str().unwrap(), &["init", "-b", "main"]).unwrap();
        let history = get_history(root.to_str().unwrap(), 20).unwrap();
        assert!(history.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parse_commit_detail_with_multiline_body_and_renames() {
        let raw = "0123456789abcdef\x1f0123456\x1fDev\x1fdev@example.com\x1f2026-11-10\x1ffeat: add detail view\x1fLine one\n\n* Bullet A\n* Bullet B\n\x1e\nM\tsrc/App.svelte\nA\tsrc/New.svelte\nR100\told.ts\tnew.ts\n";
        let detail = parse_commit_detail(raw).unwrap();
        assert_eq!(detail.sha, "0123456789abcdef");
        assert_eq!(detail.author_email, "dev@example.com");
        assert_eq!(detail.description, "Line one\n\n* Bullet A\n* Bullet B");
        assert_eq!(detail.files.len(), 3);
        assert_eq!(detail.files[0].status_code, "M");
        assert_eq!(detail.files[0].path, "src/App.svelte");
        assert_eq!(detail.files[2].status_code, "R");
        assert_eq!(detail.files[2].old_path.as_deref(), Some("old.ts"));
        assert_eq!(detail.files[2].path, "new.ts");
    }

    #[test]
    fn parse_commit_detail_empty_description_and_files() {
        let raw = "0123456789abcdef\x1f0123456\x1fDev\x1fdev@example.com\x1f2026-11-10\x1fchore: empty\x1f\x1e\n";
        let detail = parse_commit_detail(raw).unwrap();
        assert_eq!(detail.description, "");
        assert!(detail.files.is_empty());
    }

    #[test]
    fn reject_invalid_commit_sha_and_path() {
        assert!(get_commit_detail("/tmp", "--exec=bad").is_err());
        assert!(get_commit_detail("/tmp", "xyz123").is_err());
        assert!(get_commit_file_diff("/tmp", "abcdef1", "../secret", None, false).is_err());
        assert!(get_commit_file_diff("/tmp", "abcdef1", "-p", None, false).is_err());
    }

    #[test]
    fn parse_refs_and_parents_from_log_line() {
        let line = "aaaa1111bbbb2222\x1faaaa111\x1fDev\x1f2026-11-12\x1fMerge PR\x1fHEAD -> refs/heads/main, refs/remotes/origin/main, refs/heads/feat, tag: refs/tags/v2.0\x1fpppp1111 pppp2222";
        let commit = parse_log_line(line).unwrap();
        assert_eq!(commit.parents, vec!["pppp1111", "pppp2222"]);
        assert_eq!(commit.tags, vec!["v2.0"]);
        assert!(commit.refs.iter().any(|r| r.kind == "branch" && r.name == "main" && r.is_current));
        assert!(commit.refs.iter().any(|r| r.kind == "remote" && r.name == "origin/main"));
    }

    #[test]
    fn parse_blame_porcelain_extracts_lines_and_caches_commit_meta() {
        let raw = "0123456789abcdef0123456789abcdef01234567 1 1 2\nauthor Alice\nauthor-mail <alice@example.com>\nauthor-time 1700000000\nsummary init file\nfilename src/main.rs\n\tfn main() {}\n0123456789abcdef0123456789abcdef01234567 2 2\n\tprintln!(\"hi\");\n";
        let lines = parse_blame_porcelain(raw);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].author, "Alice");
        assert_eq!(lines[0].content, "fn main() {}");
        assert_eq!(lines[1].author, "Alice");
        assert_eq!(lines[1].line_no, 2);
        assert_eq!(lines[1].content, "println!(\"hi\");");
    }
}
