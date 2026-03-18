mod commands;
mod models;
mod services;

use commands::branches::{checkout_branch, create_branch, delete_branch, get_branches};
use commands::clone_init::{clone_repo, init_repo, search_commits};
use commands::commit_files::get_commit_files;
use commands::commits::get_commits;
use commands::credentials::{delete_token, load_token, save_token, save_provider_token, load_provider_token, delete_provider_token};
use commands::diff::{get_working_diff, get_commit_diff, get_file_content, save_file_content, revert_hunk};
use commands::operations::{cherry_pick, checkout_commit, create_tag_at, delete_tag, fetch_all, get_git_path, pull, push, reset_to_commit, revert_commit, run_git_command, search_github_repos, rename_branch, delete_remote_branch, set_upstream, edit_commit_message, create_annotated_tag, reset_branch_to_remote};
use commands::repository::get_repo_info;
use commands::stash::{stash_apply, stash_drop, stash_files, stash_list, stash_pop, stash_push};
use commands::status::{create_commit, discard_file, get_status, stage_file, unstage_file};
use commands::tags::get_tags;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_repo_info,
            get_commits,
            get_branches,
            checkout_branch,
            create_branch,
            delete_branch,
            get_status,
            stage_file,
            unstage_file,
            create_commit,
            pull,
            push,
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
            discard_file,
            cherry_pick,
            revert_commit,
            reset_to_commit,
            checkout_commit,
            create_tag_at,
            delete_tag,
            search_github_repos,
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
            get_working_diff,
            get_commit_diff,
            get_file_content,
            save_file_content,
            revert_hunk,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
