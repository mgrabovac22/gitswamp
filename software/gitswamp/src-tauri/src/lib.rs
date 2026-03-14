mod commands;
mod models;
mod services;

use commands::branches::{checkout_branch, create_branch, delete_branch, get_branches};
use commands::clone_init::{clone_repo, init_repo, search_commits};
use commands::commit_files::get_commit_files;
use commands::commits::get_commits;
use commands::credentials::{delete_token, load_token, save_token};
use commands::operations::{fetch_all, get_git_path, pull, push, run_git_command};
use commands::repository::get_repo_info;
use commands::stash::{stash_apply, stash_drop, stash_list, stash_pop, stash_push};
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
            get_tags,
            run_git_command,
            discard_file,
            save_token,
            load_token,
            delete_token,
            get_git_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
