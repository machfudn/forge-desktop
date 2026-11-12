use crate::diagnostics::{self, DiagnosticReport};
use crate::git::{
    self, AccountHealth, AccountManager, AccountOverview, AccountProfile, AheadBehind,
    BranchInfo, CheckRunInfo, CommitDetail, CommitInfo, DetectedSession, DeviceFlowInit,
    DeviceFlowPollOutcome, ImageDiffData, IssueInfo, LocalSessionSource, ParsedGitHubRepo,
    PublishRepoParams, PullRequestInfo, RebaseStep, RemoteInfo, RepoConfig, RepoStatus,
    GITHUB_PUBLIC_HOST,
};
use crate::system;
use crate::updater::{self, AppRuntimeInfo, AppUpdateInfo, UpdateApplyResult, UpdateChannel};
use tauri::State;

fn resolve_remote_creds(
    accounts: &AccountManager,
    repo: &str,
    remote: Option<&str>,
) -> Option<(String, String)> {
    let host = git::infer_remote_host(repo, remote)?;
    accounts
        .resolve_account_for_repo(Some(repo), &host)
        .map(|(p, t)| (p.login, t))
}

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
pub fn get_commit_detail(repo: &str, sha: &str) -> Result<CommitDetail, String> {
    git::get_commit_detail(repo, sha)
}

