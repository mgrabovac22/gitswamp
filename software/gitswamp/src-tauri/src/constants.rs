pub const DEFAULT_BRANCH: &str = "main";
pub const DEFAULT_COMMIT_AUTHOR: &str = "GitSwamp";
pub const DEFAULT_COMMIT_EMAIL: &str = "gitswamp@local";
pub const APP_USER_AGENT: &str = "GitSwamp";

pub const HTTPS_SCHEME: &str = "https://";
pub const GITHUB_HOST: &str = "github.com";
pub const GITLAB_HOST: &str = "gitlab.com";
pub const BITBUCKET_HOST: &str = "bitbucket.org";
pub const AZURE_HOST: &str = "dev.azure.com";
pub const AZURE_LEGACY_HOST: &str = "visualstudio.com";

pub const API_GITHUB_LIST_REPOS: &str = "https://api.github.com/user/repos?per_page=50&sort=updated&affiliation=owner,collaborator,organization_member";
pub const API_GITHUB_SEARCH_REPOS: &str = "https://api.github.com/search/repositories?q={}&per_page=30";
pub const API_GITHUB_USER_KEYS_PATH: &str = "https://api.github.com/user/keys";
pub const API_GITHUB_USER_PATH: &str = "https://api.github.com/user";
pub const API_BITBUCKET_LIST_REPOS: &str = "https://api.bitbucket.org/2.0/repositories?role=member&sort=-updated_on&pagelen=50";
pub const API_GITLAB_BASE_PATH: &str = "/api/v4";
pub const API_GITLAB_USER_KEYS_PATH: &str = "/api/v4/user/keys";
pub const API_GITLAB_USER_PATH: &str = "/api/v4/user";
pub const API_AZURE_REPOS_PATH: &str = "/_apis/git/repositories?api-version=7.1-preview.1";

pub const GITHUB_ACCEPT_HEADER: &str = "application/vnd.github.v3+json";
pub const JSON_ACCEPT_HEADER: &str = "application/json";

pub const AUTH_USER_GITHUB: &str = "x-access-token";
pub const AUTH_USER_GITLAB: &str = "oauth2";
pub const AUTH_USER_BITBUCKET: &str = "x-token-auth";

pub const TEMP_PUSH_REMOTE_AUTH: &str = "temp_push_origin_auth";
pub const TEMP_PUSH_REMOTE_PLATFORM: &str = "temp_push_remote";

pub const CONFLICT_START: &str = "<<<<<<<";
pub const CONFLICT_MID: &str = "=======";
pub const CONFLICT_END: &str = ">>>>>>>";

pub const PLATFORM_GITHUB: &str = "github";
pub const PLATFORM_GITHUB_ENTERPRISE: &str = "github-enterprise";
pub const PLATFORM_GITLAB: &str = "gitlab";
pub const PLATFORM_GITLAB_SELF_HOSTED: &str = "gitlab-self-hosted";
pub const PLATFORM_BITBUCKET: &str = "bitbucket";
pub const PLATFORM_AZURE: &str = "azure";

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
pub const COMMON_GIT_PATHS: &[&str] = &[
    r"C:\Program Files\Git\cmd\git.exe",
    r"C:\Program Files\Git\bin\git.exe",
    r"C:\Program Files\Git\mingw64\bin\git.exe",
    r"C:\Program Files (x86)\Git\cmd\git.exe",
    r"C:\Program Files (x86)\Git\bin\git.exe",
];
