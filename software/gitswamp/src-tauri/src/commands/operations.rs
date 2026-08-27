use crate::services::git_service::GitService;

#[tauri::command]
pub async fn get_lost_commits(
    path: String,
    max_count: Option<usize>,
) -> Result<Vec<crate::models::LostCommitInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::lost_commits(&path, max_count.unwrap_or(50))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn pull(
    path: String,
    token: Option<String>,
    auto_stash: Option<bool>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::pull(&path, token.as_deref(), auto_stash.unwrap_or(false))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn push(path: String, token: Option<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::push(&path, token.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn push_force(path: String, token: Option<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::push_force(&path, token.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn fetch_all(path: String, token: Option<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || GitService::fetch_all(&path, token.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn run_git_command(path: String, args: Vec<String>) -> Result<String, String> {
    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    GitService::run_git_command(&path, &str_args)
}

#[tauri::command]
pub fn run_shell_command(path: String, command: String) -> Result<String, String> {
    GitService::run_shell_command(&path, &command)
}

#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&path, content).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn remove_cached_all(path: String) -> Result<String, String> {
    GitService::remove_cached_all(&path)
}

#[tauri::command]
pub fn get_git_path() -> String {
    GitService::get_git_path()
}

#[tauri::command]
pub fn cherry_pick(path: String, sha: String) -> Result<String, String> {
    GitService::cherry_pick(&path, &sha)
}

#[tauri::command]
pub fn revert_commit(path: String, sha: String) -> Result<String, String> {
    GitService::revert_commit(&path, &sha)
}

#[tauri::command]
pub fn reset_to_commit(path: String, sha: String, mode: String) -> Result<String, String> {
    GitService::reset_to_commit(&path, &sha, &mode)
}

#[tauri::command]
pub fn rebase_branch_onto(
    path: String,
    source_branch: String,
    source_remote: bool,
    target_branch: String,
) -> Result<String, String> {
    GitService::rebase_branch_onto(&path, &source_branch, source_remote, &target_branch)
}

#[tauri::command]
pub fn rebase_continue(path: String) -> Result<String, String> {
    GitService::rebase_continue(&path)
}

#[tauri::command]
pub fn rebase_abort(path: String) -> Result<String, String> {
    GitService::rebase_abort(&path)
}

#[tauri::command]
pub fn rebase_skip(path: String) -> Result<String, String> {
    GitService::rebase_skip(&path)
}

#[tauri::command]
pub fn checkout_commit(path: String, sha: String) -> Result<String, String> {
    GitService::checkout_commit(&path, &sha)
}

#[tauri::command]
pub fn create_tag_at(path: String, name: String, sha: String) -> Result<String, String> {
    GitService::create_tag_at(&path, &name, &sha)
}

#[tauri::command]
pub fn delete_tag(path: String, name: String) -> Result<String, String> {
    GitService::delete_tag(&path, &name)
}

#[tauri::command]
pub fn search_github_repos(
    token: String,
    query: String,
    include_public: bool,
) -> Result<Vec<crate::models::GithubRepo>, String> {
    crate::services::git_service::GitService::search_github_repos(&token, &query, include_public)
}

#[tauri::command]
pub fn search_gitlab_repos(
    domain: String,
    token: String,
    query: String,
) -> Result<Vec<crate::models::GitlabRepo>, String> {
    crate::services::git_service::GitService::search_gitlab_repos(&domain, &token, &query)
}

#[tauri::command]
pub fn search_bitbucket_repos(
    token: String,
    query: String,
) -> Result<Vec<crate::models::BitbucketRepo>, String> {
    crate::services::git_service::GitService::search_bitbucket_repos(&token, &query)
}

#[tauri::command]
pub fn search_azure_repos(
    domain: String,
    token: String,
    query: String,
) -> Result<Vec<crate::models::AzureRepo>, String> {
    crate::services::git_service::GitService::search_azure_repos(&domain, &token, &query)
}

#[tauri::command]
pub fn generate_ssh_key(email: String, key_name: String) -> Result<(String, String), String> {
    crate::services::git_service::GitService::generate_ssh_key(&email, &key_name)
}

#[tauri::command]
pub fn add_gitlab_ssh_key(
    domain: String,
    token: String,
    title: String,
    key: String,
) -> Result<(), String> {
    crate::services::git_service::GitService::add_gitlab_ssh_key(&domain, &token, &title, &key)
}

#[tauri::command]
pub fn add_github_ssh_key(token: String, title: String, key: String) -> Result<(), String> {
    crate::services::git_service::GitService::add_github_ssh_key(&token, &title, &key)
}

#[tauri::command]
pub fn list_github_ssh_keys(token: String) -> Result<Vec<crate::models::GithubSshKey>, String> {
    crate::services::git_service::GitService::list_github_ssh_keys(&token)
}

#[tauri::command]
pub fn delete_github_ssh_key(token: String, key_id: u64) -> Result<(), String> {
    crate::services::git_service::GitService::delete_github_ssh_key(&token, key_id)
}

#[tauri::command]
pub fn verify_github_token(token: String) -> Result<String, String> {
    crate::services::git_service::GitService::verify_github_token(&token)
}

#[tauri::command]
pub fn load_ssh_public_key_from_file(file_path: String) -> Result<String, String> {
    crate::services::git_service::GitService::load_ssh_public_key_from_file(&file_path)
}

#[tauri::command]
pub fn connect_github_oauth_via_gh_cli() -> Result<String, String> {
    crate::services::git_service::GitService::connect_github_oauth_via_gh_cli()
}

#[tauri::command]
pub fn verify_gitlab_token(domain: String, token: String) -> Result<String, String> {
    crate::services::git_service::GitService::verify_gitlab_token(&domain, &token)
}

#[tauri::command]
pub fn get_available_external_editors() -> Vec<String> {
    GitService::get_available_external_editors()
}

#[tauri::command]
pub fn get_available_external_tools() -> Vec<String> {
    GitService::get_available_external_tools()
}

#[tauri::command]
pub fn open_file_with_editor(
    path: String,
    file_path: String,
    editor: String,
) -> Result<(), String> {
    GitService::open_file_with_editor(&path, &file_path, &editor)
}

#[tauri::command]
pub fn open_path_with_tool(path: String, tool: String) -> Result<(), String> {
    GitService::open_path_with_tool(&path, &tool)
}

#[tauri::command]
pub fn rename_branch(path: String, old_name: String, new_name: String) -> Result<String, String> {
    GitService::rename_branch(&path, &old_name, &new_name)
}

#[tauri::command]
pub async fn delete_remote_branch(
    path: String,
    remote: String,
    branch: String,
    token: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::delete_remote_branch(&path, &remote, &branch, token.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn set_upstream(path: String, branch: String, remote_branch: String) -> Result<String, String> {
    GitService::set_upstream(&path, &branch, &remote_branch)
}

#[tauri::command]
pub fn edit_commit_message(
    path: String,
    sha: String,
    new_message: String,
) -> Result<String, String> {
    GitService::edit_commit_message(&path, &sha, &new_message)
}

#[tauri::command]
pub fn create_annotated_tag(
    path: String,
    name: String,
    sha: String,
    message: String,
) -> Result<String, String> {
    GitService::create_annotated_tag(&path, &name, &sha, &message)
}

#[tauri::command]
pub fn reset_branch_to_remote(path: String, branch: String) -> Result<String, String> {
    GitService::reset_branch_to_remote(&path, &branch)
}

#[tauri::command]
pub async fn push_to_platform(
    path: String,
    platform: String,
    provider_token: String,
    repo_name: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        GitService::push_to_platform(&path, &platform, &provider_token, &repo_name)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn check_origin(path: String) -> Result<bool, String> {
    GitService::check_origin(&path)
}
