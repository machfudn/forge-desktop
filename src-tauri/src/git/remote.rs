use std::path::Path;
use std::process::Command;
use serde::Serialize;
use super::cli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

pub fn validate_remote_url(url: &str) -> Result<(), String> {
    let u = url.trim();
    if u.is_empty() {
        return Err("URL remote tidak boleh kosong".into());
    }
    if u.starts_with('-')
        || u.contains('\n')
        || u.contains('\r')
        || u.contains(' ')
        || u.contains(';')
        || u.contains('&')
        || u.contains('|')
        || u.contains('`')
        || u.contains('$')
    {
        return Err("URL remote tidak valid".into());
    }
    if !(u.starts_with("https://")
        || u.starts_with("http://")
        || u.starts_with("git@")
        || u.starts_with("ssh://")
        || u.starts_with("file://")
        || Path::new(u).exists())
    {
        return Err("Protokol URL remote harus https, ssh, git@, atau path valid".into());
    }
    Ok(())
}

pub fn list_remotes(repo: &str) -> Result<Vec<RemoteInfo>, String> {
    let out = cli::run(repo, &["remote", "-v"])?;
    let mut remotes = Vec::new();

    for line in out.lines() {
        if line.ends_with("(fetch)") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                remotes.push(RemoteInfo {
                    name: parts[0].to_string(),
                    url: parts[1].to_string(),
                });
            }
        }
    }

    Ok(remotes)
}

fn validate_arg<'a>(val: &'a str, label: &str) -> Result<&'a str, String> {
    let trimmed = val.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return Err(format!("{label} tidak valid"));
    }
    if trimmed
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, ';' | '&' | '|' | '`' | '$'))
    {
        return Err(format!("{label} mengandung karakter terlarang"));
    }
    Ok(trimmed)
}

fn validate_remote_name(name: &str) -> Result<&str, String> {
    let trimmed = validate_arg(name, "Nama remote")?;
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains(':') || trimmed.contains("..") {
        return Err("Nama remote tidak valid".into());
    }
    Ok(trimmed)
}

pub fn add_remote(repo: &str, name: &str, url: &str) -> Result<String, String> {
    let rem = validate_remote_name(name)?;
    let u = url.trim();
    validate_remote_url(u)?;
    cli::run(repo, &["remote", "add", rem, u])?;
    Ok(format!("Remote '{rem}' berhasil ditambahkan"))
}

pub fn set_remote_url(repo: &str, name: &str, url: &str) -> Result<String, String> {
    let rem = validate_remote_name(name)?;
    let u = url.trim();
    validate_remote_url(u)?;
    cli::run(repo, &["remote", "set-url", rem, u])?;
    Ok(format!("URL remote '{rem}' berhasil diperbarui"))
}

pub fn remove_remote(repo: &str, name: &str) -> Result<String, String> {
    let rem = validate_remote_name(name)?;
    cli::run(repo, &["remote", "remove", rem])?;
    Ok(format!("Remote '{rem}' berhasil dihapus"))
}

fn resolve_default_remote(
    repo: &str,
    requested: Option<&str>,
    branch: Option<&str>,
) -> Result<String, String> {
    if let Some(r) = requested.filter(|s| !s.trim().is_empty()) {
        return Ok(validate_remote_name(r)?.to_string());
    }
    if let Some(b) = branch.filter(|s| !s.trim().is_empty()) {
        if let Ok(cfg_rem) = cli::run(repo, &["config", "--get", &format!("branch.{b}.remote")]) {
            let trimmed = cfg_rem.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('-') {
                return Ok(trimmed.to_string());
            }
        }
    }
    let remotes = list_remotes(repo).unwrap_or_default();
    if remotes.iter().any(|r| r.name == "origin") {
        return Ok("origin".into());
    }
    if let Some(first) = remotes.first() {
        return Ok(first.name.clone());
    }
    Ok("origin".into())
}

