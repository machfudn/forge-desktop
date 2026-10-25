use serde::Serialize;
use super::cli;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
}

pub fn list_branches(repo: &str) -> Result<Vec<BranchInfo>, String> {
    let out = cli::run(repo, &["branch", "--list", "--no-color"])?;
    let mut branches = Vec::new();

    for line in out.lines() {
        if line.len() < 2 {
            continue;
        }
        let is_current = line.starts_with('*');
        let name = line[2..].trim().to_string();
        if !name.is_empty() {
            branches.push(BranchInfo { name, is_current });
        }
    }

    Ok(branches)
}

pub fn switch_branch(repo: &str, branch_name: &str) -> Result<(), String> {
    let branch = branch_name.trim();
    if branch.is_empty() || branch.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    cli::run(repo, &["checkout", branch]).map(|_| ())
}

pub fn create_branch(repo: &str, branch_name: &str) -> Result<(), String> {
    let branch = branch_name.trim();
    if branch.is_empty() || branch.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    cli::run(repo, &["checkout", "-b", branch]).map(|_| ())
}

pub fn delete_branch(repo: &str, branch_name: &str, force: bool) -> Result<String, String> {
    let branch = branch_name.trim();
    if branch.is_empty() || branch.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    let flag = if force { "-D" } else { "-d" };
    cli::run(repo, &["branch", flag, branch])
}

pub fn rename_branch(repo: &str, old_name: &str, new_name: &str) -> Result<String, String> {
    let old_b = old_name.trim();
    let new_b = new_name.trim();
    if old_b.is_empty() || old_b.starts_with('-') || new_b.is_empty() || new_b.starts_with('-') {
        return Err("Nama branch tidak valid".into());
    }
    cli::run(repo, &["branch", "-m", old_b, new_b])
}

pub fn merge_branch(repo: &str, target_branch: &str) -> Result<String, String> {
    let target = target_branch.trim();
    if target.is_empty() || target.starts_with('-') {
        return Err("Nama branch target tidak valid".into());
    }
    cli::run(repo, &["merge", target])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_invalid_branch_names() {
        assert!(switch_branch("/tmp", "  ").is_err());
        assert!(switch_branch("/tmp", "-invalid").is_err());
        assert!(create_branch("/tmp", " ").is_err());
        assert!(create_branch("/tmp", "--force").is_err());
        assert!(delete_branch("/tmp", " ", false).is_err());
        assert!(delete_branch("/tmp", "-D", true).is_err());
        assert!(rename_branch("/tmp", " ", "new", ).is_err());
        assert!(rename_branch("/tmp", "old", "-invalid").is_err());
        assert!(merge_branch("/tmp", " ").is_err());
        assert!(merge_branch("/tmp", "-m").is_err());
    }
}
