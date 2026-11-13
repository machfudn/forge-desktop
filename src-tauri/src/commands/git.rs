use std::collections::HashMap;
use std::sync::Arc;
use crate::ai::{AiConfigPublic, AiManager, AiTaskResult, AiTestResult, SaveAiConfigInput};
use crate::diagnostics::{self, DiagnosticReport};
use crate::git::{
    self, AccountHealth, AccountManager, AccountOverview, AccountProfile, AheadBehind,
    BlameLine, BranchInfo, CheckRunInfo, CommitDetail, CommitInfo, ConflictFileData,
    DetectedSession, DeviceFlowInit, DeviceFlowPollOutcome, HostingProvider, ImageDiffData,
    IssueInfo, LocalSessionSource, ParsedGitHubRepo, ParsedHostingRepo, PublishRepoParams,
    PullRequestInfo, RebaseStep, ReflogEntry, RemoteInfo, RepoConfig, RepoQuickSummary, RepoStatus,
    SubmoduleInfo, WorktreeInfo, GITHUB_PUBLIC_HOST,
};
use crate::system;
use crate::terminal::{TerminalExecResult, TerminalManager};
use crate::updater::{self, AppRuntimeInfo, AppUpdateInfo, UpdateApplyResult, UpdateChannel};
use tauri::{AppHandle, State};

async fn run_blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("Tugas latar belakang gagal dijalankan: {e}"))?
}

fn normalize_git_http_username(host: &str, login: &str) -> String {
    let trimmed = login.trim();
    if (host == "bitbucket.org" || host.contains("bitbucket"))
        && (trimmed.is_empty() || trimmed == "bitbucket-user")
    {
        return "x-token-auth".to_string();
    }
    if trimmed.is_empty() {
        return "oauth2".to_string();
    }
    trimmed.to_string()
}

fn resolve_creds_for_url(
    accounts: &AccountManager,
    repo: Option<&str>,
    account_id: Option<&str>,
    url: &str,
) -> Option<(String, String)> {
    if !git::is_http_remote_url(url) {
        return None;
    }
    let host = git::extract_host_from_url(url)?;
    accounts
        .resolve_account(account_id, repo, &host)
        .map(|(p, t)| (normalize_git_http_username(&host, &p.login), t))
}

fn resolve_remote_creds(
    accounts: &AccountManager,
    repo: &str,
    remote: Option<&str>,
) -> Option<(String, String)> {
    let rem = remote.unwrap_or("origin").trim();
    if rem.is_empty() || rem.starts_with('-') {
        return None;
    }
    let url = git::cli::run(repo, &["remote", "get-url", rem]).ok()?;
    resolve_creds_for_url(accounts, Some(repo), None, &url)
}

#[tauri::command]
pub async fn inspect_repository(repo: String) -> Result<RepoStatus, String> {
    run_blocking(move || git::inspect(&repo)).await
}

#[tauri::command]
pub async fn stage_file(repo: String, file_path: String) -> Result<(), String> {
    run_blocking(move || git::stage_file(&repo, &file_path)).await
}

#[tauri::command]
pub async fn unstage_file(repo: String, file_path: String) -> Result<(), String> {
    run_blocking(move || git::unstage_file(&repo, &file_path)).await
}

#[tauri::command]
pub async fn stage_all(repo: String) -> Result<(), String> {
    run_blocking(move || git::stage_all(&repo)).await
}

#[tauri::command]
pub async fn unstage_all(repo: String) -> Result<(), String> {
    run_blocking(move || git::unstage_all(&repo)).await
}

#[tauri::command]
pub async fn commit_changes(
    repo: String,
    summary: String,
    description: Option<String>,
    amend: Option<bool>,
) -> Result<String, String> {
    run_blocking(move || {
        git::commit(&repo, &summary, description.as_deref(), amend.unwrap_or(false))
    })
    .await
}

