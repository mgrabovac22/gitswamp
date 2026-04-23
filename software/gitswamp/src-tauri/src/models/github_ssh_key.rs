use serde::Serialize;

#[derive(Serialize)]
pub struct GithubSshKey {
    pub id: u64,
    pub title: String,
    pub key: String,
    pub fingerprint: String,
    pub created_at: String,
}
