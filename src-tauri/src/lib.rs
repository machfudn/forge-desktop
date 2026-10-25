pub mod commands;
pub mod git;

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
            commands::get_file_diff,
            commands::get_commit_history,
            commands::list_branches,
            commands::switch_branch,
            commands::create_branch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
