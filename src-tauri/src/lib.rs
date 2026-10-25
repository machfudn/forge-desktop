pub mod commands;
pub mod diagnostics;
pub mod git;
pub mod system;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::inspect_repository,
            commands::stage_file,
            commands::unstage_file,
            commands::stage_all,
            commands::unstage_all,
            commands::commit_changes,
            commands::undo_last_commit,
            commands::get_file_diff,
            commands::get_commit_history,
            commands::list_branches,
            commands::switch_branch,
            commands::create_branch,
            commands::delete_branch,
            commands::rename_branch,
            commands::merge_branch,
            commands::abort_merge,
            commands::abort_rebase,
            commands::list_remotes,
            commands::get_branch_ahead_behind,
            commands::git_fetch,
            commands::git_pull,
            commands::git_push,
            commands::clone_repository,
            commands::init_repository,
            commands::get_repo_config,
            commands::set_repo_config,
            commands::github_start_device_flow,
            commands::github_poll_token,
            commands::configure_git_credentials,
            commands::discard_file_changes,
            commands::git_stash_save,
            commands::git_stash_pop,
            commands::git_list_stashes,
            commands::get_diagnostics,
            commands::open_in_file_manager,
            commands::open_in_terminal,
            commands::open_in_editor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