#[tauri::command]
pub async fn undo_last_commit(repo: String) -> Result<String, String> {
    run_blocking(move || git::undo_last_commit(&repo)).await
}

#[tauri::command]
pub async fn get_file_diff(
    repo: String,
    file_path: String,
    staged: bool,
    ignore_whitespace: Option<bool>,
) -> Result<String, String> {
    run_blocking(move || {
        git::get_file_diff(
            &repo,
            &file_path,
            staged,
            ignore_whitespace.unwrap_or(false),
        )
    })
    .await
}

#[tauri::command]
pub async fn add_to_gitignore(repo: String, pattern: String) -> Result<(), String> {
    run_blocking(move || git::add_to_gitignore(&repo, &pattern)).await
}

#[tauri::command]
pub async fn get_commit_history(
    repo: String,
    limit: Option<usize>,
    all_branches: Option<bool>,
) -> Result<Vec<CommitInfo>, String> {
    let lim = limit.unwrap_or(100);
    let all = all_branches.unwrap_or(true);
    run_blocking(move || git::get_history_with_options(&repo, lim, all)).await
}

#[tauri::command]
pub async fn get_commit_detail(repo: String, sha: String) -> Result<CommitDetail, String> {
    run_blocking(move || git::get_commit_detail(&repo, &sha)).await
}

#[tauri::command]
pub async fn get_commit_file_diff(
    repo: String,
    sha: String,
    file_path: String,
    old_path: Option<String>,
    ignore_whitespace: Option<bool>,
) -> Result<String, String> {
    run_blocking(move || {
        git::get_commit_file_diff(
            &repo,
            &sha,
            &file_path,
            old_path.as_deref(),
            ignore_whitespace.unwrap_or(false),
        )
    })
    .await
}

#[tauri::command]
pub async fn list_branches(repo: String) -> Result<Vec<BranchInfo>, String> {
    run_blocking(move || git::list_branches(&repo)).await
}

#[tauri::command]
pub async fn switch_branch(repo: String, branch_name: String) -> Result<(), String> {
    run_blocking(move || git::switch_branch(&repo, &branch_name)).await
}

#[tauri::command]
pub async fn create_branch(repo: String, branch_name: String) -> Result<(), String> {
    run_blocking(move || git::create_branch(&repo, &branch_name)).await
}

#[tauri::command]
pub async fn delete_branch(
    repo: String,
    branch_name: String,
    force: Option<bool>,
) -> Result<String, String> {
    run_blocking(move || git::delete_branch(&repo, &branch_name, force.unwrap_or(false))).await
}

#[tauri::command]
pub async fn rename_branch(
    repo: String,
    old_name: String,
    new_name: String,
) -> Result<String, String> {
    run_blocking(move || git::rename_branch(&repo, &old_name, &new_name)).await
}

#[tauri::command]
pub async fn merge_branch(repo: String, target_branch: String) -> Result<String, String> {
    run_blocking(move || git::merge_branch(&repo, &target_branch)).await
}

#[tauri::command]
pub async fn rebase_branch(repo: String, base_branch: String) -> Result<String, String> {
    run_blocking(move || git::rebase_branch(&repo, &base_branch)).await
}

#[tauri::command]
pub async fn cherry_pick(repo: String, sha: String) -> Result<String, String> {
    run_blocking(move || git::cherry_pick(&repo, &sha)).await
}

#[tauri::command]
pub async fn revert_commit(repo: String, sha: String) -> Result<String, String> {
    run_blocking(move || git::revert_commit(&repo, &sha)).await
}

#[tauri::command]
pub async fn reset_to_commit(
    repo: String,
    sha: String,
    hard: Option<bool>,
) -> Result<String, String> {
    run_blocking(move || git::reset_to_commit(&repo, &sha, hard.unwrap_or(false))).await
}

#[tauri::command]
pub async fn list_tags(repo: String) -> Result<Vec<String>, String> {
    run_blocking(move || git::list_tags(&repo)).await
}

