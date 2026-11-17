mod chunks;
mod dependencies;
mod preview;
mod publication;
mod rules;
mod series;
mod snapshot;
mod verification;

#[cfg(test)]
mod tests;

use std::{collections::HashSet, path::Path, sync::Mutex};

use git2::{ErrorCode, Oid, Repository, RepositoryState};
use serde::Serialize;

use super::secret_service::{CompositionSecrets, SecretScan};
use crate::models::commit_composition::{ChangePreview, Draft, Group, Unit};
use crate::repositories::git_repository::GitRepository as Git;
use series::{create_series, review_intermediate, Intermediate, Series};
use snapshot::{relative, Snapshot};

const MAX_GROUPS: usize = 40;
const MAX_INDEX: u64 = 16 * 1024 * 1024;
const PATCH_FILE_LIMIT: usize = 2 * 1024 * 1024;
const STALE: &str = "Working changes or HEAD changed. Generate a new plan before committing.";
static COMPOSER: Mutex<()> = Mutex::new(());

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Composition {
    branch: String,
    snapshot: String,
    changes: Vec<Change>,
    units: Vec<Unit>,
    groups: Vec<Group>,
    max_groups: usize,
    review: SecretScan,
    limited: bool,
    requires_build_verification: bool,
    dependency_notice: Option<String>,
}

