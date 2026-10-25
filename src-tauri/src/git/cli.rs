use std::path::Path;
use std::process::Command;

pub fn run_raw(dir: &str, args: &[&str]) -> Result<String, String> {
    let p = Path::new(dir);
    if !p.exists() {
        return Err("Direktori tidak ditemukan".into());
    }
    let output = Command::new("git")
        .args(["-C", dir])
        .args(args)
        .output()
        .map_err(|error| format!("git tidak tersedia: {error}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if err.is_empty() {
            "Perintah git gagal".into()
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn run(repo: &str, args: &[&str]) -> Result<String, String> {
    if !Path::new(repo).join(".git").exists() {
        return Err("Folder bukan repository Git".into());
    }
    run_raw(repo, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_non_git_dir() {
        let res = run("/tmp/nonexistent-git-repo-test-12345", &["status"]);
        assert!(res.is_err());
    }
}