#[tauri::command]
pub async fn create_tag(
    repo: String,
    tag_name: String,
    target_sha: Option<String>,
) -> Result<String, String> {
    run_blocking(move || git::create_tag(&repo, &tag_name, target_sha.as_deref())).await
}

#[tauri::command]
pub async fn delete_tag(repo: String, tag_name: String) -> Result<String, String> {
    run_blocking(move || git::delete_tag(&repo, &tag_name)).await
}

#[tauri::command]
pub async fn abort_merge(repo: String) -> Result<(), String> {
    run_blocking(move || git::abort_merge(&repo)).await
}

#[tauri::command]
pub async fn abort_rebase(repo: String) -> Result<(), String> {
    run_blocking(move || git::abort_rebase(&repo)).await
}

#[tauri::command]
pub async fn list_remotes(repo: String) -> Result<Vec<RemoteInfo>, String> {
    run_blocking(move || git::list_remotes(&repo)).await
}

#[tauri::command]
pub async fn add_remote(repo: String, name: String, url: String) -> Result<String, String> {
    run_blocking(move || git::add_remote(&repo, &name, &url)).await
}

#[tauri::command]
pub async fn set_remote_url(repo: String, name: String, url: String) -> Result<String, String> {
    run_blocking(move || git::set_remote_url(&repo, &name, &url)).await
}

#[tauri::command]
pub async fn remove_remote(repo: String, name: String) -> Result<String, String> {
    run_blocking(move || git::remove_remote(&repo, &name)).await
}

#[tauri::command]
pub async fn get_branch_ahead_behind(
    repo: String,
    branch: String,
) -> Result<AheadBehind, String> {
    run_blocking(move || git::get_ahead_behind(&repo, &branch)).await
}

#[tauri::command]
pub async fn git_fetch(
    repo: String,
    remote: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::fetch_with(&repo, remote.as_deref(), |url| {
            resolve_creds_for_url(&mgr, Some(&repo), None, url)
        })
    })
    .await
}

#[tauri::command]
pub async fn fetch_all_remotes(
    repo: String,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::fetch_all_with(&repo, |url| {
            resolve_creds_for_url(&mgr, Some(&repo), None, url)
        })
    })
    .await
}

#[tauri::command]
pub async fn git_pull(
    repo: String,
    remote: Option<String>,
    branch: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::pull_with(&repo, remote.as_deref(), branch.as_deref(), |url| {
            resolve_creds_for_url(&mgr, Some(&repo), None, url)
        })
    })
    .await
}

#[tauri::command]
pub async fn git_push(
    repo: String,
    remote: Option<String>,
    branch: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::push_with(&repo, remote.as_deref(), branch.as_deref(), |url| {
            resolve_creds_for_url(&mgr, Some(&repo), None, url)
        })
    })
    .await
}

#[tauri::command]
pub async fn git_force_push(
    repo: String,
    remote: Option<String>,
    branch: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::force_push_with(&repo, remote.as_deref(), branch.as_deref(), |url| {
            resolve_creds_for_url(&mgr, Some(&repo), None, url)
        })
    })
    .await
}

#[tauri::command]
pub async fn clone_repository(
    url: String,
    target_dir: String,
    account_id: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        let creds = resolve_creds_for_url(&mgr, None, account_id.as_deref(), &url);
        git::clone_repository(
            &url,
            &target_dir,
            creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
        )
    })
    .await
}

