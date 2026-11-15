use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

use git2::{Index, Oid};

use super::super::secret_service::CompositionSecrets;
use super::{chunks, dependencies, git_err, Change, Draft, Snapshot, Unit};

pub(super) type Intermediate = BTreeMap<(String, Oid), usize>;

pub(super) struct Series<'a> {
    snapshot: &'a Snapshot,
    index: Index,
    active: BTreeMap<&'a str, Vec<&'a Change>>,
}

impl<'a> Series<'a> {
    pub(super) fn new(snapshot: &'a Snapshot) -> Result<Self, String> {
        let mut index = Index::new().map_err(git_err)?;
        if let Some(id) = snapshot.head {
            index
                .read_tree(
                    &snapshot
                        .repo
                        .find_commit(id)
                        .and_then(|c| c.tree())
                        .map_err(git_err)?,
                )
                .map_err(git_err)?;
        }
        Ok(Self {
            snapshot,
            index,
            active: BTreeMap::new(),
        })
    }

    pub(super) fn advance(
        &mut self,
        units: &[Unit],
        selected: &[usize],
    ) -> Result<(Oid, Intermediate), String> {
        let files = selected.iter().flat_map(|id| {
            units[*id]
                .changes
                .iter()
                .map(|id| &self.snapshot.changes[*id])
        });
        let mut dirty = BTreeMap::new();
        for file in files.clone() {
            if file.chunk.is_some() {
                self.active.entry(&file.path).or_default().push(file);
                dirty.insert(file.path.as_str(), file);
            } else if file.old.is_some() {
                let previous = file.old_path.as_deref().unwrap_or(&file.path);
                self.index
                    .remove_path(Path::new(previous))
                    .map_err(git_err)?;
            }
        }
        for file in files.filter(|file| file.chunk.is_none()) {
            if let Some((id, mode)) = file.new {
                self.add(&file.path, id, mode)?;
            }
        }
        let mut reviews = BTreeMap::new();
        for (path, file) in dirty {
            let (final_blob, mode) = file.new.ok_or("Hunk has no final file.")?;
            let id = chunks::blob(&self.snapshot.repo, &self.active[path])?;
            if id != final_blob {
                let (size, _) = self
                    .snapshot
                    .repo
                    .odb()
                    .and_then(|odb| odb.read_header(id))
                    .map_err(git_err)?;
                reviews.insert((path.to_owned(), id), size);
            }
            self.add(path, id, mode)?;
        }
        Ok((
            self.index
                .write_tree_to(&self.snapshot.repo)
                .map_err(git_err)?,
            reviews,
        ))
    }

    fn add(&mut self, path: &str, id: Oid, mode: u32) -> Result<(), String> {
        self.index
            .add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode,
                uid: 0,
                gid: 0,
                file_size: 0,
                id,
                flags: 0,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            })
            .map_err(git_err)
    }
}

pub(super) fn review_intermediate(
    snapshot: &Snapshot,
    blobs: &BTreeMap<(String, Oid), usize>,
    review: &mut CompositionSecrets,
) {
    for ((path, id), size) in blobs {
        review.inspect(&snapshot.repo, path, *id, *size, true);
    }
}

pub(super) fn create_series(
    snapshot: &Snapshot,
    signature: &git2::Signature<'_>,
    units: &[Unit],
    drafts: &[Draft],
    mut review: Option<&mut CompositionSecrets>,
) -> Result<Vec<Oid>, String> {
    let mut series = Series::new(snapshot)?;
    let mut dependencies = dependencies::Check::new(
        &snapshot.repo,
        snapshot.head,
        snapshot.tree,
        &snapshot.changes,
    )?;
    let mut parent = snapshot.head;
    let mut commits = Vec::new();
    let mut tree = None;
    let mut reviewed = HashSet::new();
    for (number, draft) in drafts.iter().enumerate() {
        let (id, mut intermediate) = series.advance(units, &draft.units)?;
        let current_tree = snapshot.repo.find_tree(id).map_err(git_err)?;
        if let Some(problem) = dependencies.verify(&current_tree) {
            return Err(format!(
                "Commit {}: {problem} Keep the changes together or move the dependency earlier.",
                number + 1
            ));
        }
        if dependencies.limited && drafts.len() > 1 {
            return Err("The dependency review reached its limit. Keep these changes in one commit or stage a smaller batch.".into());
        }
        if let Some(review) = review.as_deref_mut() {
            intermediate.retain(|key, _| reviewed.insert(key.clone()));
            review_intermediate(snapshot, &intermediate, review);
        }
        let commit = parent
            .map(|oid| snapshot.repo.find_commit(oid))
            .transpose()
            .map_err(git_err)?;
        if commit.as_ref().is_some_and(|commit| commit.tree_id() == id) {
            return Err("The plan contains an empty commit.".into());
        }
        let body = draft.description.trim();
        let message = if body.is_empty() {
            draft.subject.trim().to_owned()
        } else {
            format!("{}\n\n{body}", draft.subject.trim())
        };
        let commit = snapshot
            .repo
            .commit(
                None,
                signature,
                signature,
                &message,
                &current_tree,
                &commit.iter().collect::<Vec<_>>(),
            )
            .map_err(git_err)?;
        tree = Some(id);
        parent = Some(commit);
        commits.push(commit);
    }
    if tree != Some(snapshot.tree) {
        return Err(
            "The commit series does not match the reviewed Git tree. Nothing was committed.".into(),
        );
    }
    Ok(commits)
}
