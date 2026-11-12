use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::Serialize;
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
}