#[tauri::command]
pub async fn init_repository(
    target_path: String,
    default_branch: Option<String>,
    gitignore_content: Option<String>,
    license_content: Option<String>,
) -> Result<String, String> {
    run_blocking(move || {
        git::init_repository(
            &target_path,
            default_branch.as_deref(),
            gitignore_content.as_deref(),
            license_content.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub async fn get_repo_config(repo: String) -> Result<RepoConfig, String> {
    run_blocking(move || git::get_repo_config(&repo)).await
}

#[tauri::command]
pub async fn set_repo_config(
    repo: String,
    user_name: String,
    user_email: String,
) -> Result<(), String> {
    run_blocking(move || git::set_repo_config(&repo, &user_name, &user_email)).await
}

#[tauri::command]
pub async fn github_start_device_flow(
    host: Option<String>,
    client_id: String,
) -> Result<DeviceFlowInit, String> {
    run_blocking(move || git::start_device_flow(host.as_deref(), &client_id)).await
}

#[tauri::command]
pub async fn list_github_accounts(
    repo: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || Ok(mgr.list_overview(repo.as_deref()))).await
}

#[tauri::command]
pub async fn verify_and_connect_pat(
    host: Option<String>,
    token: String,
    label: Option<String>,
    allow_memory_fallback: Option<bool>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountProfile, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::verify_and_connect_pat(
            &mgr,
            host.as_deref(),
            &token,
            label.as_deref(),
            allow_memory_fallback.unwrap_or(false),
        )
    })
    .await
}

#[tauri::command]
pub async fn poll_and_connect_device_flow(
    host: Option<String>,
    client_id: String,
    device_code: String,
    label: Option<String>,
    allow_memory_fallback: Option<bool>,
    accounts: State<'_, AccountManager>,
) -> Result<DeviceFlowPollOutcome, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::poll_and_connect_device_flow(
            &mgr,
            host.as_deref(),
            &client_id,
            &device_code,
            label.as_deref(),
            allow_memory_fallback.unwrap_or(false),
        )
    })
    .await
}

#[tauri::command]
pub async fn detect_local_github_sessions(
    host: Option<String>,
) -> Result<Vec<DetectedSession>, String> {
    run_blocking(move || {
        git::detect_local_github_sessions(host.as_deref().unwrap_or(GITHUB_PUBLIC_HOST))
    })
    .await
}

#[tauri::command]
pub async fn import_local_github_session(
    host: String,
    source: LocalSessionSource,
    label: Option<String>,
    allow_memory_fallback: Option<bool>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountProfile, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::import_local_github_session(
            &mgr,
            &host,
            &source,
            label.as_deref(),
            allow_memory_fallback.unwrap_or(false),
        )
    })
    .await
}

#[tauri::command]
pub async fn bind_account_to_repo(
    repo: String,
    account_id: String,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || mgr.bind_repo(&repo, &account_id)).await
}

#[tauri::command]
pub async fn set_default_github_account(
    account_id: String,
    repo: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || mgr.set_default(&account_id, repo.as_deref())).await
}

#[tauri::command]
pub async fn remove_github_account(
    account_id: String,
    repo: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || mgr.remove_account(&account_id, repo.as_deref())).await
}

#[tauri::command]
pub async fn update_github_account_label(
    account_id: String,
    label: Option<String>,
    repo: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountOverview, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || mgr.update_label(&account_id, label.as_deref(), repo.as_deref())).await
}

#[tauri::command]
pub async fn check_github_account_status(
    account_id: String,
    accounts: State<'_, AccountManager>,
) -> Result<AccountHealth, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || git::check_account_health(&mgr, &account_id)).await
}

#[tauri::command]
pub async fn discard_file_changes(repo: String, file_path: String) -> Result<(), String> {
    run_blocking(move || git::discard_changes(&repo, &file_path)).await
}

#[tauri::command]
pub async fn discard_all_changes(repo: String) -> Result<(), String> {
    run_blocking(move || git::discard_all_changes(&repo)).await
}

#[tauri::command]
pub async fn git_stash_save(repo: String, message: Option<String>) -> Result<String, String> {
    run_blocking(move || git::stash_save(&repo, message.as_deref())).await
}

#[tauri::command]
pub async fn git_stash_pop(repo: String) -> Result<String, String> {
    run_blocking(move || git::stash_pop(&repo)).await
}

