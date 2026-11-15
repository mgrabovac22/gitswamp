use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use git2::{BranchType, DiffOptions, Oid, Reference, Repository, RepositoryState, StatusOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::interactive_rebase;
use super::worktree_service::ensure_branch_available;
use crate::repositories::git_repository::GitRepository as Git;

const REF_LIMIT: usize = 3000;
const STALE: &str = "The repository changed. Preview the workflow again.";
static WORKFLOW: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Strategy {
    Manual,
    Gitflow,
    Trunk,
    Github,
    Swamp,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    strategy: Strategy,
    primary: String,
    development: String,
    feature: String,
    fix: String,
    release: String,
    hotfix: String,
    support: String,
    tag: String,
    short_branches: bool,
    release_branches: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    key: String,
    current: Option<String>,
    local: Vec<String>,
    remote: Vec<String>,
    primary: String,
    development: Option<String>,
    suggestion: Option<Strategy>,
    minimal: bool,
    unborn: bool,
    blocked: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Feature,
    Fix,
    Release,
    Hotfix,
    Support,
    PreparePrimary,
    PrepareDevelopment,
    Finish,
    Cleanup,
    Tag,
    Handoff,
}

#[derive(Deserialize, Serialize)]
pub struct Request {
    action: Action,
    name: String,
    source: String,
    cleanup: bool,
    #[serde(default)]
    remote: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    token: String,
    steps: Vec<String>,
    comparisons: Vec<Comparison>,
    handoff: Option<Handoff>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Comparison {
    target: String,
    ahead: usize,
    behind: usize,
    conflicts: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    conflict_paths: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Handoff {
    source: String,
    target: String,
    published: bool,
    checkpoint: Option<Checkpoint>,
    remote: Option<String>,
    target_fetched: bool,
    source_sync: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Checkpoint {
    changed_paths: usize,
    test_paths: usize,
    review_areas: Vec<&'static str>,
    commits: Vec<CommitExcerpt>,
}

#[derive(Serialize)]
struct CommitExcerpt {
    sha: String,
    subject: String,
}

#[derive(Serialize)]
pub struct Outcome {
    status: &'static str,
    completed: Vec<String>,
    remaining: Vec<String>,
    error: Option<String>,
}

#[derive(Serialize)]
enum Step {
    Branch {
        name: String,
        from: String,
        revision: String,
        switch: bool,
    },
    Merge {
        source: String,
        target: String,
        incoming: String,
        before: String,
        fast_forward: bool,
    },
    Tag {
        name: String,
        target: String,
        revision: String,
    },
    Delete {
        name: String,
        revision: String,
        targets: Vec<String>,
    },
}

impl Step {
    fn title(&self) -> String {
        match self {
            Self::Branch {
                name, from, switch, ..
            } => {
                let verb = if *switch {
                    "Create and switch to"
                } else {
                    "Create"
                };
                format!("{verb} {name} from {from}")
            }
            Self::Merge {
                source,
                target,
                fast_forward,
                ..
            } => {
                let verb = if *fast_forward {
                    "Fast-forward"
                } else {
                    "Merge into"
                };
                if *fast_forward {
                    format!("{verb} {target} to {source}")
                } else {
                    format!("Merge {source} into {target}")
                }
            }
            Self::Tag {
                name,
                target,
                revision,
            } if !revision.is_empty() => {
                format!("Create local tag {name} at {target} ({})", &revision[..8])
            }
            Self::Tag { name, target, .. } => {
                format!("Tag the resulting {target} commit as {name}")
            }
            Self::Delete { name, .. } => format!("Delete local branch {name}"),
        }
    }
}

fn git_error(error: git2::Error) -> String {
    error.message().to_owned()
}

fn branch_ref(name: &str) -> String {
    format!("refs/heads/{name}")
}

fn valid_branch(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 160
        && !name.starts_with('-')
        && name != "HEAD"
        && Reference::is_valid_name(&branch_ref(name))
}

pub fn validate(config: &Config) -> Result<(), String> {
    if config.strategy == Strategy::Manual {
        return Ok(());
    }
    if !valid_branch(&config.primary) {
        return Err("Enter a valid primary branch name.".into());
    }
    if config.strategy == Strategy::Gitflow
        && (!valid_branch(&config.development) || config.development == config.primary)
    {
        return Err("Gitflow requires a different, valid development branch.".into());
    }
    let prefixes = match config.strategy {
        Strategy::Gitflow => vec![
            &config.feature,
            &config.release,
            &config.hotfix,
            &config.support,
        ],
        Strategy::Trunk => {
            let mut prefixes = Vec::new();
            if config.short_branches {
                prefixes.extend([&config.feature, &config.fix]);
            }
            if config.release_branches {
                prefixes.push(&config.release);
            }
            prefixes
        }
        Strategy::Github | Strategy::Swamp => vec![&config.feature, &config.fix],
        Strategy::Manual => unreachable!(),
    };
    for (index, prefix) in prefixes.iter().enumerate() {
        if prefix.len() > 80
            || !prefix.ends_with('/')
            || !valid_branch(&format!("{prefix}example"))
            || config.primary.starts_with(prefix.as_str())
            || config.strategy == Strategy::Gitflow
                && config.development.starts_with(prefix.as_str())
            || prefixes[..index].iter().any(|other| {
                prefix.starts_with(other.as_str()) || other.starts_with(prefix.as_str())
            })
        {
            return Err(
                "Use distinct branch prefixes ending in /, separate from the primary branches."
                    .into(),
            );
        }
    }
    if config.tag.len() > 80
        || format!("{}1.0.0", config.tag).starts_with('-')
        || !Reference::is_valid_name(&format!("refs/tags/{}1.0.0", config.tag))
    {
        return Err("Enter a valid version tag prefix.".into());
    }
    Ok(())
}

fn current_branch(repo: &Repository) -> Option<String> {
    repo.find_reference("HEAD")
        .ok()?
        .symbolic_target()?
        .strip_prefix("refs/heads/")
        .map(str::to_owned)
}

fn revision(repo: &Repository, reference: &str) -> Result<Oid, String> {
    repo.find_reference(reference)
        .and_then(|r| r.peel_to_commit())
        .map(|c| c.id())
        .map_err(|_| format!("Missing commit for {reference}."))
}

fn operation(repo: &Repository) -> Option<String> {
    if repo.state() != RepositoryState::Clean || interactive_rebase::is_active(repo) {
        return Some("Finish or abort the current Git operation before using the workflow.".into());
    }
    None
}

fn check_clean(repo: &Repository) -> Result<(), String> {
    if let Some(error) = operation(repo) {
        return Err(error);
    }
    if current_branch(repo).is_none() {
        return Err("Check out a local branch first.".into());
    }
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .include_ignored(false)
        .include_unmodified(false);
    if !repo
        .statuses(Some(&mut options))
        .map_err(git_error)?
        .is_empty()
    {
        return Err("Commit or stash local changes before making workflow changes. No changes were discarded.".into());
    }
    Ok(())
}

fn branch_names(repo: &Repository) -> Result<(Vec<String>, BTreeMap<String, Vec<String>>), String> {
    let mut local = Vec::new();
    let mut remote: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let configured = repo.remotes().map_err(git_error)?;
    let configured: Vec<_> = configured.iter().flatten().collect();
    for item in repo.branches(None).map_err(git_error)?.take(REF_LIMIT) {
        let (branch, kind) = item.map_err(git_error)?;
        let Some(name) = branch.name().map_err(git_error)? else {
            continue;
        };
        if kind == BranchType::Local {
            local.push(name.to_owned());
        } else if branch.get().symbolic_target().is_none() {
            let matching_remote = configured
                .iter()
                .filter(|remote| {
                    name.strip_prefix(**remote)
                        .is_some_and(|tail| tail.starts_with('/'))
                })
                .max_by_key(|remote| remote.len());
            if let Some(remote_name) = matching_remote {
                let short = &name[remote_name.len() + 1..];
                remote
                    .entry(short.into())
                    .or_default()
                    .push(format!("refs/remotes/{name}"));
            }
        }
    }
    local.sort_unstable();
    Ok((local, remote))
}

pub fn inspect(path: &str) -> Result<Inspection, String> {
    let repo = Git::open(path)?;
    let key = Git::git_cli(
        path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let (local, remote) = branch_names(&repo)?;
    let current = current_branch(&repo);
    let origin = repo
        .find_reference("refs/remotes/origin/HEAD")
        .ok()
        .and_then(|r| {
            r.symbolic_target()
                .and_then(|target| target.strip_prefix("refs/remotes/origin/"))
                .map(str::to_owned)
        });
    let names: BTreeSet<_> = local
        .iter()
        .chain(remote.keys())
        .map(String::as_str)
        .collect();
    let development = ["develop", "dev", "development"]
        .into_iter()
        .find(|name| names.contains(name))
        .map(str::to_owned);
    let primary = origin
        .filter(|name| names.contains(name.as_str()))
        .or_else(|| {
            ["main", "master", "trunk"]
                .into_iter()
                .find(|name| names.contains(name))
                .map(str::to_owned)
        })
        .or_else(|| current.clone())
        .unwrap_or_else(|| "main".into());
    let gitflow = development.as_ref().is_some_and(|name| *name != primary)
        && (names.contains("main")
            || names.contains("master")
            || names.iter().any(|name| name.starts_with("release/")));
    let suggestion = if gitflow {
        Some(Strategy::Gitflow)
    } else if names
        .iter()
        .any(|name| name.starts_with("change/") || name.starts_with("repair/"))
    {
        Some(Strategy::Swamp)
    } else if primary == "trunk" || names.iter().any(|name| name.starts_with("bugfix/")) {
        Some(Strategy::Trunk)
    } else if names
        .iter()
        .any(|name| name.starts_with("feature/") || name.starts_with("fix/"))
    {
        Some(Strategy::Github)
    } else {
        None
    };
    let tip = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let unborn = tip.is_none() && current.is_some();
    let minimal =
        !repo.is_shallow() && names.len() <= 1 && tip.is_some_and(|tip| tip.parent_count() == 0);
    let blocked = operation(&repo).or_else(|| {
        if unborn {
            Some("Make the first commit before starting a workflow.".into())
        } else if current.is_none() {
            Some("Check out a local branch to use workflow actions.".into())
        } else {
            None
        }
    });
    Ok(Inspection {
        key,
        current,
        local,
        remote: remote.into_keys().collect(),
        primary,
        development,
        suggestion,
        minimal,
        unborn,
        blocked,
    })
}

fn contains(repo: &Repository, target: Oid, source: Oid) -> Result<bool, String> {
    Ok(target == source
        || repo
            .graph_descendant_of(target, source)
            .map_err(git_error)?)
}

fn targets<'a>(config: &'a Config, source: &str) -> Result<Vec<&'a str>, String> {
    let valid = |prefix: &str| source.starts_with(prefix) && source.len() > prefix.len();
    match config.strategy {
        Strategy::Gitflow if valid(&config.feature) => Ok(vec![&config.development]),
        Strategy::Gitflow if valid(&config.release) || valid(&config.hotfix) => {
            Ok(vec![&config.primary, &config.development])
        }
        Strategy::Trunk
            if config.short_branches && (valid(&config.feature) || valid(&config.fix)) =>
        {
            Ok(vec![&config.primary])
        }
        Strategy::Swamp | Strategy::Github if valid(&config.feature) || valid(&config.fix) => {
            Ok(vec![&config.primary])
        }
        _ => Err("This branch is not an integration source for the selected strategy.".into()),
    }
}

fn target_tip(repo: &Repository, name: &str) -> Result<Oid, String> {
    revision(repo, &branch_ref(name))
}

fn is_published(repo: &Repository, source: &str) -> Result<bool, String> {
    if repo
        .find_branch(source, BranchType::Local)
        .ok()
        .and_then(|branch| branch.upstream().ok())
        .is_some()
    {
        return Ok(true);
    }
    let remotes = repo.remotes().map_err(git_error)?;
    for (i, item) in repo
        .branches(Some(BranchType::Remote))
        .map_err(git_error)?
        .enumerate()
    {
        if i >= REF_LIMIT {
            return Err("Too many remote refs to verify safe branch deletion.".into());
        }
        let (branch, _) = item.map_err(git_error)?;
        if branch.name().map_err(git_error)?.is_some_and(|name| {
            remotes.iter().flatten().any(|remote| {
                name.strip_prefix(remote)
                    .and_then(|tail| tail.strip_prefix('/'))
                    == Some(source)
            })
        }) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn check_deletion(
    repo: &Repository,
    source: &str,
    targets: &[String],
    incoming: Oid,
) -> Result<(), String> {
    if target_tip(repo, source)? != incoming {
        return Err(STALE.into());
    }
    if is_published(repo, source)? {
        return Err(
            "This branch has a remote counterpart. The workflow will not delete it.".into(),
        );
    }
    for target in targets {
        if !contains(repo, target_tip(repo, target)?, incoming)? {
            return Err(format!(
                "{source} is not fully integrated into {target}. Nothing was deleted."
            ));
        }
    }
    Ok(())
}

fn compare(
    repo: &Repository,
    source: Oid,
    target: Oid,
    name: &str,
    conflicts: bool,
    show_paths: bool,
) -> Result<Comparison, String> {
    let (ahead, behind) = repo.graph_ahead_behind(source, target).map_err(git_error)?;
    let mut conflict_paths = Vec::new();
    let conflicts = if conflicts && ahead != 0 && behind != 0 {
        let ours = repo.find_commit(target).map_err(git_error)?;
        let theirs = repo.find_commit(source).map_err(git_error)?;
        if repo.merge_base(source, target).is_err() {
            None
        } else {
            repo.merge_commits(&ours, &theirs, None).ok().map(|index| {
                if show_paths {
                    if let Ok(entries) = index.conflicts() {
                        conflict_paths.extend(entries.take(8).filter_map(|conflict| {
                            let conflict = conflict.ok()?;
                            let entry = conflict.our.or(conflict.their).or(conflict.ancestor)?;
                            Some(
                                String::from_utf8_lossy(&entry.path)
                                    .chars()
                                    .take(320)
                                    .collect(),
                            )
                        }));
                    }
                }
                index.has_conflicts()
            })
        }
    } else if conflicts {
        Some(false)
    } else {
        None
    };
    Ok(Comparison {
        target: name.into(),
        ahead,
        behind,
        conflicts,
        conflict_paths,
    })
}

fn checkpoint(repo: &Repository, incoming: Oid, target: Oid) -> Option<Checkpoint> {
    let base = repo.merge_base(incoming, target).ok()?;
    let base = repo.find_commit(base).ok()?.tree().ok()?;
    let incoming_tree = repo.find_commit(incoming).ok()?.tree().ok()?;
    let diff = repo
        .diff_tree_to_tree(
            Some(&base),
            Some(&incoming_tree),
            Some(DiffOptions::new().skip_binary_check(true)),
        )
        .ok()?;
    let mut review_areas = BTreeSet::new();
    let test_paths = diff
        .deltas()
        .filter(|delta| {
            delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .is_some_and(|path| {
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if path.starts_with(".github/workflows")
                        || matches!(
                            name.as_str(),
                            ".gitlab-ci.yml" | "jenkinsfile" | "azure-pipelines.yml"
                        )
                    {
                        review_areas.insert("CI and deployment");
                    }
                    if path.components().any(|part| {
                        matches!(
                            part.as_os_str().to_str(),
                            Some("migrations" | "alembic" | "flyway" | "liquibase")
                        )
                    }) || matches!(name.as_str(), "schema.prisma" | "schema.sql")
                    {
                        review_areas.insert("Schema and migrations");
                    }
                    if matches!(
                        name.as_str(),
                        "package.json"
                            | "package-lock.json"
                            | "pnpm-lock.yaml"
                            | "yarn.lock"
                            | "cargo.toml"
                            | "cargo.lock"
                            | "go.mod"
                            | "go.sum"
                            | "pyproject.toml"
                            | "uv.lock"
                            | "pom.xml"
                            | "build.gradle"
                            | "build.gradle.kts"
                    ) {
                        review_areas.insert("Dependencies and build");
                    }
                    path.components().any(|part| {
                        matches!(
                            part.as_os_str().to_str(),
                            Some("tests" | "test" | "spec" | "__tests__" | "androidTest")
                        )
                    }) || name.contains(".test.")
                        || name.contains(".spec.")
                        || name.starts_with("test_")
                        || name.ends_with("_test.go")
                        || name.ends_with("tests.cs")
                })
        })
        .count();
    Some(Checkpoint {
        changed_paths: diff.deltas().len(),
        test_paths,
        review_areas: review_areas.into_iter().collect(),
        commits: recent_commits(repo, incoming, target),
    })
}

fn recent_commits(repo: &Repository, incoming: Oid, target: Oid) -> Vec<CommitExcerpt> {
    let walk = (|| {
        let mut walk = repo.revwalk()?;
        walk.set_sorting(git2::Sort::TIME)?;
        walk.push(incoming)?;
        walk.hide(target)?;
        Ok::<_, git2::Error>(walk)
    })();
    walk.into_iter()
        .flatten()
        .take(6)
        .filter_map(|oid| {
            let commit = repo.find_commit(oid.ok()?).ok()?;
            let bytes = commit.message_bytes();
            let message = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
            let subject = message
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .chars()
                .filter(|character| !character.is_control())
                .take(120)
                .collect::<String>();
            Some(CommitExcerpt {
                sha: commit.id().to_string()[..8].to_owned(),
                subject: if subject.is_empty() {
                    "Untitled commit".into()
                } else {
                    subject
                },
            })
        })
        .collect()
}

fn tag_step(
    repo: &Repository,
    config: &Config,
    version: &str,
    current: Oid,
    target: &str,
    after_merge: bool,
) -> Result<Option<Step>, String> {
    let name = format!("{}{version}", config.tag);
    if version.is_empty()
        || name.len() > 160
        || name.starts_with('-')
        || !Reference::is_valid_name(&format!("refs/tags/{name}"))
    {
        return Err("Enter a valid release version or tag suffix.".into());
    }
    if let Ok(existing) = repo.find_reference(&format!("refs/tags/{name}")) {
        if !after_merge
            && existing
                .peel_to_commit()
                .is_ok_and(|commit| commit.id() == current)
        {
            return Ok(None);
        }
        return Err(format!(
            "Tag {name} already exists at a different revision. It will not be replaced."
        ));
    }
    Ok(Some(Step::Tag {
        name,
        target: target.into(),
        revision: if after_merge {
            String::new()
        } else {
            current.to_string()
        },
    }))
}

fn resolve(
    path: &str,
    config: &Config,
    request: &Request,
    analyze: bool,
) -> Result<(Plan, Vec<Step>), String> {
    validate(config)?;
    if config.strategy == Strategy::Manual {
        return Err("Select a branching strategy first.".into());
    }
    let repo = Git::open(path)?;
    let current = current_branch(&repo).ok_or("Check out a local branch first.")?;
    let head = target_tip(&repo, &current)?;
    if matches!(request.action, Action::Handoff) {
        if let Some(error) = operation(&repo) {
            return Err(error);
        }
    } else {
        check_clean(&repo)?;
    }

    let mut steps = Vec::new();
    let mut comparisons = Vec::new();
    let mut handoff = None;
    let mut pins = Vec::new();
    let start = match request.action {
        Action::Feature if config.strategy != Strategy::Trunk || config.short_branches => Some((
            &config.feature,
            if config.strategy == Strategy::Gitflow {
                &config.development
            } else {
                &config.primary
            },
        )),
        Action::Fix
            if matches!(config.strategy, Strategy::Github | Strategy::Swamp)
                || config.strategy == Strategy::Trunk && config.short_branches =>
        {
            Some((&config.fix, &config.primary))
        }
        Action::Release if config.strategy == Strategy::Gitflow => {
            Some((&config.release, &config.development))
        }
        Action::Release if config.strategy == Strategy::Trunk && config.release_branches => {
            Some((&config.release, &config.primary))
        }
        Action::Hotfix if config.strategy == Strategy::Gitflow => {
            Some((&config.hotfix, &config.primary))
        }
        Action::Support if config.strategy == Strategy::Gitflow => {
            Some((&config.support, &config.primary))
        }
        _ => None,
    };
    if let Some((prefix, base)) = start {
        let name = format!("{prefix}{}", request.name);
        if request.name.trim().is_empty() || !valid_branch(&name) {
            return Err("Enter a valid branch name.".into());
        }
        let (_, remote) = branch_names(&repo)?;
        if repo.find_branch(&name, BranchType::Local).is_ok() || remote.contains_key(&name) {
            return Err(format!(
                "Branch {name} already exists locally or on a tracked remote."
            ));
        }
        let (from, revision) =
            if matches!(request.action, Action::Support) && !request.source.is_empty() {
                let reference = format!("refs/tags/{}", request.source);
                if request.source.len() > 160 || !Reference::is_valid_name(&reference) {
                    return Err("Enter a valid existing release tag.".into());
                }
                (reference.clone(), revision(&repo, &reference)?)
            } else {
                (base.clone(), target_tip(&repo, base)?)
            };
        steps.push(Step::Branch {
            name,
            from,
            revision: revision.to_string(),
            switch: true,
        });
    } else {
        match request.action {
            Action::PreparePrimary | Action::PrepareDevelopment => {
                let name = if matches!(request.action, Action::PreparePrimary) {
                    &config.primary
                } else if config.strategy == Strategy::Gitflow {
                    &config.development
                } else {
                    return Err("This workflow has no development branch.".into());
                };
                if repo.find_branch(name, BranchType::Local).is_ok() {
                    return Err(format!("{name} already exists."));
                }
                let (_, remote) = branch_names(&repo)?;
                let (from, oid) = match remote.get(name).map(Vec::as_slice) {
                    Some([reference]) => (reference.clone(), revision(&repo, reference)?),
                    Some(_) => return Err(format!("{name} exists on multiple remotes. Check out the intended branch manually.")),
                    None if matches!(request.action, Action::PrepareDevelopment) => (config.primary.clone(), target_tip(&repo, &config.primary)?),
                    None => return Err(format!("No tracking branch for {name}. Choose an existing primary branch or create it manually.")),
                };
                steps.push(Step::Branch {
                    name: name.clone(),
                    from,
                    revision: oid.to_string(),
                    switch: false,
                });
            }
            Action::Finish | Action::Cleanup | Action::Handoff => {
                let source = &request.source;
                if !valid_branch(source) {
                    return Err("Select a local source branch.".into());
                }
                let destinations = targets(config, source)?;
                let incoming = target_tip(&repo, source)?;
                pins.push(incoming.to_string());
                let for_pr = matches!(config.strategy, Strategy::Swamp | Strategy::Github);
                if matches!(request.action, Action::Finish) && for_pr {
                    return Err("Use a pull request for this workflow.".into());
                }
                if matches!(request.action, Action::Handoff) && !for_pr {
                    return Err("This workflow has no pull request handoff action.".into());
                }
                let mut primary_after_merge = false;
                let is_handoff = matches!(request.action, Action::Handoff);
                let remote = request
                    .remote
                    .as_deref()
                    .filter(|name| !name.is_empty() && name.len() <= 160);
                let tracked = |name: &str| {
                    remote.and_then(|remote| {
                        revision(&repo, &format!("refs/remotes/{remote}/{name}")).ok()
                    })
                };
                for target in &destinations {
                    let before = if is_handoff {
                        tracked(target)
                            .map(Ok)
                            .unwrap_or_else(|| target_tip(&repo, target))?
                    } else {
                        target_tip(&repo, target)?
                    };
                    pins.push(before.to_string());
                    if matches!(request.action, Action::Finish)
                        && !contains(&repo, before, incoming)?
                    {
                        ensure_branch_available(&repo, target)?;
                        if repo.merge_base(before, incoming).is_err() {
                            return Err("No merge base is available. Fetch more history or verify the branches first.".into());
                        }
                        let fast_forward = config.strategy == Strategy::Trunk;
                        if fast_forward && !contains(&repo, incoming, before)? {
                            return Err(format!("{source} has diverged from {target}. Rebase it before a fast-forward integration."));
                        }
                        primary_after_merge |= *target == config.primary;
                        steps.push(Step::Merge {
                            source: source.clone(),
                            target: (*target).into(),
                            incoming: incoming.to_string(),
                            before: before.to_string(),
                            fast_forward,
                        });
                    }
                    if analyze && !matches!(request.action, Action::Cleanup) {
                        comparisons.push(compare(
                            &repo,
                            incoming,
                            before,
                            target,
                            true,
                            is_handoff && config.strategy == Strategy::Swamp,
                        )?);
                    }
                }
                if is_handoff {
                    let tracked_target = tracked(&config.primary);
                    let tracked_source = tracked(source);
                    pins.push(tracked_source.map(|id| id.to_string()).unwrap_or_default());
                    pins.push(tracked_target.map(|id| id.to_string()).unwrap_or_default());
                    let source_sync = match (remote, tracked_source) {
                        (None, _) => "no-remote",
                        (_, None) => "not-published",
                        (_, Some(id)) if id == incoming => "in-sync",
                        (_, Some(id))
                            if analyze && contains(&repo, incoming, id).unwrap_or(false) =>
                        {
                            "local-ahead"
                        }
                        (_, Some(id))
                            if analyze && contains(&repo, id, incoming).unwrap_or(false) =>
                        {
                            "remote-ahead"
                        }
                        _ => {
                            if repo.is_shallow() {
                                "unknown"
                            } else {
                                "different"
                            }
                        }
                    };
                    let published = tracked_source == Some(incoming) && tracked_target.is_some();
                    let checkpoint = if analyze && config.strategy == Strategy::Swamp {
                        tracked_target
                            .or_else(|| target_tip(&repo, &config.primary).ok())
                            .and_then(|target| checkpoint(&repo, incoming, target))
                    } else {
                        None
                    };
                    handoff = Some(Handoff {
                        source: source.clone(),
                        target: config.primary.clone(),
                        published,
                        checkpoint,
                        remote: remote.map(str::to_owned),
                        target_fetched: tracked_target.is_some(),
                        source_sync,
                    });
                }
                if matches!(request.action, Action::Finish)
                    && destinations.len() == 2
                    && !request.name.is_empty()
                {
                    let production = target_tip(&repo, &config.primary)?;
                    steps.extend(tag_step(
                        &repo,
                        config,
                        &request.name,
                        production,
                        &config.primary,
                        primary_after_merge,
                    )?);
                }
                if matches!(request.action, Action::Cleanup)
                    || matches!(request.action, Action::Finish) && request.cleanup
                {
                    let targets: Vec<String> =
                        destinations.iter().map(|name| (*name).into()).collect();
                    ensure_branch_available(&repo, source)?;
                    if is_published(&repo, source)? {
                        return Err(
                            "This branch has a remote counterpart. Turn off cleanup to retain it."
                                .into(),
                        );
                    }
                    if matches!(request.action, Action::Cleanup) {
                        check_deletion(&repo, source, &targets, incoming)?;
                        if current == *source {
                            ensure_branch_available(&repo, &targets[0])?;
                        }
                    }
                    steps.push(Step::Delete {
                        name: source.clone(),
                        revision: incoming.to_string(),
                        targets,
                    });
                }
            }
            Action::Tag if matches!(config.strategy, Strategy::Trunk | Strategy::Gitflow) => {
                let target = target_tip(&repo, &config.primary)?;
                steps.extend(tag_step(
                    &repo,
                    config,
                    &request.name,
                    target,
                    &config.primary,
                    false,
                )?);
            }
            _ => return Err("This action is not supported by the active strategy.".into()),
        }
    }
    let token = serde_json::to_vec(&(
        path,
        current,
        head.to_string(),
        config,
        request,
        &steps,
        pins,
    ))
    .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
    .map_err(|error| error.to_string())?;
    Ok((
        Plan {
            token,
            steps: steps.iter().map(Step::title).collect(),
            comparisons,
            handoff,
        },
        steps,
    ))
}

pub fn preview(path: &str, config: &Config, request: &Request) -> Result<Plan, String> {
    resolve(path, config, request, true).map(|(plan, _)| plan)
}

pub fn verify_handoff(
    path: &str,
    config: &Config,
    request: &Request,
    token: &str,
) -> Result<(), String> {
    let (plan, _) = resolve(path, config, request, false)?;
    if plan.token != token || !plan.handoff.is_some_and(|handoff| handoff.published) {
        return Err(STALE.into());
    }
    Ok(())
}

fn switch(path: &str, branch: &str, completed: &mut Vec<String>) -> Result<(), String> {
    if current_branch(&Git::open(path)?).as_deref() != Some(branch) {
        Git::git_cli(
            path,
            &["switch", "--no-guess", "--no-overwrite-ignore", branch],
        )?;
        completed.push(format!("Switched to {branch}"));
    }
    Ok(())
}

pub fn execute(
    path: &str,
    config: &Config,
    request: &Request,
    expected: &str,
) -> Result<Outcome, String> {
    let _guard = WORKFLOW
        .lock()
        .map_err(|_| "The workflow is unavailable.")?;
    let (plan, steps) = resolve(path, config, request, false)?;
    if plan.token != expected {
        return Err(STALE.into());
    }
    if plan.handoff.is_some() {
        return Err("A handoff does not change Git refs.".into());
    }
    let mut completed = Vec::new();
    let mut production_written = None;
    for (index, step) in steps.iter().enumerate() {
        let result = (|| {
            let repo = Git::open(path)?;
            check_clean(&repo)?;
            match step {
                Step::Branch {
                    name,
                    revision,
                    switch,
                    ..
                } => {
                    if *switch {
                        Git::git_cli(
                            path,
                            &[
                                "switch",
                                "--no-overwrite-ignore",
                                "--no-track",
                                "-c",
                                name,
                                revision,
                            ],
                        )?;
                    } else {
                        Git::git_cli(path, &["branch", "--no-track", name, revision])?;
                    }
                }
                Step::Merge {
                    source,
                    target,
                    incoming,
                    before,
                    fast_forward,
                } => {
                    if target_tip(&repo, target)?.to_string() != *before
                        || target_tip(&repo, source)?.to_string() != *incoming
                    {
                        return Err(STALE.into());
                    }
                    switch(path, target, &mut completed)?;
                    let checked = Git::open(path)?;
                    check_clean(&checked)?;
                    if current_branch(&checked).as_deref() != Some(target)
                        || target_tip(&checked, target)?.to_string() != *before
                    {
                        return Err(STALE.into());
                    }
                    let mode = if *fast_forward {
                        "--ff-only"
                    } else {
                        "--no-ff"
                    };
                    Git::git_cli(
                        path,
                        &[
                            "merge",
                            mode,
                            "--no-edit",
                            "--no-autostash",
                            "--no-overwrite-ignore",
                            incoming,
                        ],
                    )?;
                    if target == &config.primary {
                        production_written = Git::open(path)
                            .ok()
                            .and_then(|repo| target_tip(&repo, target).ok())
                            .map(|id| id.to_string());
                    }
                }
                Step::Tag {
                    name,
                    target,
                    revision,
                } => {
                    let expected = if revision.is_empty() {
                        production_written.as_deref().ok_or(STALE)?
                    } else {
                        revision
                    };
                    if target_tip(&repo, target)?.to_string() != expected {
                        return Err(STALE.into());
                    }
                    Git::git_cli(path, &["tag", "--no-sign", name, expected])?;
                }
                Step::Delete {
                    name,
                    revision,
                    targets,
                } => {
                    check_deletion(
                        &repo,
                        name,
                        targets,
                        Oid::from_str(revision).map_err(git_error)?,
                    )?;
                    if current_branch(&repo).as_ref() == Some(name) {
                        switch(path, &targets[0], &mut completed)?;
                    }
                    let checked = Git::open(path)?;
                    check_deletion(
                        &checked,
                        name,
                        targets,
                        Oid::from_str(revision).map_err(git_error)?,
                    )?;
                    Git::git_cli(path, &["branch", "--delete", "--", name])?;
                }
            }
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            let conflict = matches!(step, Step::Merge { .. })
                && Git::open(path).is_ok_and(|repo| repo.state() == RepositoryState::Merge);
            return Ok(Outcome {
                status: if conflict { "conflict" } else { "stopped" },
                completed,
                remaining: plan.steps[index..].to_vec(),
                error: Some(error),
            });
        }
        completed.push(step.title());
    }
    Ok(Outcome {
        status: "applied",
        completed,
        remaining: Vec::new(),
        error: None,
    })
}

#[cfg(test)]
#[path = "branch_workflow_tests.rs"]
mod tests;
