pub mod commands;
pub mod diagnostics;
pub mod git;
pub mod system;
pub mod updater;
use crate::git::AccountManager;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("forge-desktop"));
            app.manage(AccountManager::new(app_data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::inspect_repository,
            commands::stage_file,
            commands::unstage_file,
            commands::stage_all,
            commands::unstage_all,
            commands::commit_changes,
            commands::undo_last_commit,
            commands::get_file_diff,
            commands::add_to_gitignore,
            commands::get_commit_history,
            commands::get_commit_detail,
            commands::get_commit_file_diff,
            commands::list_branches,
            commands::switch_branch,
            commands::create_branch,
            commands::delete_branch,
            commands::rename_branch,
            commands::merge_branch,
            commands::rebase_branch,
            commands::cherry_pick,
            commands::revert_commit,
            commands::reset_to_commit,
            commands::list_tags,
            commands::create_tag,
            commands::delete_tag,
            commands::abort_merge,
            commands::abort_rebase,
            commands::list_remotes,
            commands::add_remote,
            commands::set_remote_url,
            commands::remove_remote,
            commands::get_branch_ahead_behind,
            commands::git_fetch,
            commands::git_pull,
            commands::git_push,
            commands::git_force_push,
            commands::clone_repository,
            commands::init_repository,
            commands::get_repo_config,
            commands::set_repo_config,
            commands::github_start_device_flow,
            commands::list_github_accounts,
            commands::verify_and_connect_pat,
            commands::poll_and_connect_device_flow,
            commands::detect_local_github_sessions,
            commands::import_local_github_session,
            commands::bind_account_to_repo,
            commands::set_default_github_account,
            commands::remove_github_account,
            commands::update_github_account_label,
            commands::check_github_account_status,
            commands::discard_file_changes,
            commands::discard_all_changes,
            commands::git_stash_save,
            commands::git_stash_pop,
            commands::git_list_stashes,
            commands::git_stash_show,
            commands::git_stash_drop,
            commands::check_repo_exists,
            commands::get_repo_path_state,
            commands::get_diagnostics,
            commands::open_in_file_manager,
            commands::open_in_terminal,
            commands::open_in_editor,
            commands::parse_github_repository,
            commands::list_github_pull_requests,
            commands::list_github_issues,
            commands::checkout_github_pull_request,
            commands::get_commit_check_runs,
            commands::squash_commits,
            commands::get_global_config,
            commands::set_global_config,
            commands::get_cli_repo_arg,
            commands::compare_branches,
            commands::publish_repository_to_github,
            commands::sync_fork,
            commands::get_image_diff,
            commands::get_app_runtime_info,
            commands::check_app_update,
            commands::install_app_update,
            commands::stage_hunk,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
