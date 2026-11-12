use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::Serialize;
use super::providers::{parse_hosting_repo, HostingProvider};
use super::cli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoConfig {
    pub user_name: String,
    pub user_email: String,
}

fn parse_user_config_regexp(out: &str) -> RepoConfig {
    let mut user_name = String::new();
    let mut user_email = String::new();
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (key, val) = match trimmed.split_once(char::is_whitespace) {
            Some((k, v)) => (k.trim(), v.trim()),
            None => (trimmed, ""),
        };
        match key {
            "user.name" => user_name = val.to_string(),
            "user.email" => user_email = val.to_string(),
            _ => {}
        }
    }
    RepoConfig {
        user_name,
        user_email,
    }
}

pub fn get_repo_config(repo: &str) -> Result<RepoConfig, String> {
    let out = cli::run(repo, &["config", "--get-regexp", r"^user\.(name|email)$"])
        .unwrap_or_default();
    Ok(parse_user_config_regexp(&out))
}

pub fn set_repo_config(repo: &str, user_name: &str, user_email: &str) -> Result<(), String> {
    let name = user_name.trim();
    let email = user_email.trim();
    if !name.is_empty() {
        cli::run(repo, &["config", "user.name", name])?;
    }
    if !email.is_empty() {
        cli::run(repo, &["config", "user.email", email])?;
    }
    Ok(())
}

pub fn get_global_config() -> Result<RepoConfig, String> {
    let out = cli::run_raw(".", &["config", "--global", "--get-regexp", r"^user\.(name|email)$"])
        .unwrap_or_default();
    Ok(parse_user_config_regexp(&out))
}

pub fn set_global_config(user_name: &str, user_email: &str) -> Result<(), String> {
    let name = user_name.trim();
    let email = user_email.trim();
    if !name.is_empty() {
        cli::run_raw(".", &["config", "--global", "user.name", name])?;
    }
    if !email.is_empty() {
        cli::run_raw(".", &["config", "--global", "user.email", email])?;
    }
    Ok(())
}

pub fn init_repository(
    target_path: &str,
    default_branch: Option<&str>,
    gitignore_content: Option<&str>,
    license_content: Option<&str>,
) -> Result<String, String> {
    let path = target_path.trim();
    if path.is_empty() {
        return Err("Path repositori tidak boleh kosong".into());
    }
    let p = Path::new(path);
    if !p.exists() {
        fs::create_dir_all(p).map_err(|e| format!("Gagal membuat direktori {path}: {e}"))?;
    }

    let branch = default_branch.unwrap_or("main").trim();
    let branch_arg = if branch.is_empty() { "main" } else { branch };

    cli::run_raw(path, &["init", "-b", branch_arg])?;

    if let Some(gi) = gitignore_content {
        let trimmed = gi.trim();
        let gi_path = p.join(".gitignore");
        if !trimmed.is_empty() && !gi_path.exists() {
            fs::write(gi_path, format!("{trimmed}\n"))
                .map_err(|e| format!("Gagal menulis .gitignore: {e}"))?;
        }
    }

    if let Some(lic) = license_content {
        let trimmed = lic.trim();
        let lic_path = p.join("LICENSE");
        if !trimmed.is_empty() && !lic_path.exists() {
            fs::write(lic_path, format!("{trimmed}\n"))
                .map_err(|e| format!("Gagal menulis LICENSE: {e}"))?;
        }
    }

    Ok(format!("Repositori berhasil diinisialisasi di {path}"))
}

pub fn check_repo_exists(path: &str) -> bool {
    let p = Path::new(path);
    p.exists() && p.is_dir() && p.join(".git").exists()
}

pub fn check_repos_exists(paths: &[String]) -> HashMap<String, bool> {
    paths
        .iter()
        .map(|p| (p.clone(), check_repo_exists(p)))
        .collect()
}


