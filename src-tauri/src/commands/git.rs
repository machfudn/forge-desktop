use crate::diagnostics::{self, DiagnosticReport};
use crate::git::{
    self, AheadBehind, BranchInfo, CheckRunInfo, CommitInfo, DeviceFlowInit, IssueInfo,
    PublishRepoParams, PullRequestInfo, RemoteInfo, RepoConfig, RepoStatus, TokenPollResult,
};
use crate::system;

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
pub fn commit_changes(
    repo: &str,
    summary: &str,
    description: Option<&str>,
    amend: Option<bool>,
) -> Result<String, String> {
    git::commit(repo, summary, description, amend.unwrap_or(false))
}

#[tauri::command]
pub fn undo_last_commit(repo: &str) -> Result<String, String> {
    git::undo_last_commit(repo)
}

#[tauri::command]
pub fn get_file_diff(
    repo: &str,
    file_path: &str,
    staged: bool,
    ignore_whitespace: Option<bool>,
) -> Result<String, String> {
    git::get_file_diff(repo, file_path, staged, ignore_whitespace.unwrap_or(false))
}

#[tauri::command]
pub fn add_to_gitignore(repo: &str, pattern: &str) -> Result<(), String> {
    git::add_to_gitignore(repo, pattern)
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
pub fn delete_branch(repo: &str, branch_name: &str, force: Option<bool>) -> Result<String, String> {
    git::delete_branch(repo, branch_name, force.unwrap_or(false))
}

#[tauri::command]
pub fn rename_branch(repo: &str, old_name: &str, new_name: &str) -> Result<String, String> {
    git::rename_branch(repo, old_name, new_name)
}

#[tauri::command]
pub fn merge_branch(repo: &str, target_branch: &str) -> Result<String, String> {
    git::merge_branch(repo, target_branch)
}

#[tauri::command]
pub fn rebase_branch(repo: &str, base_branch: &str) -> Result<String, String> {
    git::rebase_branch(repo, base_branch)
}

#[tauri::command]
pub fn cherry_pick(repo: &str, sha: &str) -> Result<String, String> {
    git::cherry_pick(repo, sha)
}

#[tauri::command]
pub fn revert_commit(repo: &str, sha: &str) -> Result<String, String> {
    git::revert_commit(repo, sha)
}

#[tauri::command]
pub fn reset_to_commit(repo: &str, sha: &str, hard: Option<bool>) -> Result<String, String> {
    git::reset_to_commit(repo, sha, hard.unwrap_or(false))
}

#[tauri::command]
pub fn list_tags(repo: &str) -> Result<Vec<String>, String> {
    git::list_tags(repo)
}

#[tauri::command]
pub fn create_tag(
    repo: &str,
    tag_name: &str,
    target_sha: Option<&str>,
) -> Result<String, String> {
    git::create_tag(repo, tag_name, target_sha)
}

#[tauri::command]
pub fn delete_tag(repo: &str, tag_name: &str) -> Result<String, String> {
    git::delete_tag(repo, tag_name)
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
pub fn get_branch_ahead_behind(repo: &str, branch: &str) -> Result<AheadBehind, String> {
    git::get_ahead_behind(repo, branch)
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
pub fn git_force_push(
    repo: &str,
    remote: Option<&str>,
    branch: Option<&str>,
) -> Result<String, String> {
    git::force_push(repo, remote, branch)
}

#[tauri::command]
pub fn clone_repository(url: &str, target_dir: &str) -> Result<String, String> {
    git::clone_repository(url, target_dir)
}

#[tauri::command]
pub fn init_repository(
    target_path: &str,
    default_branch: Option<&str>,
    gitignore_content: Option<&str>,
    license_content: Option<&str>,
) -> Result<String, String> {
    git::init_repository(target_path, default_branch, gitignore_content, license_content)
}

#[tauri::command]
pub fn get_repo_config(repo: &str) -> Result<RepoConfig, String> {
    git::get_repo_config(repo)
}

#[tauri::command]
pub fn set_repo_config(repo: &str, user_name: &str, user_email: &str) -> Result<(), String> {
    git::set_repo_config(repo, user_name, user_email)
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
pub fn discard_all_changes(repo: &str) -> Result<(), String> {
    git::discard_all_changes(repo)
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
pub fn git_stash_show(repo: &str, stash_ref: Option<&str>) -> Result<String, String> {
    git::stash_show(repo, stash_ref)
}

#[tauri::command]
pub fn git_stash_drop(repo: &str, stash_ref: Option<&str>) -> Result<String, String> {
    git::stash_drop(repo, stash_ref)
}

#[tauri::command]
pub fn check_repo_exists(path: &str) -> Result<bool, String> {
    Ok(git::check_repo_exists(path))
}

#[tauri::command]
pub fn get_diagnostics(repo: Option<&str>) -> Result<DiagnosticReport, String> {
    Ok(diagnostics::generate_report(repo))
}

#[tauri::command]
pub fn open_in_file_manager(path: &str) -> Result<(), String> {
    system::open_in_file_manager(path)
}

#[tauri::command]
pub fn open_in_terminal(path: &str) -> Result<(), String> {
    system::open_in_terminal(path)
}

#[tauri::command]
pub fn open_in_editor(path: &str, editor: Option<&str>) -> Result<(), String> {
    system::open_in_editor(path, editor)
}

#[tauri::command]
pub fn parse_github_repository(remote_url: &str) -> Result<Option<(String, String)>, String> {
    Ok(git::parse_github_repo(remote_url))
}

#[tauri::command]
pub fn list_github_pull_requests(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<Vec<PullRequestInfo>, String> {
    git::list_pull_requests(owner, repo, token)
}

#[tauri::command]
pub fn list_github_issues(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<Vec<IssueInfo>, String> {
    git::list_issues(owner, repo, token)
}

#[tauri::command]
pub fn checkout_github_pull_request(
    repo_path: &str,
    pr_number: u64,
    remote: Option<&str>,
) -> Result<String, String> {
    git::checkout_pull_request(repo_path, pr_number, remote)
}

#[tauri::command]
pub fn get_commit_check_runs(
    owner: &str,
    repo: &str,
    commit_sha: &str,
    token: Option<&str>,
) -> Result<Vec<CheckRunInfo>, String> {
    git::get_commit_check_runs(owner, repo, commit_sha, token)
}

#[tauri::command]
pub fn squash_commits(
    repo: &str,
    count: usize,
    new_summary: &str,
    new_description: Option<&str>,
) -> Result<String, String> {
    git::squash_commits(repo, count, new_summary, new_description)
}

#[tauri::command]
pub fn get_global_config() -> Result<RepoConfig, String> {
    git::get_global_config()
}

#[tauri::command]
pub fn set_global_config(user_name: &str, user_email: &str) -> Result<(), String> {
    git::set_global_config(user_name, user_email)
}

#[tauri::command]
pub fn get_cli_repo_arg() -> Result<Option<String>, String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let candidate = &args[1];
        if !candidate.starts_with('-') {
            let path = std::path::Path::new(candidate);
            if path.exists() && path.is_dir() {
                if let Ok(canonical) = path.canonicalize() {
                    return Ok(Some(canonical.to_string_lossy().to_string()));
                }
            }
        }
    }
    Ok(None)
}

#[tauri::command]
pub fn compare_branches(repo: &str, base: &str, target: &str) -> Result<AheadBehind, String> {
    git::compare_branches(repo, base, target)
}

#[tauri::command]
pub fn publish_repository_to_github(
    repo: &str,
    params: PublishRepoParams,
    token: Option<&str>,
) -> Result<String, String> {
    git::publish_repository_to_github(repo, params, token)
}