fn current_branch_name(repo: &str) -> Option<String> {
    cli::run(repo, &["branch", "--show-current"])
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| cli::run(repo, &["symbolic-ref", "--short", "HEAD"]).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn parse_ahead_behind_output(out: &str) -> AheadBehind {
    let parts: Vec<&str> = out.split_whitespace().collect();
    if parts.len() >= 2 {
        AheadBehind {
            ahead: parts[0].parse::<usize>().unwrap_or(0),
            behind: parts[1].parse::<usize>().unwrap_or(0),
            ..Default::default()
        }
    } else {
        AheadBehind::default()
    }
}

pub fn extract_host_from_url(url: &str) -> Option<String> {
    let u = url.trim();
    if let Some(rest) = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
        .or_else(|| u.strip_prefix("ssh://git@"))
    {
        let host_port = rest.split('/').next()?.trim();
        let host = host_port.split('@').next_back()?.split(':').next()?;
        if !host.is_empty() {
            return Some(host.to_ascii_lowercase());
        }
    } else if let Some(rest) = u.strip_prefix("git@") {
        let host = rest.split(':').next()?.trim();
        if !host.is_empty() {
            return Some(host.to_ascii_lowercase());
        }
    }
    None
}

pub fn infer_remote_host(repo: &str, remote: Option<&str>) -> Option<String> {
    let rem = remote.unwrap_or("origin").trim();
    if rem.is_empty() || rem.starts_with('-') {
        return None;
    }
    let url = cli::run(repo, &["remote", "get-url", rem]).ok()?;
    extract_host_from_url(&url)
}

pub fn fetch(
    repo: &str,
    remote: Option<&str>,
    creds: Option<(&str, &str)>,
) -> Result<String, String> {
    let rem = resolve_default_remote(repo, remote, None)?;
    cli::run_with_credentials(repo, &["fetch", &rem], creds)
}

pub fn pull(
    repo: &str,
    remote: Option<&str>,
    branch: Option<&str>,
    creds: Option<(&str, &str)>,
) -> Result<String, String> {
    let cur_branch = current_branch_name(repo);
    let rem = resolve_default_remote(repo, remote, branch.or(cur_branch.as_deref()))?;
    let mut args = vec!["pull", rem.as_str()];
    let valid_branch;
    if let Some(b) = branch.filter(|s| !s.trim().is_empty()) {
        valid_branch = validate_arg(b, "Nama branch")?;
        args.push(valid_branch);
    }
    cli::run_with_credentials(repo, &args, creds)
}

pub fn push(
    repo: &str,
    remote: Option<&str>,
    branch: Option<&str>,
    creds: Option<(&str, &str)>,
) -> Result<String, String> {
    if cli::run(repo, &["rev-parse", "--verify", "HEAD"]).is_err() {
        return Err("Belum ada komit untuk di-push. Buat komit pertama terlebih dahulu.".into());
    }
    let target_branch = match branch.filter(|s| !s.trim().is_empty()) {
        Some(b) => validate_arg(b, "Nama branch")?.to_string(),
        None => current_branch_name(repo).ok_or_else(|| {
            "Tidak dapat push dari posisi detached HEAD. Pilih atau buat branch terlebih dahulu."
                .to_string()
        })?,
    };
    let rem = resolve_default_remote(repo, remote, Some(&target_branch))?;
    let has_upstream = cli::run(
        repo,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            &format!("{target_branch}@{{upstream}}"),
        ],
    )
    .is_ok();
    if has_upstream {
        cli::run_with_credentials(repo, &["push", &rem, &target_branch], creds)
    } else {
        cli::run_with_credentials(repo, &["push", "-u", &rem, &target_branch], creds)
    }
}

