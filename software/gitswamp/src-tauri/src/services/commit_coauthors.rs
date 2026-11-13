use git2::{Oid, Repository};
use serde::Serialize;

use crate::repositories::git_repository::GitRepository;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Coauthor {
    name: String,
    email: String,
}

fn identity(value: &str) -> Option<(&str, &str)> {
    let (name, email) = value.trim().strip_suffix('>')?.rsplit_once('<')?;
    let (name, email) = (name.trim(), email.trim());
    if name.is_empty()
        || email.is_empty()
        || email.contains(['<', '>'])
        || email.chars().any(char::is_whitespace)
    {
        return None;
    }
    Some((name, email))
}

fn trailers(message: &[u8]) -> Option<git2::MessageTrailersBytes> {
    const KEY: &[u8] = b"Co-authored-by";
    if !message
        .windows(KEY.len())
        .any(|bytes| bytes.eq_ignore_ascii_case(KEY))
    {
        return None;
    }
    git2::message_trailers_bytes(message).ok()
}

fn identities(trailers: &git2::MessageTrailersBytes) -> impl Iterator<Item = (&str, &str)> {
    trailers.iter().filter_map(|(key, value)| {
        if !key.eq_ignore_ascii_case(b"Co-authored-by") {
            return None;
        }
        identity(std::str::from_utf8(value).ok()?)
    })
}

pub fn get(path: &str, sha: &str) -> Result<Vec<Coauthor>, String> {
    let repo = GitRepository::open(path)?;
    let commit = repo
        .find_commit(Oid::from_str(sha).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(from_message(commit.message_bytes()))
}

fn from_message(message: &[u8]) -> Vec<Coauthor> {
    let Some(trailers) = trailers(message) else {
        return Vec::new();
    };
    let mut result: Vec<Coauthor> = Vec::new();
    for (name, email) in identities(&trailers) {
        if !result
            .iter()
            .any(|author| author.email.eq_ignore_ascii_case(email))
        {
            result.push(Coauthor {
                name: name.to_owned(),
                email: email.to_owned(),
            });
        }
    }
    result
}

pub fn matches(commit: &git2::Commit<'_>, query: &str) -> bool {
    let Some(trailers) = trailers(commit.message_bytes()) else {
        return false;
    };
    let found = identities(&trailers).any(|(name, email)| {
        name.to_lowercase().contains(query) || email.to_lowercase().contains(query)
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::git_service::GitService;
    use std::path::Path;

    #[test]
    fn only_valid_git_trailers_are_displayed_and_duplicates_are_removed() {
        let message = "Subject\n\nCo-authored-by: Not a trailer <body@example.invalid>\n\nMore text.\n\nCo-authored-by: Ana Šarić <ana@example.invalid>\nco-authored-by: Duplicate <ANA@example.invalid>\nCo-authored-by: Broken identity\nCO-AUTHORED-BY: Ivo <ivo@example.invalid>\nSigned-off-by: Other <other@example.invalid>\n";
        let authors = from_message(message.as_bytes());
        assert_eq!(
            authors
                .iter()
                .map(|author| author.name.as_str())
                .collect::<Vec<_>>(),
            ["Ana Šarić", "Ivo"]
        );
        assert!(from_message(
            b"Subject\n\nCo-authored-by: Body <body@example.invalid>\n\nAn explanation.\n"
        )
        .is_empty());
    }

    #[test]
    fn searches_name_and_email_in_real_commits_without_changing_plain_search() {
        let directory =
            tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap();
        let repo = Repository::init(directory.path()).unwrap();
        let tree = repo
            .find_tree(repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        let signature = git2::Signature::now("Main Author", "author@example.invalid").unwrap();
        let parent = repo.commit(Some("HEAD"), &signature, &signature, "Coauthor search body example\n\nCo-authored-by: Ana Šarić <ana@example.invalid>\n\nThis example is not a trailer.", &tree, &[]).unwrap();
        let parent_commit = repo.find_commit(parent).unwrap();
        let actual = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Actual change\n\nco-authored-by: Ana Šarić <ana@example.invalid>",
                &tree,
                &[&parent_commit],
            )
            .unwrap();
        let path = directory.path().to_str().unwrap();
        for query in ["coauthor:ŠARIĆ", "coauthor:ANA@EXAMPLE.INVALID"] {
            let matches = GitService::search_commits(path, query, 20).unwrap();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].sha, actual.to_string());
        }
        assert_eq!(
            GitService::search_commits(path, "ana@example.invalid", 20)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            GitService::search_commits(path, "main author", 20)
                .unwrap()
                .len(),
            2
        );
        assert!(GitService::search_commits(path, "coauthor:", 20)
            .unwrap()
            .is_empty());
        assert_eq!(
            get(path, &actual.to_string()).unwrap()[0].email,
            "ana@example.invalid"
        );
    }
}
