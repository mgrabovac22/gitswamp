use serde::Serialize;

#[derive(Serialize)]
pub struct GitlabRepo {
    pub full_name: String,
    pub path_with_namespace: String,
    pub clone_url_ssh: String,
    pub clone_url_https: String,
    pub description: String,
    pub is_private: bool,
    pub stars: u32,
}