pub fn force_push(
    repo: &str,
    remote: Option<&str>,
    branch: Option<&str>,
    creds: Option<(&str, &str)>,
) -> Result<String, String> {
    if cli::run(repo, &["rev-parse", "--verify", "HEAD"]).is_err() {
        return Err("Belum ada komit untuk di-push. Buat komit pertama terlebih dahulu.".into());
    }
    let target_branch = match branch.filter(|s| !s.trim().is_empty()) {
        Some(b) => validate_arg(b, "Nama branch")?.to_string(),
        None => current_branch_name(repo).ok_or_else(|| {
            "Tidak dapat push dari posisi detached HEAD. Pilih atau buat branch terlebih dahulu."
                .to_string()
        })?,
    };
    let rem = resolve_default_remote(repo, remote, Some(&target_branch))?;
    let has_upstream = cli::run(
        repo,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            &format!("{target_branch}@{{upstream}}"),
        ],
    )
    .is_ok();
    if has_upstream {
        cli::run_with_credentials(
            repo,
            &["push", "--force-with-lease", &rem, &target_branch],
            creds,
        )
    } else {
        cli::run_with_credentials(
            repo,
            &["push", "--force-with-lease", "-u", &rem, &target_branch],
            creds,
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct AheadBehind {
    pub ahead: usize,
    pub behind: usize,
    pub has_remote: bool,
    pub has_upstream: bool,
    pub upstream: Option<String>,
    pub remote_branch_exists: bool,
    pub is_unborn: bool,
    pub unpushed_commits: usize,
}

pub fn get_ahead_behind(repo: &str, branch: &str) -> Result<AheadBehind, String> {
    let has_remote = !list_remotes(repo).unwrap_or_default().is_empty();
    let is_unborn = cli::run(repo, &["rev-parse", "--verify", "HEAD"]).is_err();
    let b = branch.trim();
    if is_unborn || b.is_empty() || b.starts_with('-') {
        return Ok(AheadBehind {
            has_remote,
            is_unborn,
            ..Default::default()
        });
    }

    let upstream_query = format!("{b}@{{upstream}}");
    if let Ok(up_raw) = cli::run(
        repo,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            &upstream_query,
        ],
    ) {
        let up = up_raw.trim().to_string();
        if !up.is_empty() {
            let spec = format!("{b}...{up}");
            let counts = cli::run(repo, &["rev-list", "--left-right", "--count", &spec])
                .map(|out| parse_ahead_behind_output(&out))
                .unwrap_or_default();
            return Ok(AheadBehind {
                ahead: counts.ahead,
                behind: counts.behind,
                has_remote,
                has_upstream: true,
                upstream: Some(up),
                remote_branch_exists: true,
                is_unborn: false,
                unpushed_commits: counts.ahead,
            });
        }
    }

    let matched_remote_ref = cli::run(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            &format!("refs/remotes/*/{b}"),
        ],
    )
    .ok()
    .and_then(|out| {
        out.lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(ToString::to_string)
    });

    if let Some(rem_ref) = &matched_remote_ref {
        let spec = format!("{b}...{rem_ref}");
        let counts = cli::run(repo, &["rev-list", "--left-right", "--count", &spec])
            .map(|out| parse_ahead_behind_output(&out))
            .unwrap_or_default();
        return Ok(AheadBehind {
            ahead: counts.ahead,
            behind: counts.behind,
            has_remote,
            has_upstream: false,
            upstream: None,
            remote_branch_exists: true,
            is_unborn: false,
            unpushed_commits: counts.ahead,
        });
    }

    let unpushed_commits = if has_remote {
        cli::run(repo, &["rev-list", "--count", b, "--not", "--remotes"])
    } else {
        cli::run(repo, &["rev-list", "--count", b])
    }
    .ok()
    .and_then(|out| out.trim().parse::<usize>().ok())
    .unwrap_or(0);

    Ok(AheadBehind {
        ahead: 0,
        behind: 0,
        has_remote,
        has_upstream: false,
        upstream: None,
        remote_branch_exists: false,
        is_unborn: false,
        unpushed_commits,
    })
}

