use std::collections::{BTreeMap, BTreeSet};

use git2::{
    Commit, Delta, DiffFindOptions, DiffOptions, Oid, Repository, RepositoryState, Signature, Tree,
};

use super::{GitRepository, SelectionResult, StashService};

const STASH_REF: &str = "refs/stash";

fn error(error: git2::Error) -> String {
    error.message().to_owned()
}

fn name(path: Option<&[u8]>) -> Result<Option<&str>, &'static str> {
    path.map(std::str::from_utf8)
        .transpose()
        .map_err(|_| "Selective stash restore requires UTF-8 paths.")
}

pub(super) fn open_stash(
    repo: &Repository,
    index: usize,
    sha: Option<&str>,
) -> Result<Oid, String> {
    let log = repo.reflog(STASH_REF).map_err(error)?;
    let oid = log
        .get(index)
        .map(|entry| entry.id_new())
        .ok_or("The selected stash no longer exists.")?;
    if sha.is_some_and(|sha| Oid::from_str(sha).ok() != Some(oid)) {
        return Err("The stash list changed. Refresh and select the stash again.".into());
    }
    Ok(oid)
}

fn changed_paths(
    repo: &Repository,
    left: Option<&Tree<'_>>,
    right: &Tree<'_>,
    paths: &mut BTreeSet<String>,
    renames: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let mut options = DiffOptions::new();
    options.skip_binary_check(true).include_typechange(true);
    let mut diff = repo
        .diff_tree_to_tree(left, Some(right), Some(&mut options))
        .map_err(error)?;
    if left.is_some() {
        diff.find_similar(Some(
            DiffFindOptions::new().renames(true).exact_match_only(true),
        ))
        .map_err(error)?;
    }
    for delta in diff.deltas() {
        let old = name(delta.old_file().path_bytes())?;
        let new = name(delta.new_file().path_bytes())?;
        for path in [old, new].into_iter().flatten() {
            paths.insert(path.to_owned());
        }
        if delta.status() == Delta::Renamed {
            if let (Some(old), Some(new)) = (old, new) {
                renames.push((old.into(), new.into()));
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct PathSelection<'a> {
    whole: bool,
    children: BTreeMap<&'a str, PathSelection<'a>>,
}

impl<'a> PathSelection<'a> {
    fn insert(&mut self, path: &'a str) {
        let mut node = self;
        for name in path.split('/') {
            node = node.children.entry(name).or_default();
        }
        node.whole = true;
    }

    // Only touched subtrees are rebuilt; file contents never pass through the UI or a patch buffer.
    fn overlay(
        &self,
        repo: &Repository,
        original: Option<&Tree<'_>>,
        source: Option<&Tree<'_>>,
    ) -> Result<Oid, String> {
        let mut builder = repo.treebuilder(original).map_err(error)?;
        for (name, selection) in &self.children {
            let from = original.and_then(|tree| tree.get_name(name));
            let to = source.and_then(|tree| tree.get_name(name));
            if selection.whole {
                if let Some(entry) = to {
                    builder
                        .insert(name, entry.id(), entry.filemode())
                        .map_err(error)?;
                } else if from.is_some() {
                    builder.remove(name).map_err(error)?;
                }
            } else {
                let tree = |entry: Option<git2::TreeEntry<'_>>| {
                    entry
                        .filter(|entry| entry.filemode() == 0o040000)
                        .map(|entry| repo.find_tree(entry.id()))
                        .transpose()
                        .map_err(error)
                };
                let before = tree(from)?;
                let after = tree(to)?;
                if before.is_none() && after.is_none() {
                    continue;
                }
                let updated = repo
                    .find_tree(selection.overlay(repo, before.as_ref(), after.as_ref())?)
                    .map_err(error)?;
                if !updated.is_empty() {
                    builder
                        .insert(name, updated.id(), 0o040000)
                        .map_err(error)?;
                } else if before.is_some() {
                    builder.remove(name).map_err(error)?;
                }
            }
        }
        builder.write().map_err(error)
    }
}

fn expanded_selection(
    paths: &BTreeSet<String>,
    mut selected: BTreeSet<String>,
    renames: &[(String, String)],
) -> BTreeSet<String> {
    loop {
        let size = selected.len();
        for (old, new) in renames {
            if selected.contains(old) || selected.contains(new) {
                selected.insert(old.clone());
                selected.insert(new.clone());
            }
        }
        for path in selected.iter().cloned().collect::<Vec<_>>() {
            for (index, _) in path.match_indices('/') {
                if let Some(parent) = paths.get(&path[..index]) {
                    selected.insert(parent.clone());
                }
            }
            let prefix = path + "/";
            for child in paths
                .range(prefix.clone()..)
                .take_while(|child| child.starts_with(&prefix))
            {
                selected.insert(child.clone());
            }
        }
        if size == selected.len() {
            return selected;
        }
    }
}

fn stash_commit(
    repo: &Repository,
    original: &Commit<'_>,
    base: &Commit<'_>,
    work: Oid,
    index: Oid,
    untracked: Oid,
) -> Result<Oid, String> {
    let sig = original.committer();
    let index = repo.find_tree(index).map_err(error)?;
    let index = repo
        .commit(None, &sig, &sig, "Stash index", &index, &[base])
        .map_err(error)?;
    let index = repo.find_commit(index).map_err(error)?;
    let untracked_tree = repo.find_tree(untracked).map_err(error)?;
    let untracked = if untracked_tree.is_empty() {
        None
    } else {
        let oid = repo
            .commit(
                None,
                &sig,
                &sig,
                "Stash untracked files",
                &untracked_tree,
                &[],
            )
            .map_err(error)?;
        Some(repo.find_commit(oid).map_err(error)?)
    };
    let mut parents = vec![base, &index];
    parents.extend(untracked.as_ref());
    let work = repo.find_tree(work).map_err(error)?;
    repo.commit(
        None,
        &sig,
        &sig,
        &original
            .message()
            .unwrap_or("Stashed changes")
            .replace('\0', ""),
        &work,
        &parents,
    )
    .map_err(error)
}

fn publish_pop(
    repo: &Repository,
    index: usize,
    original: Oid,
    remainder: Option<Oid>,
    signature: &Signature<'_>,
) -> Result<(), String> {
    let mut transaction = repo.transaction().map_err(error)?;
    transaction.lock_ref(STASH_REF).map_err(error)?;
    let mut log = repo.reflog(STASH_REF).map_err(error)?;
    let entry = log
        .get(index)
        .filter(|entry| entry.id_new() == original)
        .ok_or("The stash changed while restoring files.")?;
    let message = entry.message().unwrap_or("Stashed changes").to_owned();
    if let Some(remainder) = remainder {
        log.append(remainder, signature, Some(&message))
            .map_err(error)?;
        log.remove(index + 1, true).map_err(error)?;
    } else {
        log.remove(index, true).map_err(error)?;
    }
    if let Some(tip) = log.get(0).map(|entry| entry.id_new()) {
        transaction
            .set_target(STASH_REF, tip, Some(signature), "Pop selected stash files")
            .map_err(error)?;
        transaction.set_reflog(STASH_REF, log).map_err(error)?;
    } else {
        transaction.remove(STASH_REF).map_err(error)?;
    }
    transaction.commit().map_err(error)
}

pub(super) fn restore(
    path: &str,
    index: usize,
    sha: &str,
    paths: Vec<String>,
    pop: bool,
) -> Result<SelectionResult, String> {
    let repo = GitRepository::open(path)?;
    if repo.is_bare()
        || repo.state() != RepositoryState::Clean
        || crate::services::interactive_rebase::is_active(&repo)
        || repo.index().map_err(error)?.has_conflicts()
    {
        return Err("Finish the current Git operation before restoring stash files.".into());
    }
    let original = open_stash(&repo, index, Some(sha))?;
    let commit = repo.find_commit(original).map_err(error)?;
    let base = commit.parent(0).map_err(error)?;
    let base_tree = base.tree().map_err(error)?;
    let index_tree = commit
        .parent(1)
        .and_then(|parent| parent.tree())
        .map_err(error)?;
    let work_tree = commit.tree().map_err(error)?;
    let untracked_tree = (commit.parent_count() > 2)
        .then(|| commit.parent(2).and_then(|parent| parent.tree()))
        .transpose()
        .map_err(error)?;
    let empty = repo
        .treebuilder(None)
        .and_then(|builder| builder.write())
        .map_err(error)?;
    let mut changes = BTreeSet::new();
    let mut renames = Vec::new();
    changed_paths(
        &repo,
        Some(&base_tree),
        &work_tree,
        &mut changes,
        &mut renames,
    )?;
    if index_tree.id() != work_tree.id() {
        changed_paths(
            &repo,
            Some(&base_tree),
            &index_tree,
            &mut changes,
            &mut renames,
        )?;
    }
    if let Some(tree) = &untracked_tree {
        changed_paths(&repo, None, tree, &mut changes, &mut renames)?;
    }
    let selected: BTreeSet<_> = paths.into_iter().collect();
    if selected.is_empty() || !selected.is_subset(&changes) {
        return Err("Select files from the current stash preview.".into());
    }
    let selected = expanded_selection(&changes, selected, &renames);
    let (apply, remaining) = if selected.len() == changes.len() {
        (original, None)
    } else {
        let mut filter = PathSelection::default();
        for path in &selected {
            filter.insert(path);
        }
        let projected = |invert: bool| -> Result<Oid, String> {
            let project = |before, after| {
                if invert {
                    filter.overlay(&repo, after, before)
                } else {
                    filter.overlay(&repo, before, after)
                }
            };
            let work = project(Some(&base_tree), Some(&work_tree))?;
            let index = project(Some(&base_tree), Some(&index_tree))?;
            let untracked = if untracked_tree.is_some() {
                project(None, untracked_tree.as_ref())?
            } else {
                empty
            };
            stash_commit(&repo, &commit, &base, work, index, untracked)
        };
        (
            projected(false)?,
            if pop { Some(projected(true)?) } else { None },
        )
    };

    GitRepository::git_cli(path, &["stash", "apply", "--index", "--quiet", &apply.to_string()])
        .map_err(|failure| format!("Git could not restore all selected changes. The original stash was kept. Check Working Changes before retrying.\n{failure}"))?;
    if pop {
        if let Err(failure) = publish_pop(
            &repo,
            index,
            original,
            remaining,
            &StashService::stash_signature(&repo)?,
        ) {
            let recovery = remaining.unwrap_or(original);
            return Err(format!("Selected files were restored, but the stash list could not be updated: {failure}. Recovery stash object: {recovery}."));
        }
    }
    let suffix = if !pop {
        "The original stash was kept."
    } else if remaining.is_some() {
        "Unselected files remain in a stash at the top of the list."
    } else {
        "The stash was removed."
    };
    Ok(SelectionResult {
        message: format!("Restored {} stash paths. {suffix}", selected.len()),
        remainder: remaining.map(|oid| oid.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git(repo: &tempfile::TempDir, args: &[&str]) -> String {
        GitRepository::git_cli(repo.path().to_str().unwrap(), args).unwrap()
    }

    fn fixture() -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        git(&repo, &["init", "--initial-branch=main"]);
        git(&repo, &["config", "user.name", "Stash Test"]);
        git(&repo, &["config", "user.email", "stash@example.invalid"]);
        git(&repo, &["config", "core.autocrlf", "false"]);
        for file in ["first.txt", "second.txt", "local.txt"] {
            fs::write(repo.path().join(file), "base\n").unwrap();
        }
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "Base"]);
        repo
    }

    fn put(repo: &tempfile::TempDir, file: &str, body: &str) {
        fs::write(repo.path().join(file), body).unwrap();
    }
    fn get(repo: &tempfile::TempDir, file: &str) -> String {
        fs::read_to_string(repo.path().join(file)).unwrap()
    }

    #[test]
    fn pops_part_of_an_older_stash_and_preserves_staged_unstaged_and_untracked_layers() {
        let repo = fixture();
        put(&repo, "first.txt", "staged\n");
        git(&repo, &["add", "first.txt"]);
        put(&repo, "first.txt", "unstaged\n");
        put(&repo, "second.txt", "kept in stash\n");
        put(&repo, "new.bin", "untracked\n");
        git(&repo, &["stash", "push", "-u", "-m", "older"]);
        let sha = git(&repo, &["rev-parse", "refs/stash"]);
        put(&repo, "local.txt", "newer stash\n");
        git(&repo, &["stash", "push", "-m", "newer"]);
        let newer = git(&repo, &["rev-parse", "refs/stash"]);
        put(&repo, "local.txt", "local working changes\n");

        let result = restore(
            repo.path().to_str().unwrap(),
            1,
            &sha,
            vec!["first.txt".into()],
            true,
        )
        .unwrap();
        assert_eq!(get(&repo, "first.txt"), "unstaged\n");
        assert_eq!(git(&repo, &["show", ":first.txt"]), "staged");
        assert_eq!(get(&repo, "second.txt"), "base\n");
        assert_eq!(get(&repo, "local.txt"), "local working changes\n");
        assert!(!repo.path().join("new.bin").exists());
        assert_eq!(
            git(&repo, &["rev-parse", "stash@{0}"]),
            result.remainder.unwrap()
        );
        assert_eq!(git(&repo, &["rev-parse", "stash@{1}"]), newer);
        let list = StashService::stash_list(repo.path().to_str().unwrap()).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].message, "older");
        let files =
            StashService::stash_files(repo.path().to_str().unwrap(), 0, Some(&list[0].stash_sha))
                .unwrap();
        assert_eq!(
            files
                .into_iter()
                .map(|item| item.path)
                .collect::<BTreeSet<_>>(),
            ["new.bin".into(), "second.txt".into()]
                .into_iter()
                .collect()
        );
        git(&repo, &["stash", "apply", "stash@{0}"]);
        assert_eq!(get(&repo, "new.bin"), "untracked\n");
        assert_eq!(get(&repo, "second.txt"), "kept in stash\n");
    }

    #[test]
    fn applies_exact_rename_as_one_unit_and_keeps_the_original() {
        let repo = fixture();
        git(&repo, &["mv", "first.txt", "renamed.txt"]);
        put(&repo, "second.txt", "unselected\n");
        git(&repo, &["stash", "push"]);
        let sha = git(&repo, &["rev-parse", "refs/stash"]);
        restore(
            repo.path().to_str().unwrap(),
            0,
            &sha,
            vec!["renamed.txt".into()],
            false,
        )
        .unwrap();
        assert!(!repo.path().join("first.txt").exists());
        assert_eq!(get(&repo, "renamed.txt"), "base\n");
        assert_eq!(get(&repo, "second.txt"), "base\n");
        assert_eq!(git(&repo, &["rev-parse", "refs/stash"]), sha);
    }

    #[test]
    fn stale_or_unapplicable_selection_does_not_pop_any_stash() {
        let repo = fixture();
        put(&repo, "first.txt", "stash\n");
        put(&repo, "second.txt", "remaining\n");
        git(&repo, &["stash", "push"]);
        let sha = git(&repo, &["rev-parse", "refs/stash"]);
        let path = repo.path().to_str().unwrap();
        assert!(restore(path, 0, &"0".repeat(40), vec!["first.txt".into()], true).is_err());
        assert_eq!(get(&repo, "first.txt"), "base\n");
        put(&repo, "first.txt", "current dirty contents\n");
        assert!(restore(path, 0, &sha, vec!["first.txt".into()], true).is_err());
        assert_eq!(get(&repo, "first.txt"), "current dirty contents\n");
        assert_eq!(git(&repo, &["rev-parse", "refs/stash"]), sha);
    }

    #[test]
    fn popping_the_entire_selected_stash_removes_the_last_reflog_entry() {
        let repo = fixture();
        put(&repo, "first.txt", "restored\n");
        git(&repo, &["stash", "push"]);
        let sha = git(&repo, &["rev-parse", "refs/stash"]);
        let result = restore(
            repo.path().to_str().unwrap(),
            0,
            &sha,
            vec!["first.txt".into()],
            true,
        )
        .unwrap();
        assert_eq!(get(&repo, "first.txt"), "restored\n");
        assert_eq!(result.remainder, None);
        assert!(git(&repo, &["stash", "list"]).is_empty());
    }

    #[test]
    fn large_stash_preview_keeps_every_path_with_a_bounded_stats_pass() {
        let repo = fixture();
        let files: Vec<_> = (0..260).map(|n| format!("file_{n:04}.txt")).collect();
        let base = "original text to inspect in the stash preview\n".repeat(320);
        let saved = "worktree text to inspect in the stash preview\n".repeat(320);
        for file in &files {
            put(&repo, file, &base);
        }
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "Populate index"]);
        let path = repo.path().to_str().unwrap();
        put(&repo, "loose.txt", "keep me\n");
        for file in &files {
            put(&repo, file, &saved);
        }
        let oid = StashService::stash_push_oid(path, Some("large preview"), false).unwrap();
        assert_eq!(get(&repo, "loose.txt"), "keep me\n");
        assert_eq!(get(&repo, &files[0]), base);
        let start = std::time::Instant::now();
        let preview = StashService::stash_files(path, 0, Some(&oid.to_string())).unwrap();
        let bounded = start.elapsed();
        assert_eq!(preview.len(), files.len());
        assert!(preview[0].additions > 0 && preview.last().unwrap().additions == 0);
        let start = std::time::Instant::now();
        let repo = GitRepository::open(path).unwrap();
        let commit = repo.find_commit(oid).unwrap();
        let parent = commit.parent(0).unwrap().tree().unwrap();
        let saved = commit.tree().unwrap();
        let diff = repo
            .diff_tree_to_tree(Some(&parent), Some(&saved), None)
            .unwrap();
        for index in 0..diff.deltas().len() {
            assert!(
                git2::Patch::from_diff(&diff, index)
                    .unwrap()
                    .unwrap()
                    .line_stats()
                    .unwrap()
                    .1
                    > 0
            );
        }
        eprintln!(
            "260-file stash preview: bounded = {bounded:?}, all diff stats = {:?}",
            start.elapsed()
        );
    }
}
