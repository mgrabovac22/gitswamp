use serde::Serialize;

#[derive(Serialize)]
pub struct GithubRepo {
    pub full_name: String,
    pub clone_url: String,
    pub description: String,
    pub is_private: bool,
    pub stars: u32,
}