pub fn clone_repository(
    url: &str,
    target_dir: &str,
    creds: Option<(&str, &str)>,
) -> Result<String, String> {
    validate_remote_url(url)?;
    let target = target_dir.trim();
    if target.is_empty() {
        return Err("Target direktori tidak boleh kosong".into());
    }

    let mut cmd = Command::new("git");
    cmd.args(["clone", "--", url, target]);
    let _guard = cli::apply_git_askpass(&mut cmd, creds)?;
    let output = cmd
        .output()
        .map_err(|e| format!("Gagal menjalankan git clone: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if err.is_empty() {
            "Git clone gagal".into()
        } else {
            err
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn sync_fork(
    repo: &str,
    upstream_remote: Option<&str>,
    branch: Option<&str>,
    creds: Option<(&str, &str)>,
) -> Result<String, String> {
    let remote = upstream_remote.unwrap_or("upstream").trim();
    let b = branch.unwrap_or("main").trim();
    if remote.is_empty() || remote.starts_with('-') || b.is_empty() || b.starts_with('-') {
        return Err("Parameter remote atau branch tidak valid".into());
    }
    cli::run_with_credentials(repo, &["fetch", remote, b], creds)?;
    let upstream_ref = format!("{remote}/{b}");
    cli::run(repo, &["merge", "--ff-only", &upstream_ref]).or_else(|_| {
        cli::run(
            repo,
            &["merge", &upstream_ref, "-m", &format!("Merge upstream {upstream_ref}")],
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_remote_urls() {
        assert!(validate_remote_url("https://github.com/foo/bar.git").is_ok());
        assert!(validate_remote_url("git@github.com:foo/bar.git").is_ok());
        assert!(validate_remote_url("ssh://git@github.com/foo/bar.git").is_ok());
    }

    #[test]
    fn test_invalid_remote_urls() {
        assert!(validate_remote_url("  ").is_err());
        assert!(validate_remote_url("-badurl").is_err());
        assert!(validate_remote_url("ftp://foo/bar").is_err());
    }

    #[test]
    fn test_extract_host_from_url() {
        assert_eq!(
            extract_host_from_url("https://git.kantor.id/org/repo.git"),
            Some("git.kantor.id".into())
        );
        assert_eq!(
            extract_host_from_url("git@github.com:octocat/hello.git"),
            Some("github.com".into())
        );
    }

    #[test]
    fn ahead_behind_tracks_unborn_no_remote_unpushed_and_synced_branches() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("forge-remote-push-status-{nanos}"));
        let bare = root.join("bare.git");
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
        run_git(&local, &["init", "-b", "main"]);
        run_git(&local, &["config", "user.name", "Tester"]);
        run_git(&local, &["config", "user.email", "tester@example.com"]);

        let local_str = local.to_str().unwrap();
        let unborn = get_ahead_behind(local_str, "main").unwrap();
        assert!(unborn.is_unborn);
        assert!(!unborn.has_remote);
        assert!(!unborn.has_upstream);
        assert!(push(local_str, None, None, None).is_err());

        std::fs::write(local.join("a.txt"), "one\n").unwrap();
        run_git(&local, &["add", "a.txt"]);
        run_git(&local, &["commit", "-m", "initial"]);
        let no_remote = get_ahead_behind(local_str, "main").unwrap();
        assert!(!no_remote.is_unborn);
        assert!(!no_remote.has_remote);
        assert!(!no_remote.has_upstream);
        assert_eq!(no_remote.unpushed_commits, 1);

        add_remote(local_str, "origin", bare.to_str().unwrap()).unwrap();
        let before_push = get_ahead_behind(local_str, "main").unwrap();
        assert!(before_push.has_remote);
        assert!(!before_push.has_upstream);
        assert!(!before_push.remote_branch_exists);
        assert_eq!(before_push.unpushed_commits, 1);

        push(local_str, None, None, None).unwrap();
        let after_push = get_ahead_behind(local_str, "main").unwrap();
        assert!(after_push.has_remote);
        assert!(after_push.has_upstream);
        assert_eq!(after_push.upstream.as_deref(), Some("origin/main"));
        assert!(after_push.remote_branch_exists);
        assert_eq!(after_push.ahead, 0);
        assert_eq!(after_push.unpushed_commits, 0);

        run_git(&local, &["checkout", "-b", "feat/local-only"]);
        let feat_status = get_ahead_behind(local_str, "feat/local-only").unwrap();
        assert!(feat_status.has_remote);
        assert!(!feat_status.has_upstream);
        assert!(!feat_status.remote_branch_exists);

        set_remote_url(local_str, "origin", bare.to_str().unwrap()).unwrap();
        assert!(remove_remote(local_str, "origin").is_ok());
        assert!(list_remotes(local_str).unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }
}
