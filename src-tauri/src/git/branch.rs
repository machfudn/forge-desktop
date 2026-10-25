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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_invalid_branch_names() {
        assert!(switch_branch("/tmp", "  ").is_err());
        assert!(switch_branch("/tmp", "-invalid").is_err());
        assert!(create_branch("/tmp", " ").is_err());
        assert!(create_branch("/tmp", "--force").is_err());
    }
}