#[tauri::command]
pub async fn git_list_stashes(repo: String) -> Result<Vec<String>, String> {
    run_blocking(move || git::list_stashes(&repo)).await
}

#[tauri::command]
pub async fn git_stash_show(repo: String, stash_ref: Option<String>) -> Result<String, String> {
    run_blocking(move || git::stash_show(&repo, stash_ref.as_deref())).await
}

#[tauri::command]
pub async fn git_stash_drop(repo: String, stash_ref: Option<String>) -> Result<String, String> {
    run_blocking(move || git::stash_drop(&repo, stash_ref.as_deref())).await
}

#[tauri::command]
pub async fn check_repo_exists(path: String) -> Result<bool, String> {
    run_blocking(move || Ok(git::check_repo_exists(&path))).await
}

#[tauri::command]
pub async fn check_repos_exists(paths: Vec<String>) -> Result<HashMap<String, bool>, String> {
    run_blocking(move || Ok(git::check_repos_exists(&paths))).await
}

#[tauri::command]
pub async fn batch_inspect_repo_summaries(
    paths: Vec<String>,
) -> Result<HashMap<String, RepoQuickSummary>, String> {
    run_blocking(move || Ok(git::batch_inspect_repo_summaries(&paths))).await
}

#[tauri::command]
pub async fn get_repo_path_state(path: String) -> Result<String, String> {
    run_blocking(move || Ok(git::get_repo_path_state(&path))).await
}

#[tauri::command]
pub async fn get_diagnostics(repo: Option<String>) -> Result<DiagnosticReport, String> {
    run_blocking(move || Ok(diagnostics::generate_report(repo.as_deref()))).await
}

#[tauri::command]
pub async fn open_in_file_manager(path: String) -> Result<(), String> {
    run_blocking(move || system::open_in_file_manager(&path)).await
}

#[tauri::command]
pub async fn open_in_terminal(path: String) -> Result<(), String> {
    run_blocking(move || system::open_in_terminal(&path)).await
}

