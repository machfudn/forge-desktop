use std::fs;
use std::path::Path;
use serde::Serialize;
use super::cli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoConfig {
    pub user_name: String,
    pub user_email: String,
}

pub fn get_repo_config(repo: &str) -> Result<RepoConfig, String> {
    let name = cli::run(repo, &["config", "--get", "user.name"]).unwrap_or_default();
    let email = cli::run(repo, &["config", "--get", "user.email"]).unwrap_or_default();
    Ok(RepoConfig {
        user_name: name.trim().to_string(),
        user_email: email.trim().to_string(),
    })
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
    let name = cli::run_raw(".", &["config", "--global", "user.name"]).unwrap_or_default();
    let email = cli::run_raw(".", &["config", "--global", "user.email"]).unwrap_or_default();
    Ok(RepoConfig {
        user_name: name.trim().to_string(),
        user_email: email.trim().to_string(),
    })
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
}
