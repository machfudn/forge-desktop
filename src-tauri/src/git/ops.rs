use super::cli;

fn validate_sha(sha: &str) -> Result<&str, String> {
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

fn validate_tag_name(name: &str) -> Result<&str, String> {
    let n = name.trim();
    if n.is_empty()
        || n.starts_with('-')
        || n.contains(' ')
        || n.contains('~')
        || n.contains('^')
        || n.contains(':')
    {
        return Err("Nama tag tidak valid".into());
    }
    Ok(n)
}

pub fn cherry_pick(repo: &str, sha: &str) -> Result<String, String> {
    let valid_sha = validate_sha(sha)?;
    cli::run(repo, &["cherry-pick", valid_sha])
}

pub fn revert_commit(repo: &str, sha: &str) -> Result<String, String> {
    let valid_sha = validate_sha(sha)?;
    cli::run(repo, &["revert", "--no-edit", valid_sha])
}

pub fn reset_to_commit(repo: &str, sha: &str, hard: bool) -> Result<String, String> {
    let valid_sha = validate_sha(sha)?;
    let mode = if hard { "--hard" } else { "--soft" };
    cli::run(repo, &["reset", mode, valid_sha])
}

pub fn rebase_branch(repo: &str, base_branch: &str) -> Result<String, String> {
    let base = base_branch.trim();
    if base.is_empty() || base.starts_with('-') {
        return Err("Nama branch dasar tidak valid".into());
    }
    cli::run(repo, &["rebase", base])
}

pub fn list_tags(repo: &str) -> Result<Vec<String>, String> {
    let out = cli::run(repo, &["tag", "-l"])?;
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

pub fn create_tag(repo: &str, tag_name: &str, target_sha: Option<&str>) -> Result<String, String> {
    let name = validate_tag_name(tag_name)?;
    let mut args = vec!["tag", name];
    let sha_val;
    if let Some(s) = target_sha {
        if !s.trim().is_empty() {
            sha_val = validate_sha(s)?;
            args.push(sha_val);
        }
    }
    cli::run(repo, &args)
}

pub fn delete_tag(repo: &str, tag_name: &str) -> Result<String, String> {
    let name = validate_tag_name(tag_name)?;
    cli::run(repo, &["tag", "-d", name])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_sha_and_tags() {
        assert!(validate_sha("abc").is_err());
        assert!(validate_sha("not-hex").is_err());
        assert!(validate_sha("a1b2c3d").is_ok());

        assert!(validate_tag_name("").is_err());
        assert!(validate_tag_name("-invalid").is_err());
        assert!(validate_tag_name("v1.0.0").is_ok());
    }
}