#[tauri::command]
pub fn get_commit_file_diff(
    repo: &str,
    sha: &str,
    file_path: &str,
    old_path: Option<&str>,
    ignore_whitespace: Option<bool>,
) -> Result<String, String> {
    git::get_commit_file_diff(
        repo,
        sha,
        file_path,
        old_path,
        ignore_whitespace.unwrap_or(false),
    )
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
pub fn add_remote(repo: &str, name: &str, url: &str) -> Result<String, String> {
    git::add_remote(repo, name, url)
}

#[tauri::command]
pub fn set_remote_url(repo: &str, name: &str, url: &str) -> Result<String, String> {
    git::set_remote_url(repo, name, url)
}

#[tauri::command]
pub fn remove_remote(repo: &str, name: &str) -> Result<String, String> {
    git::remove_remote(repo, name)
}

#[tauri::command]
pub fn get_branch_ahead_behind(repo: &str, branch: &str) -> Result<AheadBehind, String> {
    git::get_ahead_behind(repo, branch)
}

#[tauri::command]
pub fn git_fetch(
    repo: &str,
    remote: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let creds = resolve_remote_creds(&accounts, repo, remote);
    git::fetch(
        repo,
        remote,
        creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
    )
}

#[tauri::command]
pub fn git_pull(
    repo: &str,
    remote: Option<&str>,
    branch: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let creds = resolve_remote_creds(&accounts, repo, remote);
    git::pull(
        repo,
        remote,
        branch,
        creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
    )
}

#[tauri::command]
pub fn git_push(
    repo: &str,
    remote: Option<&str>,
    branch: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let creds = resolve_remote_creds(&accounts, repo, remote);
    git::push(
        repo,
        remote,
        branch,
        creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
    )
}

#[tauri::command]
pub fn git_force_push(
    repo: &str,
    remote: Option<&str>,
    branch: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let creds = resolve_remote_creds(&accounts, repo, remote);
    git::force_push(
        repo,
        remote,
        branch,
        creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
    )
}

#[tauri::command]
pub fn clone_repository(
    url: &str,
    target_dir: &str,
    account_id: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let creds = git::extract_host_from_url(url)
        .and_then(|host| accounts.resolve_account(account_id, None, &host))
        .map(|(p, t)| (p.login, t));
    git::clone_repository(
        url,
        target_dir,
        creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
    )
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
pub fn github_start_device_flow(
    host: Option<&str>,
    client_id: &str,
) -> Result<DeviceFlowInit, String> {
    git::start_device_flow(host, client_id)
}

#[tauri::command]
pub fn list_github_accounts(
    repo: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    Ok(accounts.list_overview(repo))
}

#[tauri::command]
pub fn verify_and_connect_pat(
    host: Option<&str>,
    token: &str,
    label: Option<&str>,
    allow_memory_fallback: Option<bool>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountProfile, String> {
    git::verify_and_connect_pat(
        &accounts,
        host,
        token,
        label,
        allow_memory_fallback.unwrap_or(false),
    )
}

#[tauri::command]
pub fn poll_and_connect_device_flow(
    host: Option<&str>,
    client_id: &str,
    device_code: &str,
    label: Option<&str>,
    allow_memory_fallback: Option<bool>,
    accounts: State<'_, AccountManager>,
) -> Result<DeviceFlowPollOutcome, String> {
    git::poll_and_connect_device_flow(
        &accounts,
        host,
        client_id,
        device_code,
        label,
        allow_memory_fallback.unwrap_or(false),
    )
}

#[tauri::command]
pub fn detect_local_github_sessions(host: Option<&str>) -> Result<Vec<DetectedSession>, String> {
    git::detect_local_github_sessions(host.unwrap_or(GITHUB_PUBLIC_HOST))
}

#[tauri::command]
pub fn import_local_github_session(
    host: &str,
    source: LocalSessionSource,
    label: Option<&str>,
    allow_memory_fallback: Option<bool>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountProfile, String> {
    git::import_local_github_session(
        &accounts,
        host,
        &source,
        label,
        allow_memory_fallback.unwrap_or(false),
    )
}

#[tauri::command]
pub fn bind_account_to_repo(
    repo: &str,
    account_id: &str,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    accounts.bind_repo(repo, account_id)
}

#[tauri::command]
pub fn set_default_github_account(
    account_id: &str,
    repo: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    accounts.set_default(account_id, repo)
}

#[tauri::command]
pub fn remove_github_account(
    account_id: &str,
    repo: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    accounts.remove_account(account_id, repo)
}

#[tauri::command]
pub fn update_github_account_label(
    account_id: &str,
    label: Option<&str>,
    repo: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    accounts.update_label(account_id, label, repo)
}

#[tauri::command]
pub fn check_github_account_status(
    account_id: &str,
    accounts: State<'_, AccountManager>,
) -> Result<AccountHealth, String> {
    git::check_account_health(&accounts, account_id)
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
pub fn get_repo_path_state(path: &str) -> Result<String, String> {
    Ok(git::get_repo_path_state(path))
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
pub fn parse_github_repository(
    remote_url: &str,
    accounts: State<'_, AccountManager>,
) -> Result<Option<ParsedGitHubRepo>, String> {
    let hosts = accounts.registered_hosts();
    Ok(git::parse_github_repo_with_hosts(remote_url, &hosts))
}

#[tauri::command]
pub fn list_github_pull_requests(
    owner: &str,
    repo: &str,
    host: Option<&str>,
    repo_path: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<Vec<PullRequestInfo>, String> {
    let target_host = host.unwrap_or(GITHUB_PUBLIC_HOST);
    let token = accounts
        .resolve_account_for_repo(repo_path, target_host)
        .map(|(_, t)| t);
    git::list_pull_requests(target_host, owner, repo, token.as_deref())
}

#[tauri::command]
pub fn list_github_issues(
    owner: &str,
    repo: &str,
    host: Option<&str>,
    repo_path: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<Vec<IssueInfo>, String> {
    let target_host = host.unwrap_or(GITHUB_PUBLIC_HOST);
    let token = accounts
        .resolve_account_for_repo(repo_path, target_host)
        .map(|(_, t)| t);
    git::list_issues(target_host, owner, repo, token.as_deref())
}

#[tauri::command]
pub fn checkout_github_pull_request(
    repo_path: &str,
    pr_number: u64,
    remote: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let creds = resolve_remote_creds(&accounts, repo_path, remote);
    git::checkout_pull_request(
        repo_path,
        pr_number,
        remote,
        creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
    )
}

#[tauri::command]
pub fn get_commit_check_runs(
    owner: &str,
    repo: &str,
    commit_sha: &str,
    host: Option<&str>,
    repo_path: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<Vec<CheckRunInfo>, String> {
    let target_host = host.unwrap_or(GITHUB_PUBLIC_HOST);
    let token = accounts
        .resolve_account_for_repo(repo_path, target_host)
        .map(|(_, t)| t);
    git::get_commit_check_runs(target_host, owner, repo, commit_sha, token.as_deref())
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
    account_id: Option<&str>,
    host: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    git::publish_repository_to_github(&accounts, repo, params, account_id, host)
}

#[tauri::command]
pub fn sync_fork(
    repo: &str,
    upstream_remote: Option<&str>,
    branch: Option<&str>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let rem = upstream_remote.unwrap_or("upstream");
    let creds = resolve_remote_creds(&accounts, repo, Some(rem));
    git::sync_fork(
        repo,
        upstream_remote,
        branch,
        creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
    )
}

#[tauri::command]
pub fn get_image_diff(repo: &str, file_path: &str) -> Result<ImageDiffData, String> {
    git::get_image_diff(repo, file_path)
}

#[tauri::command]
pub fn get_app_runtime_info() -> AppRuntimeInfo {
    updater::get_app_runtime_info()
}

#[tauri::command]
pub async fn check_app_update(
    channel: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<AppUpdateInfo, String> {
    let token = accounts
        .resolve_account_for_repo(None, GITHUB_PUBLIC_HOST)
        .map(|(_, t)| t);
    let ch = UpdateChannel::parse(channel.as_deref());
    tauri::async_runtime::spawn_blocking(move || updater::check_app_update(ch, token.as_deref()))
        .await
        .map_err(|e| format!("Gagal memeriksa pembaruan: {e}"))?
}

#[tauri::command]
pub async fn install_app_update(
    channel: Option<String>,
    app: tauri::AppHandle,
    accounts: State<'_, AccountManager>,
) -> Result<UpdateApplyResult, String> {
    let token = accounts
        .resolve_account_for_repo(None, GITHUB_PUBLIC_HOST)
        .map(|(_, t)| t);
    let ch = UpdateChannel::parse(channel.as_deref());
    let res = tauri::async_runtime::spawn_blocking(move || {
        updater::download_and_install_update(ch, token.as_deref())
    })
    .await
    .map_err(|e| format!("Gagal menjalankan pembaruan: {e}"))??;

    if res.restart_pending {
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(350));
            handle.exit(0);
        });
    }
    Ok(res)
}

#[tauri::command]
pub fn stage_hunk(repo: &str, patch_text: &str, reverse: bool) -> Result<(), String> {
    git::stage_hunk(repo, patch_text, reverse)
}

#[tauri::command]
pub fn interactive_rebase(
    repo: &str,
    base_sha: &str,
    steps: Vec<RebaseStep>,
) -> Result<String, String> {
    git::interactive_rebase(repo, base_sha, steps)
}
