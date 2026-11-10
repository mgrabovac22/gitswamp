use git2::{Commit, Oid};
use serde::{Deserialize, Serialize};

use crate::repositories::git_repository::GitRepository;

const MAX_INCOMING: usize = 6;
const MAX_SUBJECT_CHARS: usize = 160;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationKind {
    Merge,
    CherryPick,
    Revert,
}

#[derive(Serialize)]
struct OperationCommit {
    sha: String,
    subject: String,
}

impl From<Commit<'_>> for OperationCommit {
    fn from(commit: Commit<'_>) -> Self {
        Self {
            sha: commit.id().to_string(),
            subject: commit
                .summary_bytes()
                .map(|bytes| {
                    String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_SUBJECT_CHARS * 4)])
                        .chars()
                        .take(MAX_SUBJECT_CHARS)
                        .map(|ch| if ch.is_control() { ' ' } else { ch })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationSummary {
    before: OperationCommit,
    result: OperationCommit,
    incoming: Vec<OperationCommit>,
    has_more: bool,
}

pub fn summarize(
    path: &str,
    kind: OperationKind,
    result_sha: &str,
    before_sha: Option<&str>,
    source_sha: Option<&str>,
) -> Result<Option<OperationSummary>, String> {
    let repo = GitRepository::open(path)?;
    let find = |sha| -> Result<Commit<'_>, git2::Error> { repo.find_commit(Oid::from_str(sha)?) };
    let result = find(result_sha).map_err(|error| error.to_string())?;
    let merge = matches!(kind, OperationKind::Merge);
    if (merge && result.parent_count() < 2) || (!merge && result.parent_count() != 1) {
        return Ok(None);
    }
    let before = result.parent(0).map_err(|error| error.to_string())?;
    if before_sha.is_some_and(|sha| Oid::from_str(sha).ok() != Some(before.id())) {
        return Ok(None);
    }

    let (incoming, has_more) = if merge {
        // Exclude the target's old history and the merge commit itself. One extra SHA indicates truncation.
        let parents = format!("{}^@", result.id());
        let exclude = format!("^{}", before.id());
        let limit = format!("--max-count={}", MAX_INCOMING + 1);
        let output = GitRepository::git_cli(
            path,
            &[
                "--no-replace-objects",
                "rev-list",
                &limit,
                &parents,
                &exclude,
                "--",
            ],
        )?;
        let mut shas = output.lines().filter(|line| Oid::from_str(line).is_ok());
        let incoming = shas
            .by_ref()
            .take(MAX_INCOMING)
            .map(|sha| find(sha).map(OperationCommit::from))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        (incoming, shas.next().is_some())
    } else {
        let Some(source_sha) = source_sha else {
            return Ok(None);
        };
        let source = find(source_sha).map_err(|error| error.to_string())?;
        (vec![source.into()], false)
    };

    Ok(Some(OperationSummary {
        before: before.into(),
        result: result.into(),
        incoming,
        has_more,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};

    #[test]
    fn previews_incoming_history_and_exact_operation_results() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        let path =
            std::env::temp_dir().join(format!("gitswamp-operation-preview-{}", stamp.as_nanos()));
        let repo = Repository::init(&path).unwrap();
        let dir = path.to_str().unwrap();
        let sig = Signature::now("GitSwamp", "test@example.com").unwrap();
        let tree = repo
            .find_tree(repo.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        let commit = |subject: &str, parents: &[&Commit<'_>]| {
            let oid = repo
                .commit(None, &sig, &sig, subject, &tree, parents)
                .unwrap();
            repo.find_commit(oid).unwrap()
        };
        let root = commit("Shared base", &[]);
        let target = commit("Target-only work", &[&root]);
        let first_incoming = commit("Incoming work", &[&root]);
        let small_merge = commit("Merge topic", &[&target, &first_incoming]);
        let before = target.id().to_string();
        let small = summarize(
            dir,
            OperationKind::Merge,
            &small_merge.id().to_string(),
            Some(&before),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(small.before.sha, before);
        assert_eq!(small.result.subject, "Merge topic");
        assert_eq!(small.incoming.len(), 1);
        assert_eq!(small.incoming[0].sha, first_incoming.id().to_string());
        assert!(!small.has_more);

        let mut tip = first_incoming.clone();
        for index in 0..MAX_INCOMING + 1 {
            tip = commit(&format!("Incoming {index}"), &[&tip]);
        }
        let merge = commit("Merge larger topic", &[&target, &tip]);
        let after = merge.id().to_string();
        let capped = summarize(dir, OperationKind::Merge, &after, None, None)
            .unwrap()
            .unwrap();
        assert_eq!(capped.incoming.len(), MAX_INCOMING);
        assert!(capped.has_more);
        assert!(capped
            .incoming
            .iter()
            .all(|item| item.subject.starts_with("Incoming")));
        assert!(
            summarize(dir, OperationKind::Merge, &after, Some(&after), None)
                .unwrap()
                .is_none()
        );
        assert!(summarize(dir, OperationKind::Merge, &before, None, None)
            .unwrap()
            .is_none());

        let original = first_incoming.id().to_string();
        for (kind, title) in [
            (OperationKind::CherryPick, "Incoming work"),
            (OperationKind::Revert, "Revert incoming work"),
        ] {
            let result = commit(title, &[&target]).id().to_string();
            let summary = summarize(dir, kind, &result, Some(&before), Some(&original))
                .unwrap()
                .unwrap();
            assert_eq!(summary.before.sha, before);
            assert_eq!(summary.incoming[0].sha, original);
            assert_eq!(summary.result.sha, result);
            assert_ne!(summary.result.sha, original);
        }

        drop((merge, tip, small_merge, first_incoming, target, root, tree));
        drop(repo);
        let _ = std::fs::remove_dir_all(path);
    }
}
