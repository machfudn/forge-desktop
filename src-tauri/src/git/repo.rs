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
        if !trimmed.is_empty() {
            let gi_path = p.join(".gitignore");
            fs::write(gi_path, format!("{trimmed}\n"))
                .map_err(|e| format!("Gagal menulis .gitignore: {e}"))?;
        }
    }

    if let Some(lic) = license_content {
        let trimmed = lic.trim();
        if !trimmed.is_empty() {
            let lic_path = p.join("LICENSE");
            fs::write(lic_path, format!("{trimmed}\n"))
                .map_err(|e| format!("Gagal menulis LICENSE: {e}"))?;
        }
    }

    Ok(format!("Repositori berhasil diinisialisasi di {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_empty_init_path() {
        assert!(init_repository("   ", None, None, None).is_err());
    }
}