#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoQuickSummary {
    pub path: String,
    pub exists: bool,
    pub is_git: bool,
    pub branch: String,
    pub remote_url: Option<String>,
    pub provider: Option<HostingProvider>,
    pub dirty_count: usize,
    pub ahead: usize,
    pub behind: usize,
    pub unpushed_commits: usize,
    pub has_remote: bool,
    pub has_upstream: bool,
    pub is_unborn: bool,
}

fn run_git_no_lock(repo: &str, args: &[&str]) -> Option<String> {
    let out = crate::system::command_no_window("git")
        .arg("-C")
        .arg(repo)
        .arg("--no-optional-locks")
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn inspect_repo_summary(path: &str) -> RepoQuickSummary {
    let p = Path::new(path);
    let exists = p.exists() && p.is_dir();
    let is_git = exists && p.join(".git").exists();
    let mut summary = RepoQuickSummary {
        path: path.to_string(),
        exists,
        is_git,
        branch: String::new(),
        remote_url: None,
        provider: None,
        dirty_count: 0,
        ahead: 0,
        behind: 0,
        unpushed_commits: 0,
        has_remote: false,
        has_upstream: false,
        is_unborn: false,
    };
    if !is_git {
        return summary;
    }

    if let Some(status_out) = run_git_no_lock(
        path,
        &["status", "--porcelain=v2", "--branch", "--untracked-files=normal"],
    ) {
        for line in status_out.lines() {
            if let Some(oid) = line.strip_prefix("# branch.oid ") {
                if oid.trim() == "(initial)" {
                    summary.is_unborn = true;
                }
            } else if let Some(head) = line.strip_prefix("# branch.head ") {
                summary.branch = head.trim().to_string();
            } else if let Some(up) = line.strip_prefix("# branch.upstream ") {
                if !up.trim().is_empty() {
                    summary.has_upstream = true;
                }
            } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
                for tok in ab.split_whitespace() {
                    if let Some(a) = tok.strip_prefix('+') {
                        summary.ahead = a.parse::<usize>().unwrap_or(0);
                    } else if let Some(b) = tok.strip_prefix('-') {
                        summary.behind = b.parse::<usize>().unwrap_or(0);
                    }
                }
            } else if !line.trim().is_empty() && !line.starts_with('#') {
                summary.dirty_count += 1;
            }
        }
    }

    if let Some(cfg_out) = run_git_no_lock(path, &["config", "--get-regexp", r"^remote\..*\.url$"]) {
        let mut origin_url: Option<String> = None;
        let mut first_url: Option<String> = None;
        for line in cfg_out.lines() {
            let trimmed = line.trim();
            if let Some((k, v)) = trimmed.split_once(char::is_whitespace) {
                let val = v.trim();
                if val.is_empty() {
                    continue;
                }
                if first_url.is_none() {
                    first_url = Some(val.to_string());
                }
                if k == "remote.origin.url" {
                    origin_url = Some(val.to_string());
                }
            }
        }
        let chosen = origin_url.or(first_url);
        if let Some(url) = chosen {
            summary.has_remote = true;
            summary.provider = parse_hosting_repo(&url).map(|parsed| parsed.provider);
            summary.remote_url = Some(url);
        }
    }

    if summary.has_upstream {
        summary.unpushed_commits = summary.ahead;
    } else if !summary.is_unborn {
        let count_str = run_git_no_lock(path, &["rev-list", "--count", "HEAD", "--not", "--remotes"])
            .or_else(|| run_git_no_lock(path, &["rev-list", "--count", "HEAD"]));
        if let Some(s) = count_str {
            summary.unpushed_commits = s.trim().parse::<usize>().unwrap_or(0);
        }
    }

    summary
}

