use crate::diagnostics::{self, DiagnosticReport};
use crate::git::{
    self, BranchInfo, CommitInfo, DeviceFlowInit, RemoteInfo, RepoStatus, TokenPollResult,
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

#[tauri::command]
pub fn abort_merge(repo: &str) -> Result<(), String> {
    git::abort_merge(repo)
}

#[tauri::command]
pub fn abort_rebase(repo: &str) -> Result<(), String> {
    git::abort_rebase(repo)
}

#[tauri::command]
pub fn list_remotes(repo: &str) -> Result<Vec<RemoteInfo>, String> {
    git::list_remotes(repo)
}

#[tauri::command]
pub fn git_fetch(repo: &str, remote: Option<&str>) -> Result<String, String> {
    git::fetch(repo, remote)
}

#[tauri::command]
pub fn git_pull(repo: &str, remote: Option<&str>, branch: Option<&str>) -> Result<String, String> {
    git::pull(repo, remote, branch)
}

#[tauri::command]
pub fn git_push(repo: &str, remote: Option<&str>, branch: Option<&str>) -> Result<String, String> {
    git::push(repo, remote, branch)
}

#[tauri::command]
pub fn clone_repository(url: &str, target_dir: &str) -> Result<String, String> {
    git::clone_repository(url, target_dir)
}

#[tauri::command]
pub fn github_start_device_flow(client_id: &str) -> Result<DeviceFlowInit, String> {
    git::start_device_flow(client_id)
}

#[tauri::command]
pub fn github_poll_token(client_id: &str, device_code: &str) -> Result<TokenPollResult, String> {
    git::poll_device_token(client_id, device_code)
}

#[tauri::command]
pub fn configure_git_credentials(username: &str, token: &str) -> Result<(), String> {
    git::configure_git_credentials(username, token)
}

#[tauri::command]
pub fn discard_file_changes(repo: &str, file_path: &str) -> Result<(), String> {
    git::discard_changes(repo, file_path)
}

#[tauri::command]
pub fn git_stash_save(repo: &str, message: Option<&str>) -> Result<String, String> {
    git::stash_save(repo, message)
}

#[tauri::command]
pub fn git_stash_pop(repo: &str) -> Result<String, String> {
    git::stash_pop(repo)
}

#[tauri::command]
pub fn git_list_stashes(repo: &str) -> Result<Vec<String>, String> {
    git::list_stashes(repo)
}

#[tauri::command]
pub fn get_diagnostics(repo: Option<&str>) -> Result<DiagnosticReport, String> {
    Ok(diagnostics::generate_report(repo))
}