#[tauri::command]
pub async fn open_in_editor(path: String, editor: Option<String>) -> Result<(), String> {
    run_blocking(move || system::open_in_editor(&path, editor.as_deref())).await
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
pub async fn list_github_pull_requests(
    owner: String,
    repo: String,
    host: Option<String>,
    repo_path: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<Vec<PullRequestInfo>, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        let target_host = host.as_deref().unwrap_or(GITHUB_PUBLIC_HOST);
        let token = mgr
            .resolve_account_for_repo(repo_path.as_deref(), target_host)
            .map(|(_, t)| t);
        git::list_pull_requests(target_host, &owner, &repo, token.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn list_github_issues(
    owner: String,
    repo: String,
    host: Option<String>,
    repo_path: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<Vec<IssueInfo>, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        let target_host = host.as_deref().unwrap_or(GITHUB_PUBLIC_HOST);
        let token = mgr
            .resolve_account_for_repo(repo_path.as_deref(), target_host)
            .map(|(_, t)| t);
        git::list_issues(target_host, &owner, &repo, token.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn checkout_github_pull_request(
    repo_path: String,
    pr_number: u64,
    remote: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        let creds = resolve_remote_creds(&mgr, &repo_path, remote.as_deref());
        git::checkout_pull_request(
            &repo_path,
            pr_number,
            remote.as_deref(),
            creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
        )
    })
    .await
}

#[tauri::command]
pub async fn get_commit_check_runs(
    owner: String,
    repo: String,
    commit_sha: String,
    host: Option<String>,
    repo_path: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<Vec<CheckRunInfo>, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        let target_host = host.as_deref().unwrap_or(GITHUB_PUBLIC_HOST);
        let token = mgr
            .resolve_account_for_repo(repo_path.as_deref(), target_host)
            .map(|(_, t)| t);
        git::get_commit_check_runs(target_host, &owner, &repo, &commit_sha, token.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn squash_commits(
    repo: String,
    count: usize,
    new_summary: String,
    new_description: Option<String>,
) -> Result<String, String> {
    run_blocking(move || {
        git::squash_commits(&repo, count, &new_summary, new_description.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn get_global_config() -> Result<RepoConfig, String> {
    run_blocking(git::get_global_config).await
}

#[tauri::command]
pub async fn set_global_config(user_name: String, user_email: String) -> Result<(), String> {
    run_blocking(move || git::set_global_config(&user_name, &user_email)).await
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
pub async fn compare_branches(
    repo: String,
    base: String,
    target: String,
) -> Result<AheadBehind, String> {
    run_blocking(move || git::compare_branches(&repo, &base, &target)).await
}

#[tauri::command]
pub async fn publish_repository_to_github(
    repo: String,
    params: PublishRepoParams,
    account_id: Option<String>,
    host: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::publish_repository_to_github(
            &mgr,
            &repo,
            params,
            account_id.as_deref(),
            host.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub async fn sync_fork(
    repo: String,
    upstream_remote: Option<String>,
    branch: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        let rem = upstream_remote.as_deref().unwrap_or("upstream");
        let creds = resolve_remote_creds(&mgr, &repo, Some(rem));
        git::sync_fork(
            &repo,
            upstream_remote.as_deref(),
            branch.as_deref(),
            creds.as_ref().map(|(u, t)| (u.as_str(), t.as_str())),
        )
    })
    .await
}

#[tauri::command]
pub async fn get_image_diff(repo: String, file_path: String) -> Result<ImageDiffData, String> {
    run_blocking(move || git::get_image_diff(&repo, &file_path)).await
}

#[tauri::command]
pub async fn get_app_runtime_info() -> Result<AppRuntimeInfo, String> {
    run_blocking(|| Ok(updater::get_app_runtime_info())).await
}

#[tauri::command]
pub async fn check_app_update(
    channel: Option<String>,
    target_mode: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<AppUpdateInfo, String> {
    let token = accounts
        .resolve_account_for_repo(None, GITHUB_PUBLIC_HOST)
        .map(|(_, t)| t);
    let ch = UpdateChannel::parse(channel.as_deref());
    let target = updater::TargetPackageMode::parse(target_mode.as_deref());
    run_blocking(move || updater::check_app_update(ch, target, token.as_deref())).await
}

#[tauri::command]
pub async fn install_app_update(
    channel: Option<String>,
    target_mode: Option<String>,
    app: tauri::AppHandle,
    accounts: State<'_, AccountManager>,
) -> Result<UpdateApplyResult, String> {
    let token = accounts
        .resolve_account_for_repo(None, GITHUB_PUBLIC_HOST)
        .map(|(_, t)| t);
    let ch = UpdateChannel::parse(channel.as_deref());
    let target = updater::TargetPackageMode::parse(target_mode.as_deref());
    let res = run_blocking(move || updater::download_and_install_update(ch, target, token.as_deref())).await?;
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
pub async fn stage_hunk(
    repo: String,
    patch_text: String,
    reverse: bool,
) -> Result<(), String> {
    run_blocking(move || git::stage_hunk(&repo, &patch_text, reverse)).await
}

#[tauri::command]
pub async fn interactive_rebase(
    repo: String,
    base_sha: String,
    steps: Vec<RebaseStep>,
) -> Result<String, String> {
    run_blocking(move || git::interactive_rebase(&repo, &base_sha, steps)).await
}

#[tauri::command]
pub async fn stage_lines(
    repo: String,
    hunk_patch: String,
    selected_indices: Vec<usize>,
    reverse: bool,
) -> Result<(), String> {
    run_blocking(move || git::stage_lines(&repo, &hunk_patch, selected_indices, reverse)).await
}

#[tauri::command]
pub async fn get_file_blame(
    repo: String,
    file_path: String,
    rev: Option<String>,
) -> Result<Vec<BlameLine>, String> {
    run_blocking(move || git::get_file_blame(&repo, &file_path, rev.as_deref())).await
}

#[tauri::command]
pub async fn get_file_history(
    repo: String,
    file_path: String,
    limit: Option<usize>,
) -> Result<Vec<CommitInfo>, String> {
    let lim = limit.unwrap_or(80);
    run_blocking(move || git::get_file_history(&repo, &file_path, lim)).await
}

#[tauri::command]
pub async fn get_conflict_file(
    repo: String,
    file_path: String,
) -> Result<ConflictFileData, String> {
    run_blocking(move || git::get_conflict_file(&repo, &file_path)).await
}

#[tauri::command]
pub async fn resolve_conflict_file(
    repo: String,
    file_path: String,
    resolved_content: String,
    stage_after: Option<bool>,
) -> Result<(), String> {
    let stage = stage_after.unwrap_or(true);
    run_blocking(move || git::resolve_conflict_file(&repo, &file_path, &resolved_content, stage)).await
}

#[tauri::command]
pub async fn list_reflog(repo: String, limit: Option<usize>) -> Result<Vec<ReflogEntry>, String> {
    let lim = limit.unwrap_or(80);
    run_blocking(move || git::list_reflog(&repo, lim)).await
}

#[tauri::command]
pub async fn list_submodules(repo: String) -> Result<Vec<SubmoduleInfo>, String> {
    run_blocking(move || git::list_submodules(&repo)).await
}

#[tauri::command]
pub async fn add_submodule(repo: String, url: String, path: String) -> Result<String, String> {
    run_blocking(move || git::add_submodule(&repo, &url, &path)).await
}

#[tauri::command]
pub async fn update_submodules(
    repo: String,
    init: Option<bool>,
    recursive: Option<bool>,
) -> Result<String, String> {
    run_blocking(move || git::update_submodules(&repo, init.unwrap_or(true), recursive.unwrap_or(true))).await
}

#[tauri::command]
pub async fn sync_submodules(repo: String) -> Result<String, String> {
    run_blocking(move || git::sync_submodules(&repo)).await
}

#[tauri::command]
pub async fn list_worktrees(repo: String) -> Result<Vec<WorktreeInfo>, String> {
    run_blocking(move || git::list_worktrees(&repo)).await
}

#[tauri::command]
pub async fn add_worktree(
    repo: String,
    path: String,
    branch: String,
    create_new_branch: Option<bool>,
) -> Result<String, String> {
    run_blocking(move || git::add_worktree(&repo, &path, &branch, create_new_branch.unwrap_or(false))).await
}

#[tauri::command]
pub async fn remove_worktree(
    repo: String,
    path: String,
    force: Option<bool>,
) -> Result<String, String> {
    run_blocking(move || git::remove_worktree(&repo, &path, force.unwrap_or(false))).await
}

#[tauri::command]
pub async fn prune_worktrees(repo: String) -> Result<String, String> {
    run_blocking(move || git::prune_worktrees(&repo)).await
}

#[tauri::command]
pub async fn parse_hosting_repository(remote_url: String) -> Result<Option<ParsedHostingRepo>, String> {
    run_blocking(move || Ok(git::parse_hosting_repo(&remote_url))).await
}

#[tauri::command]
pub async fn connect_provider_pat(
    provider: HostingProvider,
    host: String,
    token: String,
    label: Option<String>,
    allow_memory_fallback: Option<bool>,
    accounts: State<'_, AccountManager>,
) -> Result<AccountProfile, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        git::connect_provider_pat(
            &mgr,
            provider,
            &host,
            &token,
            label.as_deref(),
            allow_memory_fallback.unwrap_or(false),
        )
    })
    .await
}

#[tauri::command]
pub async fn list_provider_pull_requests(
    remote_url: String,
    repo: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<Vec<PullRequestInfo>, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || git::list_provider_pull_requests(&mgr, &remote_url, repo.as_deref())).await
}

#[tauri::command]
pub async fn list_provider_issues(
    remote_url: String,
    repo: Option<String>,
    accounts: State<'_, AccountManager>,
) -> Result<Vec<IssueInfo>, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || git::list_provider_issues(&mgr, &remote_url, repo.as_deref())).await
}

#[tauri::command]
pub async fn checkout_provider_pull_request(
    repo: String,
    provider: HostingProvider,
    pr_number: u64,
    head_branch: String,
    accounts: State<'_, AccountManager>,
) -> Result<String, String> {
    let mgr = accounts.inner().clone();
    run_blocking(move || {
        let creds = resolve_remote_creds(&mgr, &repo, Some("origin"));
        git::checkout_provider_pull_request(&repo, provider, pr_number, &head_branch, creds)
    })
    .await
}

#[tauri::command]
pub async fn get_ai_config(ai: State<'_, Arc<AiManager>>) -> Result<AiConfigPublic, String> {
    let mgr = Arc::clone(ai.inner());
    run_blocking(move || mgr.get_config()).await
}

#[tauri::command]
pub async fn save_ai_config(
    input: SaveAiConfigInput,
    ai: State<'_, Arc<AiManager>>,
) -> Result<AiConfigPublic, String> {
    let mgr = Arc::clone(ai.inner());
    run_blocking(move || mgr.save_config(input)).await
}

#[tauri::command]
pub async fn test_ai_connection(ai: State<'_, Arc<AiManager>>) -> Result<AiTestResult, String> {
    let mgr = Arc::clone(ai.inner());
    run_blocking(move || mgr.test_connection()).await
}

#[tauri::command]
pub async fn ai_run_task(
    task_kind: String,
    prompt_input: String,
    repo: Option<String>,
    extra_context: Option<String>,
    ai: State<'_, Arc<AiManager>>,
) -> Result<AiTaskResult, String> {
    let mgr = Arc::clone(ai.inner());
    run_blocking(move || {
        mgr.run_task(
            &task_kind,
            &prompt_input,
            repo.as_deref(),
            extra_context.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub async fn run_terminal_command(
    session_id: String,
    repo: String,
    command: String,
    shell_kind: Option<String>,
    app: AppHandle,
    term: State<'_, Arc<TerminalManager>>,
) -> Result<TerminalExecResult, String> {
    let mgr = Arc::clone(term.inner());
    run_blocking(move || {
        mgr.run_command(&app, &session_id, &repo, &command, shell_kind.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn kill_terminal_command(
    session_id: String,
    term: State<'_, Arc<TerminalManager>>,
) -> Result<bool, String> {
    let mgr = Arc::clone(term.inner());
    run_blocking(move || mgr.kill_session(&session_id)).await
}

#[tauri::command]
pub async fn get_terminal_cwd(
    session_id: String,
    repo: String,
    term: State<'_, Arc<TerminalManager>>,
) -> Result<String, String> {
    let mgr = Arc::clone(term.inner());
    run_blocking(move || {
        let p = mgr.resolve_cwd(&session_id, &repo);
        Ok(p.to_string_lossy().to_string())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::normalize_git_http_username;

    #[test]
    fn normalize_git_http_username_handles_multi_provider_logins() {
        assert_eq!(normalize_git_http_username("bitbucket.org", "bitbucket-user"), "x-token-auth");
        assert_eq!(normalize_git_http_username("bitbucket.org", ""), "x-token-auth");
        assert_eq!(normalize_git_http_username("bitbucket.org", "alice"), "alice");
        assert_eq!(normalize_git_http_username("gitlab.com", ""), "oauth2");
        assert_eq!(normalize_git_http_username("gitlab.com", "bob"), "bob");
        assert_eq!(normalize_git_http_username("github.com", "octocat"), "octocat");
    }
}
