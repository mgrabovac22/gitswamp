use serde::Serialize;

#[derive(Serialize)]
pub struct BitbucketSshKey {
    pub uuid: String,
    pub label: String,
    pub key: String,
    pub fingerprint: String,
    pub created_on: String,
}
