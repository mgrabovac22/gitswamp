use serde::Serialize;

#[derive(Serialize)]
pub struct GithubRepo {
    pub full_name: String,
    pub clone_url: String,
    pub description: String,
    pub is_private: bool,
    pub stars: u32,
    pub owner_login: String,
    pub owner_type: String,
    pub viewer_login: String,
    pub is_public_search_result: bool,
}
