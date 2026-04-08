mod commands;
mod constants;
mod models;
mod repositories;
mod services;

use commands::branches::{checkout_branch, create_branch, delete_branch, get_branches};
use commands::clone_init::{clone_repo, init_repo, search_commits};
use commands::commit_files::get_commit_files;
use commands::commits::{get_author_deletion_stats, get_commit_tree_paths, get_commits};
use commands::conflicts::{get_conflict_hotspots, get_conflict_pairs, get_repository_tree_paths, get_merge_preflight_risk};
use commands::credentials::{delete_token, load_token, save_token, save_provider_token, load_provider_token, delete_provider_token};
use commands::diff::{get_working_diff, get_commit_diff, get_file_content, get_staged_file_content, get_staged_diff_summary, save_file_content, revert_hunk, has_conflict_markers};
use commands::ghost::{discard_ghost_branch, get_ghost_branch_state, materialize_ghost_branch, start_ghost_branch};
use commands::logs::{append_app_log, get_app_log_path};
use commands::operations::{cherry_pick, checkout_commit, create_tag_at, delete_tag, fetch_all, get_git_path, pull, push, push_force, reset_to_commit, revert_commit, run_git_command, run_shell_command, remove_cached_all, search_github_repos, search_gitlab_repos, search_bitbucket_repos, search_azure_repos, generate_ssh_key, add_gitlab_ssh_key, verify_gitlab_token, get_available_external_editors, get_available_external_tools, open_file_with_editor, open_path_with_tool, rename_branch, delete_remote_branch, set_upstream, edit_commit_message, create_annotated_tag, reset_branch_to_remote, push_to_platform, check_origin};
use commands::repository::get_repo_info;
use commands::stash::{stash_apply, stash_drop, stash_files, stash_list, stash_pop, stash_push};
use commands::status::{add_gitkeep, create_commit, discard_file, discard_files, get_empty_directories, get_status, resolve_all_conflicts, resolve_conflict_file, stage_file, stage_files, unstage_file, unstage_files};
use commands::tags::get_tags;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_repo_info,
            get_commits,
            get_author_deletion_stats,
            get_commit_tree_paths,
            get_conflict_hotspots,
            get_conflict_pairs,
            get_repository_tree_paths,
            get_merge_preflight_risk,
            get_branches,
            checkout_branch,
            create_branch,
            delete_branch,
            get_status,
            stage_file,
            stage_files,
            unstage_file,
            unstage_files,
            create_commit,
            pull,
            push,
            push_force,
            fetch_all,
            clone_repo,
            init_repo,
            search_commits,
            get_commit_files,
            stash_list,
            stash_push,
            stash_pop,
            stash_apply,
            stash_drop,
            stash_files,
            get_tags,
            run_git_command,
            run_shell_command,
            remove_cached_all,
            discard_file,
            discard_files,
            resolve_conflict_file,
            resolve_all_conflicts,
            get_empty_directories,
            add_gitkeep,
            cherry_pick,
            revert_commit,
            reset_to_commit,
            checkout_commit,
            create_tag_at,
            delete_tag,
            search_github_repos,
            search_gitlab_repos,
            search_bitbucket_repos,
            search_azure_repos,
            generate_ssh_key,
            add_gitlab_ssh_key,
            verify_gitlab_token,
            get_ghost_branch_state,
            start_ghost_branch,
            materialize_ghost_branch,
            discard_ghost_branch,
            get_available_external_editors,
            get_available_external_tools,
            open_file_with_editor,
            open_path_with_tool,
            save_token,
            load_token,
            delete_token,
            save_provider_token,
            load_provider_token,
            delete_provider_token,
            get_git_path,
            rename_branch,
            delete_remote_branch,
            set_upstream,
            edit_commit_message,
            create_annotated_tag,
            reset_branch_to_remote,
            push_to_platform,
            check_origin,
            get_working_diff,
            get_commit_diff,
            get_file_content,
            get_staged_file_content,
            get_staged_diff_summary,
            save_file_content,
            revert_hunk,
            has_conflict_markers,
            append_app_log,
            get_app_log_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
