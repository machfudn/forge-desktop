use crate::git::{
    self, BranchInfo, CommitInfo, RepoStatus,
};

#[tauri::command]
pub fn inspect_repository(repo: &str) -> Result<RepoStatus, String> {
    git::inspect(repo)
}

#[tauri::command]
pub fn stage_file(repo: &str, file_path: &str) -> Result<(), String> {
    git::stage_file(repo, file_path)
}

#[tauri::command]
pub fn unstage_file(repo: &str, file_path: &str) -> Result<(), String> {
    git::unstage_file(repo, file_path)
}

#[tauri::command]
pub fn stage_all(repo: &str) -> Result<(), String> {
    git::stage_all(repo)
}

#[tauri::command]
pub fn unstage_all(repo: &str) -> Result<(), String> {
    git::unstage_all(repo)
}

#[tauri::command]
pub fn commit_changes(repo: &str, message: &str) -> Result<String, String> {
    git::commit(repo, message)
}

#[tauri::command]
pub fn get_file_diff(repo: &str, file_path: &str, staged: bool) -> Result<String, String> {
    git::get_file_diff(repo, file_path, staged)
}

#[tauri::command]
pub fn get_commit_history(repo: &str, limit: Option<usize>) -> Result<Vec<CommitInfo>, String> {
    let lim = limit.unwrap_or(50);
    git::get_history(repo, lim)
}

#[tauri::command]
pub fn list_branches(repo: &str) -> Result<Vec<BranchInfo>, String> {
    git::list_branches(repo)
}

#[tauri::command]
pub fn switch_branch(repo: &str, branch_name: &str) -> Result<(), String> {
    git::switch_branch(repo, branch_name)
}

#[tauri::command]
pub fn create_branch(repo: &str, branch_name: &str) -> Result<(), String> {
    git::create_branch(repo, branch_name)
}
