use super::*;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AmendAction {
    Edit,
    Reword,
    Squash,
    Fixup,
    Drop,
}

impl AmendAction {
    fn rebase(self) -> Action {
        match self {
            Self::Edit => Action::Edit,
            Self::Reword => Action::Reword,
            Self::Squash => Action::Squash,
            Self::Fixup => Action::Fixup,
            Self::Drop => Action::Drop,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmendPlan {
    branch: String,
    head: String,
    target: RebaseCommit,
    parent: Option<RebaseCommit>,
    replay: usize,
    published: bool,
    dirty: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Confirmation {
    pub head: String,
    pub branch: String,
    pub message: Option<String>,
    pub acknowledge_published: bool,
}

fn reviewed(commit: &git2::Commit<'_>) -> RebaseCommit {
    RebaseCommit {
        sha: commit.id().to_string(),
        message: String::from_utf8_lossy(commit.message_bytes())
            .trim_end()
            .into(),
    }
}

fn root_plan(repo: &Repository) -> Result<RebasePlan, String> {
    let head = repo.head().map_err(|e| e.to_string())?;
    let branch = head
        .shorthand()
        .filter(|_| head.is_branch())
        .ok_or("Check out a local branch first.")?;
    let mut commit = head.peel_to_commit().map_err(|e| e.to_string())?;
    let head = commit.id().to_string();
    let mut commits = Vec::new();
    loop {
        commits.push(reviewed(&commit));
        match commit.parent_count() {
            0 => break,
            1 => commit = commit.parent(0).map_err(|e| e.to_string())?,
            _ => return Err("Interactive Amend does not rewrite merge commits. Select a commit after the merge.".into()),
        }
    }
    commits.reverse();
    Ok(RebasePlan {
        branch: branch.into(),
        head,
        onto: String::new(),
        upstream: String::new(),
        fast_forward: false,
        commits,
    })
}

fn inspect(
    repo: &Repository,
    target: &str,
    action: AmendAction,
) -> Result<(RebasePlan, AmendPlan, bool), String> {
    if repo.state() != RepositoryState::Clean {
        return Err("Finish the current Git operation before amending history.".into());
    }
    let target = repo
        .find_commit(Oid::from_str(target).map_err(|_| "Invalid commit SHA.")?)
        .map_err(|_| "The selected commit no longer exists.")?;
    if target.parent_count() > 1 {
        return Err("Merge commits cannot be rewritten with Interactive Amend.".into());
    }
    let parent = (target.parent_count() == 1)
        .then(|| target.parent(0))
        .transpose()
        .map_err(|e| e.to_string())?;
    let combines = action.rebase().combines();
    let first = if combines {
        parent
            .as_ref()
            .ok_or("The root commit has no parent to combine with.")?
    } else {
        &target
    };
    if first.parent_count() > 1 {
        return Err("The parent is a merge commit and cannot be rewritten here.".into());
    }
    let root = first.parent_count() == 0;
    let plan = if root {
        root_plan(repo)?
    } else {
        build_plan(
            repo,
            &first.parent_id(0).map_err(|e| e.to_string())?.to_string(),
        )?
    };
    let target_sha = target.id().to_string();
    let target_index = plan
        .commits
        .iter()
        .position(|item| item.sha == target_sha)
        .ok_or("The selected commit is not in the checked-out branch's linear history.")?;
    if root && plan.commits.len() == 1 && matches!(action, AmendAction::Drop) {
        return Err("Dropping the only commit would remove the branch history.".into());
    }
    if target_index != usize::from(combines) {
        return Err(
            "The selected commits are not a linear parent and child on this branch.".into(),
        );
    }
    let upstream = repo
        .find_branch(&plan.branch, git2::BranchType::Local)
        .ok()
        .and_then(|branch| branch.upstream().ok())
        .and_then(|branch| branch.get().target());
    let published = upstream.is_some_and(|sha| {
        sha == first.id() || repo.graph_descendant_of(sha, first.id()).unwrap_or(false)
    });
    let info = AmendPlan {
        branch: plan.branch.clone(),
        head: plan.head.clone(),
        target: reviewed(&target),
        parent: parent.as_ref().map(reviewed),
        replay: plan.commits.len() - target_index - 1,
        published,
        dirty: false,
    };
    Ok((plan, info, root))
}

pub fn plan(path: &str, target: &str, action: AmendAction) -> Result<AmendPlan, String> {
    let repo = GitRepository::open(path)?;
    let (_, mut info, _) = inspect(&repo, target, action)?;
    info.dirty = require_clean(&repo).is_err();
    Ok(info)
}

pub fn start(
    path: &str,
    target: &str,
    action: AmendAction,
    confirmation: Confirmation,
) -> Result<Outcome, String> {
    let _lock = REBASE.lock().map_err(|e| e.to_string())?;
    let repo = GitRepository::open(path)?;
    let (plan, reviewed, root) = inspect(&repo, target, action)?;
    if plan.head != confirmation.head || plan.branch != confirmation.branch {
        return Err("HEAD or branch changed. Review Interactive Amend again.".into());
    }
    if reviewed.published && !confirmation.acknowledge_published {
        return Err("Confirm that this will rewrite published history.".into());
    }
    require_clean(&repo)?;
    let message = confirmation.message;
    let mut steps: Vec<_> = plan
        .commits
        .iter()
        .map(|commit| RebaseStep {
            sha: commit.sha.clone(),
            action: Action::Pick,
            message: None,
            combined_message: None,
        })
        .collect();
    let index = usize::from(action.rebase().combines());
    steps[index].action = action.rebase();
    if matches!(action, AmendAction::Reword) {
        steps[index].message = message;
    } else if matches!(action, AmendAction::Squash) {
        steps[0].combined_message =
            Some(message.ok_or("Enter the new message for the combined commit.")?);
    }
    validate(&plan, &steps)?;
    let base = (!root).then_some(plan.onto.as_str());
    let replay = steps
        .iter()
        .filter(|step| step.action != Action::Drop)
        .map(|step| step.sha.as_str());
    protect_local_paths(
        &repo,
        base.into_iter().chain(replay),
        false,
        "Starting Interactive Amend",
    )?;
    execute(&repo, path, &plan, &steps, root)
}