#[derive(Serialize)]
pub struct BuildPlan {
    groups: Vec<Group>,
    review: SecretScan,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Change {
    path: String,
    old_path: Option<String>,
    status: &'static str,
    staged: bool,
    partial: bool,
    added: usize,
    removed: usize,
    kind: rules::Kind,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk: Option<chunks::Chunk>,
    #[serde(skip)]
    new: Option<(Oid, u32)>,
    #[serde(skip)]
    old: Option<(Oid, u32)>,
    #[serde(skip)]
    hints: rules::Hints,
}

fn git_err(error: git2::Error) -> String {
    error.message().to_owned()
}

fn head(repo: &Repository) -> Result<(String, Option<Oid>), String> {
    if repo.is_bare()
        || repo.state() != RepositoryState::Clean
        || super::interactive_rebase::is_active(repo)
    {
        return Err("Finish the current Git operation before composing commits.".into());
    }
    let reference = repo.find_reference("HEAD").map_err(git_err)?;
    let name = reference
        .symbolic_target()
        .filter(|name| name.starts_with("refs/heads/"))
        .ok_or("Check out a local branch before composing commits.")?
        .to_owned();
    let oid = match repo.find_reference(&name) {
        Ok(branch) => Some(branch.target().ok_or("Invalid local branch.")?),
        Err(error) if error.code() == ErrorCode::NotFound => None,
        Err(error) => return Err(git_err(error)),
    };
    Ok((name, oid))
}

pub fn plan(path: &str, include_secrets: bool) -> Result<Composition, String> {
    let _guard = COMPOSER
        .lock()
        .map_err(|_| "Commit composer is unavailable.")?;
    let snapshot = Snapshot::build(&Git::open(path)?, true)?;
    let fingerprint = snapshot.fingerprint(include_secrets);
    let (units, mut groups) = rules::compose(&snapshot.changes);
    let mut review = CompositionSecrets::new(&fingerprint, include_secrets);
    snapshot.review(&mut review)?;
    let mut dependencies = dependencies::Check::new(
        &snapshot.repo,
        snapshot.head,
        snapshot.tree,
        &snapshot.changes,
    )?;
    let target = snapshot.repo.find_tree(snapshot.tree).map_err(git_err)?;
    if let Some(problem) = dependencies.verify(&target) {
        return Err(format!(
            "The proposed final state is missing a local dependency. {problem}"
        ));
    }
    let (mut dependency_notice, intermediate) = arrange_groups(
        &snapshot,
        &units,
        &mut groups,
        &mut dependencies,
        include_secrets,
    )?;
    if dependencies.limited {
        let warning = "Some source files exceeded the static dependency analysis limits. Enable build verification before creating multiple commits.";
        dependency_notice = Some(match dependency_notice {
            Some(notice) => format!("{notice} {warning}"),
            None => warning.into(),
        });
    }
    review_intermediate(&snapshot, &intermediate, &mut review);
    let movable = rules::movable_units(&snapshot.changes);
    let mut by_change = vec![0; snapshot.changes.len()];
    for unit in &movable {
        for change in &unit.changes {
            by_change[*change] = unit.id;
        }
    }
    // The suggested groups may be edited, but Git tree replacements cannot be split.
    for group in &mut groups {
        let mut seen = HashSet::new();
        group.units = group
            .units
            .iter()
            .flat_map(|id| &units[*id].changes)
            .map(|change| by_change[*change])
            .filter(|id| seen.insert(*id))
            .collect();
    }
    Ok(Composition {
        branch: snapshot.branch["refs/heads/".len()..].into(),
        snapshot: fingerprint,
        changes: snapshot.changes,
        units: movable,
        groups,
        max_groups: MAX_GROUPS,
        review: review.finish(),
        limited: snapshot.limited,
        requires_build_verification: dependencies.limited,
        dependency_notice,
    })
}

pub fn preview_change(
    path: &str,
    expected: &str,
    include_secrets: bool,
    change: usize,
) -> Result<ChangePreview, String> {
    let snapshot = Snapshot::build(&Git::open(path)?, false)?;
    if snapshot.fingerprint(include_secrets) != expected {
        return Err(STALE.into());
    }
    preview::file(&snapshot.repo, snapshot.changes.get(change).ok_or(STALE)?)
}

fn arrange_groups(
    snapshot: &Snapshot,
    units: &[Unit],
    groups: &mut Vec<Group>,
    dependencies: &mut dependencies::Check<'_>,
    inspect_intermediate: bool,
) -> Result<(Option<String>, Intermediate), String> {
    let mut notice = None;
    loop {
        let mut series = Series::new(snapshot)?;
        let mut pieces = Intermediate::new();
        let mut failed = None;
        let mut last = None;
        for (position, group) in groups.iter().enumerate() {
            let (id, staged) = series.advance(units, &group.units)?;
            last = Some(id);
            if let Some(problem) =
                dependencies.verify(&snapshot.repo.find_tree(id).map_err(git_err)?)
            {
                failed = Some((position, problem));
                break;
            }
            if inspect_intermediate {
                pieces.extend(staged);
            }
        }
        let Some((position, problem)) = failed else {
            if last != Some(snapshot.tree) {
                return Err(
                    "The proposed hunks do not reproduce the final reviewed Git tree.".into(),
                );
            }
            return Ok((notice, pieces));
        };
        notice.get_or_insert_with(|| format!("Grouped dependent changes. {problem}"));
        let provider = groups
            .iter()
            .enumerate()
            .skip(position + 1)
            .find(|(_, group)| {
                group
                    .units
                    .iter()
                    .flat_map(|id| &units[*id].changes)
                    .any(|id| problem.is_provider(&snapshot.changes[*id]))
            })
            .map(|(index, _)| index);
        if let Some(provider) = provider {
            let dependency = groups.remove(provider);
            let selected = groups[position]
                .units
                .iter()
                .copied()
                .chain(dependency.units)
                .collect();
            groups[position] = rules::combined_with_reason(
                &snapshot.changes,
                units,
                selected,
                "Keep these changes together so local references still resolve at this point in the series.",
            );
        } else if position + 1 < groups.len() {
            let following = groups.remove(position + 1);
            let selected = groups[position]
                .units
                .iter()
                .copied()
                .chain(following.units)
                .collect();
            groups[position] = rules::combined_with_reason(
                &snapshot.changes,
                units,
                selected,
                "Keep these changes together so local references still resolve at this point in the series.",
            );
        } else {
            return Err(format!(
                "The composed changes do not pass dependency review. {problem}"
            ));
        }
    }
}

fn validate_drafts(units: &[Unit], drafts: &[Draft]) -> Result<(), String> {
    let mut seen = HashSet::new();
    if drafts.is_empty()
        || drafts.len() > MAX_GROUPS
        || drafts.iter().any(|draft| {
            draft.units.is_empty()
                || draft.units.len() > units.len()
                || draft
                    .units
                    .iter()
                    .any(|id| *id >= units.len() || !seen.insert(*id))
                || draft.subject.trim().is_empty()
                || draft.subject.len() > 288
                || draft.subject.chars().any(char::is_control)
                || draft.description.len() > 32 * 1024
                || draft.description.contains('\0')
        })
        || seen.len() != units.len()
    {
        return Err(
            "Each change set must appear exactly once and each commit needs a valid subject."
                .into(),
        );
    }
    Ok(())
}

pub fn find_buildable_groups(
    path: &str,
    expected: &str,
    drafts: &[Draft],
    include_secrets: bool,
    command: &str,
) -> Result<BuildPlan, String> {
    use verification::CheckError;

    let _guard = COMPOSER
        .lock()
        .map_err(|_| "Commit composer is unavailable.")?;
    let snapshot = Snapshot::build(&Git::open(path)?, true)?;
    if snapshot.fingerprint(include_secrets) != expected {
        return Err(STALE.into());
    }
    let units = rules::movable_units(&snapshot.changes);
    validate_drafts(&units, drafts)?;
    let verify = verification::Verification::new(command)?;
    let mut dependencies = dependencies::Check::new(
        &snapshot.repo,
        snapshot.head,
        snapshot.tree,
        &snapshot.changes,
    )?;
    let mut state = Series::new(&snapshot)?;
    let mut groups = Vec::new();
    let mut pieces = Intermediate::new();
    let mut pending = Vec::new();
    let mut start = 0;
    let mut last = None;
    for (index, draft) in drafts.iter().enumerate() {
        pending.extend(draft.units.iter().copied());
        let (id, staged) = state.advance(&units, &draft.units)?;
        last = Some(id);
        let number = groups.len() + 1;
        let tree = snapshot.repo.find_tree(id).map_err(git_err)?;
        let problem = dependencies.verify(&tree);
        if index + 1 < drafts.len() && problem.is_some() {
            continue;
        }
        if let Some(problem) = problem {
            return Err(format!(
                "The final state does not pass dependency review: {problem}"
            ));
        }
        match verify.check(&snapshot.repo, &tree, number) {
            Ok(()) => {}
            Err(CheckError::Rejected(_)) if index + 1 < drafts.len() => continue,
            Err(CheckError::Rejected(message)) => {
                return Err(format!("The final state does not build with this command. Regrouping cannot fix the final tree.\n{message}"));
            }
            Err(error) => return Err(error.message()),
        }
        if include_secrets {
            // A chunk that was selected in a failed prefix may still be present in this tree.
            for file in pending.iter().flat_map(|unit| &units[*unit].changes) {
                let file = &snapshot.changes[*file];
                if file.chunk.is_some() {
                    let entry = tree.get_path(Path::new(&file.path)).map_err(git_err)?;
                    if file.new.is_some_and(|(final_id, _)| entry.id() != final_id) {
                        let (size, _) = snapshot
                            .repo
                            .odb()
                            .and_then(|odb| odb.read_header(entry.id()))
                            .map_err(git_err)?;
                        pieces.insert((file.path.clone(), entry.id()), size);
                    }
                }
            }
            pieces.extend(staged);
        }
        let selected = std::mem::take(&mut pending);
        let group = if index == start {
            Group {
                subject: draft.subject.clone(),
                description: draft.description.clone(),
                units: selected,
            }
        } else {
            rules::combined_with_reason(
                &snapshot.changes,
                &units,
                selected,
                "Keep these changes together so the intermediate snapshot passes the configured checks.",
            )
        };
        groups.push(group);
        start = index + 1;
    }
    if last != Some(snapshot.tree) {
        return Err("The proposed hunks do not reproduce the final reviewed Git tree.".into());
    }
    if Snapshot::build(&Git::open(path)?, false)?.fingerprint(include_secrets) != expected {
        return Err(STALE.into());
    }
    let mut review = CompositionSecrets::new(expected, include_secrets);
    snapshot.review(&mut review)?;
    review_intermediate(&snapshot, &pieces, &mut review);
    Ok(BuildPlan {
        groups,
        review: review.finish(),
    })
}

#[cfg(test)]
fn apply(
    path: &str,
    expected: &str,
    drafts: &[Draft],
    include_secrets: bool,
    approved: bool,
) -> Result<Vec<String>, String> {
    apply_with_verification(path, expected, drafts, include_secrets, approved, None)
}

pub fn apply_with_verification(
    path: &str,
    expected: &str,
    drafts: &[Draft],
    include_secrets: bool,
    approved: bool,
    build_command: Option<&str>,
) -> Result<Vec<String>, String> {
    let _guard = COMPOSER
        .lock()
        .map_err(|_| "Commit composer is unavailable.")?;
    let repo = Git::open(path)?;
    let snapshot = Snapshot::build(&repo, false)?;
    if snapshot.fingerprint(include_secrets) != expected {
        return Err(STALE.into());
    }
    let units = rules::movable_units(&snapshot.changes);
    validate_drafts(&units, drafts)?;
    let mut review = CompositionSecrets::new(expected, include_secrets);
    snapshot.review(&mut review)?;
    let signature = repo.signature().map_err(git_err)?;
    let commits = create_series(
        &snapshot,
        &signature,
        &units,
        drafts,
        include_secrets.then_some(&mut review),
        build_command.is_some(),
    )?;
    let review = review.finish();
    if !review.large_files.is_empty() {
        return Err(
            "The plan contains files over 100 MiB. Stage them with Git LFS Manager first.".into(),
        );
    }
    if (!review.findings.is_empty() || !review.skipped.is_empty()) && !approved {
        return Err("Review the secret scan warnings before confirming.".into());
    }
    if let Some(command) = build_command {
        let verify = verification::Verification::new(command)?;
        for (i, commit) in commits.iter().enumerate() {
            let tree = snapshot
                .repo
                .find_commit(*commit)
                .and_then(|commit| commit.tree())
                .map_err(git_err)?;
            verify
                .check(&snapshot.repo, &tree, i + 1)
                .map_err(|error| format!("{} No commits were published.", error.message()))?;
        }
        // The working copy can change while an external build is running.
        if Snapshot::build(&Git::open(path)?, false)?.fingerprint(include_secrets) != expected {
            return Err(STALE.into());
        }
    }

    publication::publish(&repo, &snapshot, &signature, &commits)?;
    Ok(commits.iter().map(Oid::to_string).collect())
}