pub fn batch_inspect_repo_summaries(paths: &[String]) -> HashMap<String, RepoQuickSummary> {
    if paths.is_empty() {
        return HashMap::new();
    }
    std::thread::scope(|s| {
        let handles: Vec<_> = paths
            .iter()
            .map(|p| s.spawn(move || (p.clone(), inspect_repo_summary(p))))
            .collect();
        handles
            .into_iter()
            .filter_map(|h| h.join().ok())
            .collect()
    })
}
pub fn get_repo_path_state(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "missing".into();
    }
    let p = Path::new(trimmed);
    if !p.exists() || !p.is_dir() {
        "missing".into()
    } else if p.join(".git").exists() {
        "git".into()
    } else {
        "non_git".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_empty_init_path() {
        assert!(init_repository("   ", None, None, None).is_err());
    }

    #[test]
    fn repo_path_state_detects_missing_non_git_and_git() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("forge-repo-state-{nanos}"));
        let missing = root.join("does-not-exist");
        assert_eq!(get_repo_path_state(missing.to_str().unwrap()), "missing");

        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".gitignore"), "custom-keep\n").unwrap();
        assert_eq!(get_repo_path_state(root.to_str().unwrap()), "non_git");

        init_repository(root.to_str().unwrap(), Some("main"), Some("ignored/\n"), None).unwrap();
        assert_eq!(get_repo_path_state(root.to_str().unwrap()), "git");
        // Existing .gitignore must not be overwritten
        assert_eq!(std::fs::read_to_string(root.join(".gitignore")).unwrap(), "custom-keep\n");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parses_user_config_regexp_last_wins() {
        let parsed = parse_user_config_regexp(
            "user.name Global User\nuser.email global@example.com\nuser.name Local User\n",
        );
        assert_eq!(parsed.user_name, "Local User");
        assert_eq!(parsed.user_email, "global@example.com");
    }

    #[test]
    fn batch_check_repos_exists_reports_each_path() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("forge-batch-repos-{nanos}"));
        let valid = root.join("valid");
        let missing = root.join("missing");
        init_repository(valid.to_str().unwrap(), Some("main"), None, None).unwrap();

        let paths = vec![
            valid.to_string_lossy().to_string(),
            missing.to_string_lossy().to_string(),
        ];
        let result = check_repos_exists(&paths);
        assert_eq!(result.get(&paths[0]), Some(&true));
        assert_eq!(result.get(&paths[1]), Some(&false));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn batch_inspect_repo_summaries_reports_provider_dirty_and_ahead_counts() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("forge-repo-summary-{nanos}"));
        let repo_dir = root.join("app");
        let missing_dir = root.join("missing");
        init_repository(repo_dir.to_str().unwrap(), Some("main"), None, None).unwrap();

        let run_git = |args: &[&str]| {
            let out = crate::system::command_no_window("git")
                .args(args)
                .current_dir(&repo_dir)
                .output()
                .unwrap();
            assert!(out.status.success());
        };
        run_git(&["config", "user.name", "Tester"]);
        run_git(&["config", "user.email", "tester@example.com"]);
        run_git(&["remote", "add", "origin", "https://gitlab.com/acme/demo.git"]);
        std::fs::write(repo_dir.join("a.txt"), "1\n").unwrap();
        run_git(&["add", "a.txt"]);
        run_git(&["commit", "-m", "init"]);
        std::fs::write(repo_dir.join("dirty.txt"), "2\n").unwrap();

        let paths = vec![
            repo_dir.to_string_lossy().to_string(),
            missing_dir.to_string_lossy().to_string(),
        ];
        let map = batch_inspect_repo_summaries(&paths);
        let ok_sum = map.get(&paths[0]).unwrap();
        assert!(ok_sum.exists && ok_sum.is_git);
        assert_eq!(ok_sum.branch, "main");
        assert_eq!(ok_sum.provider, Some(HostingProvider::Gitlab));
        assert_eq!(ok_sum.dirty_count, 1);
        assert_eq!(ok_sum.unpushed_commits, 1);
        assert!(ok_sum.has_remote);

        let miss_sum = map.get(&paths[1]).unwrap();
        assert!(!miss_sum.exists && !miss_sum.is_git);
        let _ = std::fs::remove_dir_all(&root);
    }
}
