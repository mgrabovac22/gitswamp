use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use git2::{ObjectType, Oid, Repository, RepositoryState, Status, StatusOptions, StatusShow, Tree};
use serde::{Deserialize, Serialize};

use crate::repositories::git_repository::GitRepository;

pub mod amend;

static REBASE: Mutex<()> = Mutex::new(());
const STATE: &str = "rebase-merge/gitswamp-messages";
// Git owns the todo and recovery state. Install the plan only for the reviewed HEAD.
const SEQUENCE: &str = r#"set -eu
dir=${1%/*}
cmp "$dir/orig-head" "$GITSWAMP_REBASE_TEMP/head"
cmp "$dir/head-name" "$GITSWAMP_REBASE_TEMP/branch"
while read -r action sha rest; do
    case "$action" in pick) printf '%s\n' "$sha";; esac
done < "$1" | cmp - "$GITSWAMP_REBASE_TEMP/original"
cp -R -- "$GITSWAMP_REBASE_TEMP/messages" "$dir/gitswamp-messages"
cat -- "$GITSWAMP_REBASE_TEMP/todo" > "$1"
"#;
const EDITOR: &str = r#"set -eu
sha=
while read -r action oid rest; do
    case "$action" in pick|reword|edit|squash|fixup) sha=$oid;; esac
done < "$GITSWAMP_REBASE_STATE/../done"
if [ -n "$sha" ] && [ -f "$GITSWAMP_REBASE_STATE/$sha" ]; then
    cat -- "$GITSWAMP_REBASE_STATE/$sha" > "$1"
fi
"#;

#[derive(Serialize)]
pub struct RebaseCommit {
    sha: String,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RebasePlan {
    branch: String,
    head: String,
    onto: String,
    upstream: String,
    fast_forward: bool,
    commits: Vec<RebaseCommit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    Pick,
    Reword,
    Edit,
    Squash,
    Fixup,
    Drop,
}

impl Action {
    fn git(self) -> &'static str {
        match self {
            Self::Pick => "pick",
            Self::Reword => "reword",
            Self::Edit => "edit",
            Self::Squash => "squash",
            Self::Fixup => "fixup",
            Self::Drop => "drop",
        }
    }
    fn combines(self) -> bool {
        matches!(self, Self::Squash | Self::Fixup)
    }
}

#[derive(Deserialize)]
pub struct RebaseStep {
    sha: String,
    action: Action,
    message: Option<String>,
    #[serde(rename = "combinedMessage")]
    combined_message: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Completed,
    FastForwarded,
    Editing,
    Conflict,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FollowUp {
    Continue,
    Skip,
    Abort,
}

pub fn is_active(repo: &Repository) -> bool {
    repo.path().join(STATE).is_dir()
}

pub fn can_amend(repo: &Repository) -> bool {
    repo.path().join("rebase-merge/amend").is_file()
        && !repo.path().join("CHERRY_PICK_HEAD").is_file()
        && fs::read_to_string(repo.path().join("rebase-merge/done")).is_ok_and(|done| {
            done.lines()
                .rev()
                .find(|line| !line.trim().is_empty())
                .is_some_and(|line| line.starts_with("edit "))
        })
}

pub fn require_inactive(repo: &Repository) -> Result<(), String> {
    if is_active(repo) {
        Err(
            "Continue or abort the interactive rebase before changing branches or resetting HEAD."
                .into(),
        )
    } else {
        Ok(())
    }
}

pub fn plan(path: &str, base: &str) -> Result<RebasePlan, String> {
    let repo = GitRepository::open(path)?;
    build_plan(&repo, base)
}

fn build_plan(repo: &Repository, base: &str) -> Result<RebasePlan, String> {
    if repo.state() != RepositoryState::Clean {
        return Err("Finish the current Git operation before starting another rebase.".into());
    }
    let head_ref = repo
        .head()
        .map_err(|_| "The current branch has no commits.")?;
    let branch = head_ref
        .shorthand()
        .filter(|_| head_ref.is_branch())
        .ok_or("Check out a local branch before starting an interactive rebase.")?
        .to_owned();
    let mut commit = head_ref.peel_to_commit().map_err(|e| e.to_string())?;
    let head = commit.id().to_string();
    let onto = Oid::from_str(base).map_err(|_| "Invalid base commit.")?;
    repo.find_commit(onto)
        .map_err(|_| "The selected commit no longer exists.")?;
    if onto == commit.id() {
        return Err("There are no commits to rebase onto the selected commit.".into());
    }
    let upstream = repo.merge_base(commit.id(), onto).map_err(|_| {
        "The selected commit has no common history with the checked-out branch.".to_string()
    })?;
    let fast_forward = upstream == commit.id();
    let mut commits = Vec::new();
    while commit.id() != upstream {
        if commit.parent_count() != 1 {
            return Err("This range contains a merge commit. Choose a base after the merge to preserve the branch structure.".into());
        }
        commits.push(RebaseCommit {
            sha: commit.id().to_string(),
            message: String::from_utf8_lossy(commit.message_bytes())
                .trim_end()
                .to_owned(),
        });
        commit = commit.parent(0).map_err(|e| e.to_string())?;
    }
    commits.reverse();
    Ok(RebasePlan {
        branch,
        head,
        onto: onto.to_string(),
        upstream: upstream.to_string(),
        fast_forward,
        commits,
    })
}

fn git_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    if cfg!(windows) {
        path.trim_start_matches(r"\\?\").replace('\\', "/")
    } else {
        path.into_owned()
    }
}

fn write(path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    fs::write(path, content).map_err(|error| format!("Could not prepare the rebase: {error}"))
}

fn require_clean(repo: &Repository) -> Result<(), String> {
    let mut options = StatusOptions::new();
    options.include_untracked(true);
    if !repo
        .statuses(Some(&mut options))
        .map_err(|e| e.to_string())?
        .is_empty()
    {
        return Err(
            "Commit or stash staged, unstaged and untracked changes before starting the rebase."
                .into(),
        );
    }
    Ok(())
}

fn validate(plan: &RebasePlan, steps: &[RebaseStep]) -> Result<(), String> {
    let mut available: HashSet<_> = plan
        .commits
        .iter()
        .map(|commit| commit.sha.as_str())
        .collect();
    let mut previous = false;
    for step in steps {
        if !available.remove(step.sha.as_str()) {
            return Err("The plan must contain every commit in the range exactly once.".into());
        }
        if step.action.combines() && !previous {
            return Err("Squash and Fixup need a preceding commit that is not dropped.".into());
        }
        for message in [step.message.as_deref(), step.combined_message.as_deref()]
            .into_iter()
            .flatten()
        {
            if message.trim().is_empty() || message.contains('\0') {
                return Err("Commit messages cannot be empty or contain a NUL character.".into());
            }
        }
        if step.action == Action::Reword && step.message.is_none() {
            return Err("Enter the new message for each Reword commit.".into());
        }
        previous |= step.action != Action::Drop;
    }
    if !available.is_empty() {
        return Err("The plan is missing commits from the range.".into());
    }
    Ok(())
}

fn prepare(directory: &Path, plan: &RebasePlan, steps: &[RebaseStep]) -> Result<char, String> {
    use std::fmt::Write;
    let mut todo = String::new();
    let messages = directory.join("messages");
    fs::create_dir(&messages).map_err(|e| e.to_string())?;
    let mut comment_chars: HashSet<char> = "#;@!%~^$&|:+=?".chars().collect();
    for commit in &plan.commits {
        for first in commit
            .message
            .lines()
            .filter_map(|line| line.chars().next())
        {
            comment_chars.remove(&first);
        }
    }
    let kept: Vec<_> = steps
        .iter()
        .filter(|step| step.action != Action::Drop)
        .collect();
    let mut group_head = 0;
    for (i, step) in kept.iter().enumerate() {
        let _ = writeln!(todo, "{} {}", step.action.git(), step.sha);
        if !step.action.combines() {
            group_head = i;
        }
        let group_end = kept.get(i + 1).is_none_or(|next| !next.action.combines());
        let combined = (step.action.combines()
            && group_end
            && kept[group_head + 1..=i]
                .iter()
                .any(|part| part.action == Action::Squash))
        .then(|| kept[group_head].combined_message.as_deref())
        .flatten();
        let message = if step.action == Action::Reword {
            step.message.as_deref()
        } else {
            combined
        };
        if let Some(message) = message {
            for first in message.lines().filter_map(|line| line.chars().next()) {
                comment_chars.remove(&first);
            }
            write(
                &messages.join(&step.sha),
                format!("{}\n", message.trim_end()),
            )?;
        }
    }
    if todo.is_empty() {
        todo.push_str("noop\n");
    }
    let mut original = String::new();
    for commit in &plan.commits {
        let _ = writeln!(original, "{}", commit.sha);
    }
    let comment = "#;@!%~^$&|:+=?"
        .chars()
        .find(|ch| comment_chars.contains(ch))
        .ok_or(
            "The commit messages use all available Git comment characters at the start of lines.",
        )?;
    write(&messages.join("comment"), comment.to_string())?;
    write(&directory.join("head"), format!("{}\n", plan.head))?;
    write(
        &directory.join("branch"),
        format!("refs/heads/{}\n", plan.branch),
    )?;
    write(&directory.join("original"), original)?;
    write(&directory.join("todo"), todo)?;
    write(&directory.join("sequence.sh"), SEQUENCE)?;
    Ok(comment)
}

pub fn start(
    path: &str,
    base: &str,
    head: &str,
    branch: &str,
    steps: &[RebaseStep],
) -> Result<Outcome, String> {
    let _lock = REBASE.lock().map_err(|e| e.to_string())?;
    let repo = GitRepository::open(path)?;
    let plan = build_plan(&repo, base)?;
    if plan.head != head || plan.branch != branch {
        return Err("HEAD moved or the branch changed. Reopen the rebase plan.".into());
    }
    require_clean(&repo)?;
    validate(&plan, steps)?;
    if plan.fast_forward {
        // No local commits need rewriting. Let Git move the checked-out branch safely.
        GitRepository::git_cli_with_env(
            path,
            &[
                "merge",
                "--ff-only",
                "--no-autostash",
                "--no-overwrite-ignore",
                &plan.onto,
            ],
            &[],
        )?;
        return Ok(Outcome::FastForwarded);
    }
    if plan.onto == plan.upstream
        && steps
            .iter()
            .zip(&plan.commits)
            .all(|(step, commit)| step.action == Action::Pick && step.sha == commit.sha)
    {
        return Ok(Outcome::Completed);
    }
    execute(&repo, path, &plan, steps, false)
}

fn execute(
    repo: &Repository,
    path: &str,
    plan: &RebasePlan,
    steps: &[RebaseStep],
    root: bool,
) -> Result<Outcome, String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let comment = prepare(temp.path(), plan, steps)?;
    // Pin the selected range and disable config that could change the plan or other refs.
    let mut args = vec![
        "-c",
        "core.abbrev=40",
        "-c",
        "rebase.abbreviateCommands=false",
        "-c",
        "rebase.instructionFormat=%s",
        "-c",
        "rebase.missingCommitsCheck=ignore",
        "rebase",
        "--interactive",
        "--no-autosquash",
        "--no-autostash",
        "--no-rebase-merges",
        "--no-update-refs",
        "--no-fork-point",
        "--reapply-cherry-picks",
        "--empty=keep",
    ];
    if root {
        args.push("--root");
    } else {
        args.extend(["--onto", &plan.onto, &plan.upstream]);
    }
    run(repo, path, temp.path(), comment, &args)
}

pub fn follow_up(
    path: &str,
    action: FollowUp,
    secret_check: bool,
    secret_approval: Option<&str>,
) -> Result<Outcome, String> {
    let _lock = REBASE.lock().map_err(|e| e.to_string())?;
    let repo = GitRepository::open(path)?;
    if !is_active(&repo) {
        return Err("No visual interactive rebase is active in this repository.".into());
    }
    if matches!(action, FollowUp::Abort) {
        let original = fs::read_to_string(repo.path().join("rebase-merge/orig-head"))
            .map_err(|error| error.to_string())?;
        protect_local_paths(&repo, [original.trim()], true, "Aborting")?;
        GitRepository::git_cli(path, &["rebase", "--abort"])?;
        return Ok(Outcome::Completed);
    }
    if matches!(action, FollowUp::Skip) && can_amend(&repo) {
        return Err("This commit is paused for editing, not conflict resolution. Amend and continue, or abort the rebase.".into());
    }
    if matches!(action, FollowUp::Continue) {
        let index = repo.index().map_err(|e| e.to_string())?;
        if index.has_conflicts() {
            return Err("Resolve all conflicts before continuing.".into());
        }
        let head = repo
            .head()
            .and_then(|head| head.peel_to_tree())
            .map_err(|e| e.to_string())?;
        let staged = repo
            .diff_tree_to_index(Some(&head), Some(&index), None)
            .map_err(|e| e.to_string())?;
        if staged.deltas().len() > 0 {
            super::secret_service::validate_commit(path, secret_check, secret_approval)?;
        }
    }
    let todo = fs::read_to_string(repo.path().join("rebase-merge/git-rebase-todo"))
        .map_err(|error| error.to_string())?;
    let skip = matches!(action, FollowUp::Skip);
    let current = skip
        .then(|| {
            repo.head()
                .and_then(|head| head.peel_to_commit())
                .map(|commit| commit.id().to_string())
        })
        .transpose()
        .map_err(|error| error.to_string())?;
    let remaining = todo.lines().filter_map(|line| {
        let mut fields = line.split_ascii_whitespace();
        match fields.next()? {
            "pick" | "p" | "reword" | "r" | "edit" | "e" | "squash" | "s" => fields.next(),
            "fixup" | "f" => fields.find(|field| !field.starts_with('-')),
            _ => None,
        }
    });
    protect_local_paths(
        &repo,
        current.as_deref().into_iter().chain(remaining),
        skip,
        if skip { "Skipping" } else { "Continuing" },
    )?;
    let comment = fs::read_to_string(repo.path().join(STATE).join("comment"))
        .ok()
        .and_then(|value| value.chars().next())
        .unwrap_or('#');
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    run(
        &repo,
        path,
        temp.path(),
        comment,
        &[
            "rebase",
            if matches!(action, FollowUp::Continue) {
                "--continue"
            } else {
                "--skip"
            },
        ],
    )
}

fn protect_local_paths<'a>(
    repo: &Repository,
    commits: impl IntoIterator<Item = &'a str>,
    include_untracked: bool,
    operation: &str,
) -> Result<(), String> {
    let mut commits = commits.into_iter().peekable();
    if commits.peek().is_none() {
        return Ok(());
    }
    let mut options = StatusOptions::new();
    options
        .show(StatusShow::Workdir)
        .include_untracked(include_untracked)
        .include_ignored(true);
    let statuses = repo
        .statuses(Some(&mut options))
        .map_err(|error| error.to_string())?;
    let flags = if include_untracked {
        Status::WT_NEW | Status::IGNORED
    } else {
        Status::IGNORED
    };
    let local: Vec<_> = statuses
        .iter()
        .filter(|entry| entry.status().intersects(flags))
        .collect();
    if local.is_empty() {
        return Ok(());
    }

    let ignore_case = repo
        .config()
        .and_then(|config| config.get_bool("core.ignorecase"))
        .unwrap_or(cfg!(windows));
    let mut previous = None;
    for commit in commits {
        let tree = if commit.len() == 40 {
            Oid::from_str(commit)
                .and_then(|id| repo.find_commit(id))
                .and_then(|commit| commit.tree())
        } else {
            repo.revparse_single(commit)
                .and_then(|object| object.peel_to_tree())
        }
        .map_err(|error| error.to_string())?;
        if previous == Some(tree.id()) {
            continue;
        }
        previous = Some(tree.id());
        // Git's rebase can overwrite ignored files while visiting intermediate trees.
        for entry in &local {
            let path = entry.path().ok_or("Move untracked or ignored paths with non-UTF-8 names outside the repository before continuing.")?.trim_end_matches('/');
            if tree_contains_local_path(repo, &tree, path, ignore_case)? {
                return Err(format!("{operation} would overwrite local path {path}. Move it outside the repository and try again."));
            }
        }
    }
    Ok(())
}

fn tree_contains_local_path(
    repo: &Repository,
    tree: &Tree<'_>,
    path: &str,
    ignore_case: bool,
) -> Result<bool, String> {
    let (name, rest) = path
        .split_once('/')
        .map_or((path, None), |(head, tail)| (head, Some(tail)));
    let entry = tree.get_name(name).or_else(|| {
        if !ignore_case {
            return None;
        }
        let name = name.to_lowercase();
        tree.iter().find(|entry| {
            entry
                .name()
                .is_some_and(|value| value.to_lowercase() == name)
        })
    });
    let Some(entry) = entry else {
        return Ok(false);
    };
    match rest {
        Some(rest) if entry.kind() == Some(ObjectType::Tree) => {
            let child = repo
                .find_tree(entry.id())
                .map_err(|error| error.to_string())?;
            tree_contains_local_path(repo, &child, rest, ignore_case)
        }
        _ => Ok(true),
    }
}

fn run(
    repo: &Repository,
    path: &str,
    temp: &Path,
    comment: char,
    args: &[&str],
) -> Result<Outcome, String> {
    let editor = temp.join("editor.sh");
    write(&editor, EDITOR)?;
    let config = format!("core.commentChar={comment}");
    // Set only a single comment token. An explicit commentString takes precedence on recent Git.
    let comment_string = format!("core.commentString={comment}");
    let mut git_args = vec![
        "-c",
        config.as_str(),
        "-c",
        comment_string.as_str(),
        "-c",
        "commit.cleanup=strip",
    ];
    git_args.extend_from_slice(args);
    let result = GitRepository::git_cli_with_env(
        path,
        &git_args,
        &[
            (
                "GIT_SEQUENCE_EDITOR",
                OsStr::new("sh \"$GITSWAMP_REBASE_TEMP/sequence.sh\""),
            ),
            ("GIT_EDITOR", OsStr::new("sh \"$GITSWAMP_REBASE_EDITOR\"")),
            ("GITSWAMP_REBASE_TEMP", OsStr::new(&git_path(temp))),
            ("GITSWAMP_REBASE_EDITOR", OsStr::new(&git_path(&editor))),
            (
                "GITSWAMP_REBASE_STATE",
                OsStr::new(&git_path(&repo.path().join(STATE))),
            ),
        ],
    );
    if is_active(repo) {
        let mut index = repo.index().map_err(|e| e.to_string())?;
        index.read(true).map_err(|e| e.to_string())?;
        if index.has_conflicts() {
            return Ok(Outcome::Conflict);
        }
    }
    result?;
    if is_active(repo) {
        Ok(Outcome::Editing)
    } else {
        Ok(Outcome::Completed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _directory: tempfile::TempDir,
        path: String,
    }

    impl Fixture {
        fn new() -> Self {
            let directory =
                tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap();
            let path = directory.path().to_str().unwrap().to_owned();
            let fixture = Self {
                _directory: directory,
                path,
            };
            fixture.git(&["init", "-b", "main"]);
            fixture.git(&["config", "user.name", "Test"]);
            fixture.git(&["config", "user.email", "test@example.invalid"]);
            fixture.git(&["config", "commit.gpgsign", "false"]);
            fixture
        }
        fn git(&self, args: &[&str]) -> String {
            GitRepository::git_cli(&self.path, args).unwrap()
        }
        fn commit(&self, file: &str, content: &str, message: &str) -> String {
            fs::write(Path::new(&self.path).join(file), content).unwrap();
            self.git(&["add", "--", file]);
            self.git(&["commit", "-m", message]);
            self.git(&["rev-parse", "HEAD"])
        }
        fn start(&self, base: &str, steps: &[RebaseStep]) -> Result<Outcome, String> {
            let plan = plan(&self.path, base).unwrap();
            start(&self.path, base, &plan.head, &plan.branch, steps)
        }

        fn clone_local(source: &str) -> Self {
            let directory =
                tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap();
            let path = directory.path().to_str().unwrap().to_owned();
            let fixture = Self {
                _directory: directory,
                path,
            };
            fixture.git(&[
                "clone",
                "-c",
                "core.autocrlf=false",
                "--no-hardlinks",
                "--no-local",
                "--",
                source,
                ".",
            ]);
            fixture.git(&["config", "user.name", "GitSwamp Test"]);
            fixture.git(&["config", "user.email", "test@example.invalid"]);
            fixture.git(&["config", "commit.gpgsign", "false"]);
            fixture
        }
    }

    fn step(sha: &str, action: Action) -> RebaseStep {
        RebaseStep {
            sha: sha.into(),
            action,
            message: None,
            combined_message: None,
        }
    }

    fn perform_amend(
        f: &Fixture,
        target: &str,
        action: amend::AmendAction,
        message: Option<&str>,
    ) -> Result<Outcome, String> {
        let head = f.git(&["rev-parse", "HEAD"]);
        let branch = f.git(&["branch", "--show-current"]);
        amend::plan(&f.path, target, action)?;
        amend::start(
            &f.path,
            target,
            action,
            amend::Confirmation {
                head,
                branch,
                message: message.map(str::to_owned),
                acknowledge_published: false,
            },
        )
    }

    fn exercise_amend_and_patches(f: &Fixture) {
        use super::super::{
            amend_selection::Removal,
            git_service::GitService,
            patches::{self, ExportScope},
        };
        use amend::AmendAction::*;

        let first = "gitswamp-test-parent.txt";
        let selected = "gitswamp-test-selected.txt";
        let later = "gitswamp-test-later.txt";
        let new = "gitswamp-test-extra.txt";
        f.git(&["config", "core.autocrlf", "false"]);
        f.git(&["switch", "-c", "verify-amend-original"]);
        let parent = f.commit(first, "parent contents\n", "Amend fixture parent");
        let target = f.commit(selected, "selected contents\n", "Amend fixture selected");
        let tip = f.commit(later, "later contents\n", "Amend fixture later");
        let tree = f.git(&["rev-parse", "HEAD^{tree}"]);

        f.git(&["switch", "-c", "verify-reword", &tip]);
        assert!(matches!(
            perform_amend(f, &target, Reword, Some("Reword selected\n\nKeep the tree")).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(
            f.git(&["show", "-s", "--format=%B", "HEAD^"]),
            "Reword selected\n\nKeep the tree"
        );
        assert_eq!(f.git(&["rev-parse", "HEAD^{tree}"]), tree);
        assert_eq!(f.git(&["rev-parse", "HEAD^^"]), parent);

        for (branch, action, message, subject) in [
            (
                "verify-squash",
                Squash,
                Some("Combined fixture"),
                "Combined fixture",
            ),
            ("verify-fixup", Fixup, None, "Amend fixture parent"),
        ] {
            f.git(&["switch", "-c", branch, &tip]);
            assert!(matches!(
                perform_amend(f, &target, action, message).unwrap(),
                Outcome::Completed
            ));
            assert_eq!(f.git(&["show", "-s", "--format=%s", "HEAD^"]), subject);
            assert_eq!(f.git(&["rev-parse", "HEAD^{tree}"]), tree);
            assert_eq!(
                f.git(&["show", "-s", "--format=%s", "HEAD"]),
                "Amend fixture later"
            );
        }

        f.git(&["switch", "-c", "verify-drop", &tip]);
        assert!(matches!(
            perform_amend(f, &target, Drop, None).unwrap(),
            Outcome::Completed
        ));
        assert!(!Path::new(&f.path).join(selected).exists());
        assert_eq!(f.git(&["rev-parse", "HEAD^"]), parent);

        let artifacts = tempfile::tempdir().unwrap();
        let export = artifacts.path().join("selected.patch");
        patches::export(&f.path, &export, Some(&target), ExportScope::AllTracked).unwrap();
        let patch = fs::read_to_string(&export).unwrap();
        assert!(patch.contains("Subject: [PATCH] Amend fixture selected"));
        patches::import(
            &f.path,
            &export,
            false,
            Some(&f.git(&["rev-parse", "HEAD"])),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(Path::new(&f.path).join(selected)).unwrap(),
            "selected contents\n"
        );
        assert!(f.git(&["status", "--porcelain"]).starts_with("??"));
        assert!(patches::import(
            &f.path,
            &export,
            false,
            Some(&f.git(&["rev-parse", "HEAD"]))
        )
        .unwrap_err()
        .contains("No changes were made"));
        fs::remove_file(Path::new(&f.path).join(selected)).unwrap();

        f.git(&["switch", "-c", "verify-edit-and-patch", &tip]);
        assert!(matches!(
            perform_amend(f, &target, Edit, None).unwrap(),
            Outcome::Editing
        ));
        assert!(can_amend(&GitRepository::open(&f.path).unwrap()));
        let edit = artifacts.path().join("new-binary.diff");
        fs::write(Path::new(&f.path).join(new), [0, 1, 2, 255]).unwrap();
        f.git(&["add", new]);
        patches::export(&f.path, &edit, None, ExportScope::Staged).unwrap();
        assert!(fs::read_to_string(&edit)
            .unwrap()
            .contains("GIT binary patch"));
        f.git(&["restore", "--staged", "--", new]);
        fs::remove_file(Path::new(&f.path).join(new)).unwrap();
        patches::import(&f.path, &edit, true, Some(&f.git(&["rev-parse", "HEAD"]))).unwrap();
        assert_eq!(
            fs::read(Path::new(&f.path).join(new)).unwrap(),
            [0, 1, 2, 255]
        );
        GitService::amend_commit(&f.path, "Edited contents", false, false).unwrap();
        assert!(matches!(
            follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(
            f.git(&["show", "-s", "--format=%s", "HEAD^"]),
            "Edited contents"
        );
        assert_eq!(
            f.git(&["log", "-1", "--format=%s", "--", new]),
            "Edited contents"
        );

        f.git(&["switch", "-c", "verify-cancel", &tip]);
        assert!(matches!(
            perform_amend(f, &target, Edit, None).unwrap(),
            Outcome::Editing
        ));
        assert!(matches!(
            follow_up(&f.path, FollowUp::Abort, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), tip);
        assert_eq!(f.git(&["rev-parse", "verify-amend-original"]), tip);
        assert!(f.git(&["status", "--porcelain"]).is_empty());

        f.git(&["switch", "-c", "verify-move-out", &tip]);
        fs::write(Path::new(&f.path).join(later), "additional local edit\n").unwrap();
        f.git(&["add", later]);
        let removal = || Removal {
            head: tip.clone(),
            paths: vec![later.into()],
        };
        let refused = GitService::amend_with_removal(
            &f.path,
            "Keep later separate",
            false,
            false,
            removal(),
            false,
            None,
        )
        .unwrap_err();
        assert!(refused.contains("Unstage"));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), tip);
        assert_eq!(
            f.git(&["show", &format!(":{later}")]),
            "additional local edit"
        );
        f.git(&["restore", "--staged", "--", later]);
        fs::write(Path::new(&f.path).join(new), "another staged addition\n").unwrap();
        f.git(&["add", new]);
        GitService::amend_with_removal(
            &f.path,
            "Keep later separate",
            false,
            false,
            removal(),
            false,
            None,
        )
        .unwrap();
        assert_eq!(f.git(&["status", "--porcelain"]), format!("?? {later}"));
        assert_eq!(
            fs::read_to_string(Path::new(&f.path).join(later)).unwrap(),
            "additional local edit\n"
        );
        assert_eq!(
            f.git(&["show", &format!("HEAD:{new}")]),
            "another staged addition"
        );
    }

    #[test]
    fn interactive_amend_actions_and_patch_round_trips() {
        exercise_amend_and_patches(&Fixture::new());
    }

    #[test]
    #[ignore = "set GITSWAMP_POKUSAJ to the local pokusaj integration-test repository"]
    fn interactive_amend_and_patch_on_pokusaj() {
        let source = std::env::var("GITSWAMP_POKUSAJ").expect("GITSWAMP_POKUSAJ");
        exercise_amend_and_patches(&Fixture::clone_local(&source));
    }

    fn exercise_amend_conflict_actions(f: &Fixture) {
        use super::super::git_service::GitService;
        use amend::AmendAction::*;

        let text = "amend-conflict.txt";
        let binary = "amend-binary.bin";
        let notes = "amend-local.txt";
        let tail = "amend-tail.txt";
        let worktree = Path::new(&f.path);
        f.git(&["config", "core.autocrlf", "false"]);
        f.git(&["config", "merge.conflictStyle", "diff3"]);
        f.git(&["config", "rebase.autoStash", "true"]);
        f.git(&["config", "rebase.updateRefs", "true"]);
        f.git(&["switch", "-c", "amend-conflict-check"]);
        f.commit(text, "original\n", "Base text");
        fs::write(worktree.join(binary), [0, 1, 255]).unwrap();
        f.git(&["add", "--", binary]);
        f.git(&["commit", "-m", "Base binary"]);
        let base = f.git(&["rev-parse", "HEAD"]);
        fs::write(worktree.join(binary), [0, 2, 255]).unwrap();
        f.git(&["add", "--", binary]);
        let target = f.commit(text, "selected version\n", "Drop this change");
        fs::write(worktree.join(binary), [0, 3, 255]).unwrap();
        f.git(&["add", "--", binary]);
        let replay = f.commit(text, "replayed version\n", "Replay these changes");
        let tip = f.commit(tail, "independent tail\n", "Keep this commit");
        f.git(&["branch", "amend-must-not-move", &replay]);

        let staged_before = f.git(&["write-tree"]);
        fs::write(worktree.join(text), "staged local edit\n").unwrap();
        GitService::stage_file(&f.path, text).unwrap();
        fs::write(worktree.join(text), "unstaged local edit\n").unwrap();
        fs::write(worktree.join(notes), "untracked local file\n").unwrap();
        let dirty_before = f.git(&["status", "--porcelain"]);
        let dirty_index = f.git(&["write-tree"]);
        assert_ne!(dirty_index, staged_before);
        for action in [Edit, Reword, Squash, Fixup, Drop] {
            let review =
                serde_json::to_value(amend::plan(&f.path, &target, action).unwrap()).unwrap();
            assert_eq!(review["dirty"], true);
            assert!(perform_amend(f, &target, action, Some("Message"))
                .unwrap_err()
                .contains("Commit or stash"));
            assert_eq!(f.git(&["rev-parse", "HEAD"]), tip);
            assert_eq!(f.git(&["status", "--porcelain"]), dirty_before);
            assert_eq!(f.git(&["write-tree"]), dirty_index);
            assert_eq!(
                fs::read(worktree.join(text)).unwrap(),
                b"unstaged local edit\n"
            );
            assert_eq!(
                fs::read(worktree.join(notes)).unwrap(),
                b"untracked local file\n"
            );
            assert!(!is_active(&GitRepository::open(&f.path).unwrap()));
        }
        assert!(f.git(&["stash", "list"]).is_empty());
        f.git(&["restore", "--staged", "--worktree", "--", text]);
        fs::remove_file(worktree.join(notes)).unwrap();

        for (branch, action) in [
            ("amend-abort-conflict", FollowUp::Abort),
            ("amend-skip-conflict", FollowUp::Skip),
            ("amend-resolve-conflict", FollowUp::Continue),
        ] {
            f.git(&["switch", "-c", branch, &tip]);
            assert!(matches!(
                perform_amend(f, &target, Drop, None).unwrap(),
                Outcome::Conflict
            ));
            // A fresh repo_info is how the UI restores its operation controls on reopen.
            let operation = GitService::repo_info(&f.path).unwrap().operation.unwrap();
            assert!(operation.visual_rebase && !operation.visual_rebase_editing);
            assert_eq!(
                GitService::status(&f.path)
                    .unwrap()
                    .iter()
                    .filter(|file| file.conflicted)
                    .count(),
                2
            );
            assert!(GitService::checkout_branch(&f.path, "amend-must-not-move").is_err());
            assert!(GitService::reset_to_commit(&f.path, &tip, "hard", false).is_err());
            assert!(GitService::pull(&f.path, None, true).is_err());
            assert!(GitService::cherry_pick(&f.path, &tip).is_err());
            assert!(GitService::create_commit(&f.path, "Do not bypass rebase").is_err());
            assert!(
                GitService::amend_commit(&f.path, "Do not bypass conflict", false, false).is_err()
            );
            assert!(amend::plan(&f.path, &target, Edit).is_err());
            assert!(follow_up(&f.path, FollowUp::Continue, false, None)
                .unwrap_err()
                .contains("Resolve all conflicts"));

            if matches!(action, FollowUp::Continue) {
                GitService::resolve_conflict_file(&f.path, text, "keep-base").unwrap();
                assert_eq!(f.git(&["show", &format!(":{text}")]), "original");
                assert!(GitService::resolve_conflict_file(&f.path, text, "keep-modified").is_err());
                assert!(follow_up(&f.path, FollowUp::Continue, false, None).is_err());
                GitService::resolve_all_conflicts(&f.path, "keep-modified").unwrap();
                assert_eq!(fs::read(worktree.join(binary)).unwrap(), [0, 3, 255]);
            }
            if matches!(action, FollowUp::Abort) {
                fs::write(worktree.join(tail), "local file at the restored path\n").unwrap();
                assert!(follow_up(&f.path, action, false, None)
                    .unwrap_err()
                    .contains(tail));
                assert!(is_active(&GitRepository::open(&f.path).unwrap()));
                assert_eq!(
                    fs::read(worktree.join(tail)).unwrap(),
                    b"local file at the restored path\n"
                );
                let excludes = GitRepository::open(&f.path)
                    .unwrap()
                    .path()
                    .join("info/exclude");
                let before = fs::read(&excludes).unwrap_or_default();
                fs::write(
                    &excludes,
                    [&before[..], format!("\n/{tail}\n").as_bytes()].concat(),
                )
                .unwrap();
                assert!(follow_up(&f.path, action, false, None)
                    .unwrap_err()
                    .contains(tail));
                fs::write(&excludes, before).unwrap();
                fs::remove_file(worktree.join(tail)).unwrap();
            }
            fs::write(worktree.join(notes), "created after the conflict\n").unwrap();
            assert!(matches!(
                follow_up(&f.path, action, false, None).unwrap(),
                Outcome::Completed
            ));
            assert_eq!(f.git(&["branch", "--show-current"]), branch);
            assert_eq!(f.git(&["rev-parse", "amend-must-not-move"]), replay);
            assert_eq!(f.git(&["rev-parse", "amend-conflict-check"]), tip);
            assert_eq!(f.git(&["status", "--porcelain"]), format!("?? {notes}"));
            assert_eq!(
                fs::read(worktree.join(notes)).unwrap(),
                b"created after the conflict\n"
            );
            assert!(GitService::repo_info(&f.path).unwrap().operation.is_none());
            assert!(follow_up(&f.path, action, false, None).is_err());
            assert_eq!(
                f.git(&["show", &format!("HEAD:{tail}")]),
                "independent tail"
            );
            match action {
                FollowUp::Abort => assert_eq!(f.git(&["rev-parse", "HEAD"]), tip),
                FollowUp::Skip => {
                    assert_eq!(f.git(&["rev-parse", "HEAD^"]), base);
                    assert_eq!(fs::read(worktree.join(binary)).unwrap(), [0, 1, 255]);
                }
                FollowUp::Continue => {
                    assert_eq!(f.git(&["rev-parse", "HEAD^^"]), base);
                    assert_eq!(f.git(&["show", &format!("HEAD:{text}")]), "original");
                }
            }
            fs::remove_file(worktree.join(notes)).unwrap();
        }
    }

    #[test]
    fn interactive_amend_preserves_working_layers_and_handles_conflict_actions() {
        exercise_amend_conflict_actions(&Fixture::new());
    }

    #[test]
    #[ignore = "set GITSWAMP_POKUSAJ to the local pokusaj integration-test repository"]
    fn interactive_amend_conflict_actions_on_pokusaj() {
        let source = std::env::var("GITSWAMP_POKUSAJ").expect("GITSWAMP_POKUSAJ");
        exercise_amend_conflict_actions(&Fixture::clone_local(&source));
    }

    #[test]
    fn interactive_amend_modify_delete_conflict_can_keep_either_side() {
        use super::super::git_service::GitService;
        use amend::AmendAction::Drop;

        let f = Fixture::new();
        let base = f.commit("base", "base\n", "Base");
        let target = f.commit("new.txt", "first\n", "Introduce file");
        let tip = f.commit("new.txt", "second\n", "Modify file");
        for (branch, side, exists) in [
            ("resolve-absent", "keep-base", false),
            ("resolve-present", "keep-modified", true),
        ] {
            f.git(&["switch", "-c", branch, &tip]);
            assert!(matches!(
                perform_amend(&f, &target, Drop, None).unwrap(),
                Outcome::Conflict
            ));
            GitService::resolve_conflict_file(&f.path, "new.txt", side).unwrap();
            assert_eq!(Path::new(&f.path).join("new.txt").exists(), exists);
            assert!(matches!(
                follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
                Outcome::Completed
            ));
            assert!(f.git(&["status", "--porcelain"]).is_empty());
            if exists {
                assert_eq!(f.git(&["show", "HEAD:new.txt"]), "second");
            } else {
                assert_eq!(
                    f.git(&["rev-parse", "HEAD^{tree}"]),
                    f.git(&["rev-parse", &format!("{base}^{{tree}}")])
                )
            }
        }
    }

    #[test]
    fn interactive_amend_edit_does_not_skip_or_consume_unstaged_work() {
        use super::super::git_service::GitService;
        use amend::AmendAction::Edit;

        let f = Fixture::new();
        f.git(&["config", "core.autocrlf", "false"]);
        f.commit("base", "base\n", "Base");
        let selected = f.commit("selected", "original\n", "Selected");
        f.commit("later", "later\n", "Later");
        assert!(matches!(
            perform_amend(&f, &selected, Edit, None).unwrap(),
            Outcome::Editing
        ));
        assert!(
            GitService::repo_info(&f.path)
                .unwrap()
                .operation
                .unwrap()
                .visual_rebase_editing
        );
        let file = Path::new(&f.path).join("selected");
        fs::write(&file, "edited during pause\n").unwrap();
        assert!(follow_up(&f.path, FollowUp::Skip, false, None)
            .unwrap_err()
            .contains("paused for editing"));
        assert!(follow_up(&f.path, FollowUp::Continue, false, None).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"edited during pause\n");
        assert!(can_amend(&GitRepository::open(&f.path).unwrap()));
        GitService::stage_file(&f.path, "selected").unwrap();
        GitService::amend_commit(&f.path, "Amended during pause", false, false).unwrap();
        assert!(matches!(
            follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["show", "HEAD^:selected"]), "edited during pause");
        assert_eq!(
            f.git(&["log", "-2", "--format=%s"]),
            "Later\nAmended during pause"
        );
        assert!(f.git(&["status", "--porcelain"]).is_empty());
    }

    #[test]
    fn interactive_amend_published_and_stale_confirmations_leave_other_refs_untouched() {
        use amend::AmendAction::Reword;

        let f = Fixture::new();
        let original = f.commit("data", "base\n", "Base");
        let head = f.commit("other", "second\n", "Second");
        let original_tree = f.git(&["rev-parse", "HEAD^{tree}"]);
        f.git(&["remote", "add", "origin", "."]);
        f.git(&["update-ref", "refs/remotes/origin/main", &head]);
        f.git(&["branch", "--set-upstream-to=origin/main"]);
        f.git(&["branch", "keep-this-ref", &head]);

        let review =
            serde_json::to_value(amend::plan(&f.path, &original, Reword).unwrap()).unwrap();
        assert_eq!(review["published"], true);
        let confirm = |acknowledge_published| amend::Confirmation {
            branch: "main".into(),
            head: head.clone(),
            message: Some("Reword the published root".into()),
            acknowledge_published,
        };
        assert!(amend::start(&f.path, &original, Reword, confirm(false))
            .unwrap_err()
            .contains("published history"));
        f.git(&["switch", "keep-this-ref"]);
        assert!(amend::start(&f.path, &original, Reword, confirm(true))
            .unwrap_err()
            .contains("HEAD or branch changed"));
        f.git(&["switch", "main"]);
        assert_eq!(f.git(&["rev-parse", "HEAD"]), head);
        assert!(matches!(
            amend::start(&f.path, &original, Reword, confirm(true)).unwrap(),
            Outcome::Completed
        ));
        assert_ne!(f.git(&["rev-parse", "HEAD"]), head);
        assert_eq!(f.git(&["rev-parse", "HEAD^{tree}"]), original_tree);
        assert_eq!(f.git(&["rev-parse", "refs/remotes/origin/main"]), head);
        assert_eq!(f.git(&["rev-parse", "keep-this-ref"]), head);
        assert_eq!(
            f.git(&["log", "-2", "--format=%s"]),
            "Second\nReword the published root"
        );
    }

    #[test]
    fn interactive_amend_preserves_ignored_files_at_start_and_during_replay() {
        use amend::AmendAction::{Edit, Reword};

        let f = Fixture::new();
        f.git(&["config", "core.autocrlf", "false"]);
        f.commit("base", "base\n", "Base");
        let edit = f.commit("edit", "edit\n", "Editable");
        f.commit("local.txt", "once tracked\n", "Track old file");
        f.git(&["rm", "local.txt"]);
        f.git(&["commit", "-m", "Remove old file"]);
        let original = f.git(&["rev-parse", "HEAD"]);
        let excludes = GitRepository::open(&f.path)
            .unwrap()
            .path()
            .join("info/exclude");
        fs::write(excludes, "/local.txt\n/unrelated.txt\n").unwrap();
        let local = Path::new(&f.path).join("local.txt");
        let unrelated = Path::new(&f.path).join("unrelated.txt");
        fs::write(&local, "private local data\n").unwrap();
        fs::write(&unrelated, "unrelated ignored data\n").unwrap();
        assert!(
            perform_amend(&f, &original, Reword, Some("Reword the deletion"))
                .unwrap_err()
                .contains("local.txt")
        );
        assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
        assert!(!is_active(&GitRepository::open(&f.path).unwrap()));
        assert_eq!(fs::read(&local).unwrap(), b"private local data\n");
        fs::remove_file(&local).unwrap();

        assert!(matches!(
            perform_amend(&f, &edit, Edit, None).unwrap(),
            Outcome::Editing
        ));
        fs::write(&local, "private data created at edit pause\n").unwrap();
        assert!(follow_up(&f.path, FollowUp::Continue, false, None)
            .unwrap_err()
            .contains("local.txt"));
        assert!(can_amend(&GitRepository::open(&f.path).unwrap()));
        assert_eq!(
            fs::read(&local).unwrap(),
            b"private data created at edit pause\n"
        );
        fs::remove_file(&local).unwrap();
        assert!(matches!(
            follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), original);
        assert_eq!(fs::read(&unrelated).unwrap(), b"unrelated ignored data\n");
    }

    #[test]
    fn interactive_amend_checks_dirty_and_stale_branches_and_preserves_conflict_recovery() {
        let f = Fixture::new();
        let root = f.commit("base.txt", "root\n", "Root");
        let target = f.commit("conflict.txt", "from target\n", "Target");
        let tip = f.commit("conflict.txt", "later version\n", "Later");
        let confirmation = |head: &str| amend::Confirmation {
            head: head.into(),
            branch: "main".into(),
            message: None,
            acknowledge_published: false,
        };
        assert!(amend::start(
            &f.path,
            &target,
            amend::AmendAction::Drop,
            confirmation(&root)
        )
        .unwrap_err()
        .contains("HEAD or branch changed"));
        fs::write(Path::new(&f.path).join("conflict.txt"), "local contents\n").unwrap();
        assert!(amend::start(
            &f.path,
            &target,
            amend::AmendAction::Drop,
            confirmation(&tip)
        )
        .unwrap_err()
        .contains("Commit or stash"));
        assert_eq!(
            fs::read_to_string(Path::new(&f.path).join("conflict.txt")).unwrap(),
            "local contents\n"
        );
        f.git(&["restore", "--worktree", "--", "conflict.txt"]);
        assert!(matches!(
            perform_amend(&f, &target, amend::AmendAction::Edit, None).unwrap(),
            Outcome::Editing
        ));
        fs::write(Path::new(&f.path).join("conflict.txt"), "edited at pause\n").unwrap();
        f.git(&["add", "conflict.txt"]);
        super::super::git_service::GitService::amend_commit(&f.path, "Edited", false, false)
            .unwrap();
        assert!(matches!(
            follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
            Outcome::Conflict
        ));
        assert!(matches!(
            follow_up(&f.path, FollowUp::Abort, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), tip);
        assert!(f.git(&["status", "--porcelain"]).is_empty());
        assert!(matches!(
            perform_amend(
                &f,
                &root,
                amend::AmendAction::Reword,
                Some("Root was reworded")
            )
            .unwrap(),
            Outcome::Completed
        ));
        assert_eq!(
            f.git(&["show", "-s", "--format=%s", "HEAD^^"]),
            "Root was reworded"
        );
    }

    #[test]
    fn last_commit_removal_preserves_modified_deleted_and_renamed_working_files() {
        use super::super::{amend_selection::Removal, git_service::GitService};
        let f = Fixture::new();
        f.git(&["config", "core.autocrlf", "false"]);
        f.commit("modified.txt", "original\n", "Original content");
        f.commit("rename.txt", "rename contents\n", "Add rename source");
        f.commit(
            "delete.txt",
            "should stay deleted on disk\n",
            "Add deleted file",
        );
        fs::write(
            Path::new(&f.path).join("modified.txt"),
            "committed update\n",
        )
        .unwrap();
        f.git(&["mv", "rename.txt", "renamed.txt"]);
        f.git(&["rm", "delete.txt"]);
        f.git(&["add", "modified.txt"]);
        f.git(&["commit", "-m", "Review selective removal"]);
        let head = f.git(&["rev-parse", "HEAD"]);
        fs::write(
            Path::new(&f.path).join("modified.txt"),
            "new unstaged change\n",
        )
        .unwrap();
        let removal = Removal {
            head,
            paths: vec![
                "modified.txt".into(),
                "renamed.txt".into(),
                "delete.txt".into(),
            ],
        };
        GitService::amend_with_removal(
            &f.path,
            "Keep the changes for later",
            false,
            false,
            removal,
            false,
            None,
        )
        .unwrap();
        assert_eq!(f.git(&["show", "HEAD:modified.txt"]), "original");
        assert_eq!(f.git(&["show", "HEAD:rename.txt"]), "rename contents");
        assert_eq!(
            f.git(&["show", "HEAD:delete.txt"]),
            "should stay deleted on disk"
        );
        assert!(!Path::new(&f.path).join("rename.txt").exists());
        assert!(!Path::new(&f.path).join("delete.txt").exists());
        assert_eq!(
            fs::read_to_string(Path::new(&f.path).join("modified.txt")).unwrap(),
            "new unstaged change\n"
        );
        assert_eq!(
            fs::read_to_string(Path::new(&f.path).join("renamed.txt")).unwrap(),
            "rename contents\n"
        );
        assert!(f.git(&["diff", "--cached", "--name-only"]).is_empty());
    }

    #[test]
    fn patch_import_refuses_dirty_overlaps_and_unsafe_paths() {
        use super::super::patches::{self, ExportScope};
        let f = Fixture::new();
        f.git(&["config", "core.autocrlf", "false"]);
        let head = f.commit("notes.txt", "base\n", "Base");
        let temp = tempfile::tempdir().unwrap();
        let patch = temp.path().join("notes.patch");
        fs::write(Path::new(&f.path).join("notes.txt"), "patch contents\n").unwrap();
        patches::export(&f.path, &patch, None, ExportScope::Unstaged).unwrap();
        fs::write(Path::new(&f.path).join("notes.txt"), "local changes stay\n").unwrap();
        assert!(patches::import(&f.path, &patch, false, Some(&head))
            .unwrap_err()
            .contains("No changes were made"));
        assert_eq!(
            fs::read_to_string(Path::new(&f.path).join("notes.txt")).unwrap(),
            "local changes stay\n"
        );
        let attack = "diff --git a/../gitswamp-unwanted.txt b/../gitswamp-unwanted.txt\nnew file mode 100644\n--- /dev/null\n+++ b/../gitswamp-unwanted.txt\n@@ -0,0 +1 @@\n+unsafe\n";
        fs::write(&patch, attack).unwrap();
        assert!(patches::import(&f.path, &patch, false, Some(&head)).is_err());
        fs::write(
            &patch,
            attack.replace("../gitswamp-unwanted", ".git/gitswamp-unwanted"),
        )
        .unwrap();
        assert!(patches::import(&f.path, &patch, false, Some(&head)).is_err());
        assert!(!Path::new(&f.path)
            .join(".git/gitswamp-unwanted.txt")
            .exists());
    }

    #[test]
    #[ignore = "set GITSWAMP_POKUSAJ to the local pokusaj integration-test repository"]
    fn visual_rebase_on_pokusaj_history() {
        let source = std::env::var("GITSWAMP_POKUSAJ").expect("GITSWAMP_POKUSAJ");
        let f = Fixture::clone_local(&source);
        let base = f.git(&["rev-parse", "c85386c"]);
        let master = f.git(&["rev-parse", "a141a40"]);
        let branch = f.git(&["rev-parse", "0845684"]);
        f.git(&["switch", "-c", "verify-rebase", &branch]);
        f.git(&["branch", "must-not-move", &master]);
        let mut rename = step(&branch, Action::Reword);
        rename.message = Some("Reworded on pokusaj\n\nPreserved body".into());
        assert!(matches!(
            f.start(&base, &[rename, step(&master, Action::Edit)])
                .unwrap(),
            Outcome::Editing
        ));
        let info = super::super::git_service::GitService::repo_info(&f.path).unwrap();
        assert!(info.operation.unwrap().visual_rebase_editing);
        assert!(
            super::super::git_service::GitService::checkout_branch(&f.path, "must-not-move")
                .is_err()
        );
        fs::write(Path::new(&f.path).join("master-test.txt"), "Rebase edit\n").unwrap();
        f.git(&["add", "master-test.txt"]);
        super::super::git_service::GitService::amend_commit(
            &f.path,
            "Amended on pokusaj",
            false,
            false,
        )
        .unwrap();
        assert!(matches!(
            follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(
            f.git(&["log", "-2", "--format=%s"]),
            "Amended on pokusaj\nReworded on pokusaj"
        );
        assert_eq!(f.git(&["rev-parse", "HEAD^^"]), base);
        assert_eq!(f.git(&["rev-parse", "must-not-move"]), master);
        assert!(f.git(&["status", "--porcelain"]).is_empty());

        let original_base = f.git(&["rev-parse", "db60555"]);
        f.git(&["switch", "-c", "verify-conflict", &base]);
        // Replay an existing README change after a different version of the same region.
        fs::write(
            Path::new(&f.path).join("README.md"),
            "second test version\n",
        )
        .unwrap();
        f.git(&["add", "README.md"]);
        f.git(&["commit", "-m", "Second conflicting change"]);
        let tip = f.git(&["rev-parse", "HEAD"]);
        let steps = [step(&tip, Action::Pick), step(&base, Action::Drop)];
        assert!(matches!(
            f.start(&original_base, &steps).unwrap(),
            Outcome::Conflict
        ));
        assert!(
            super::super::git_service::GitService::repo_info(&f.path)
                .unwrap()
                .operation
                .unwrap()
                .visual_rebase
        );
        assert!(follow_up(&f.path, FollowUp::Continue, false, None).is_err());
        assert!(matches!(
            follow_up(&f.path, FollowUp::Abort, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), tip);
        assert!(matches!(
            f.start(&original_base, &steps).unwrap(),
            Outcome::Conflict
        ));
        fs::write(
            Path::new(&f.path).join("README.md"),
            "Resolved on pokusaj\n",
        )
        .unwrap();
        f.git(&["add", "README.md"]);
        assert!(matches!(
            follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["show", "HEAD:README.md"]), "Resolved on pokusaj");
        f.git(&["reset", "--hard", &tip]);
        assert!(matches!(
            f.start(&original_base, &steps).unwrap(),
            Outcome::Conflict
        ));
        assert!(matches!(
            follow_up(&f.path, FollowUp::Skip, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), original_base);
        assert!(plan(&f.path, &original_base).is_err());
    }

    #[test]
    fn visual_rebase_reorders_rewords_squashes_fixes_and_drops_only_the_selected_range() {
        let f = Fixture::new();
        let base = f.commit("base", "root", "Base");
        let a = f.commit("a", "A", "Original A");
        let b = f.commit("b", "B", "Original B");
        let c = f.commit("c", "C", "Original C");
        let d = f.commit("d", "D", "Original D");
        let e = f.commit("e", "E", "Original E");
        f.git(&["branch", "untouched", &c]);
        f.git(&["config", "rebase.updateRefs", "true"]);
        let mut first = step(&b, Action::Reword);
        first.message = Some("Renamed B\n\n# Preserved body".into());
        let mut group = step(&a, Action::Pick);
        group.combined_message = Some("Combined A, C and D\n\nBody".into());
        let steps = [
            first,
            group,
            step(&c, Action::Squash),
            step(&e, Action::Drop),
            step(&d, Action::Fixup),
        ];
        assert!(matches!(
            f.start(&base, &steps).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(
            f.git(&["log", "-3", "--format=%s"]),
            "Combined A, C and D\nRenamed B\nBase"
        );
        assert_eq!(
            f.git(&["show", "-s", "--format=%b", "HEAD^"]),
            "# Preserved body"
        );
        assert_eq!(f.git(&["rev-parse", "HEAD^^"]), base);
        assert_eq!(f.git(&["rev-parse", "untouched"]), c);
        assert!(!Path::new(&f.path).join("e").exists());
        assert!(Path::new(&f.path).join("d").exists());
    }

    #[test]
    fn visual_rebase_replays_the_branch_onto_an_ahead_branch() {
        let f = Fixture::new();
        let base = f.commit("base", "base", "Base");
        let a = f.commit("a", "A", "A");
        f.git(&["checkout", "-b", "ahead", &base]);
        let other = f.commit("other", "Other", "Other");
        f.git(&["update-ref", "refs/remotes/origin/main", &other]);
        f.git(&["checkout", "main"]);
        assert!(matches!(
            f.start(&other, &[step(&a, Action::Pick)]).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD^"]), other);
        assert_eq!(f.git(&["log", "-3", "--format=%s"]), "A\nOther\nBase");
        assert_eq!(f.git(&["show", "HEAD:a"]), "A");
        assert_eq!(f.git(&["show", "HEAD:other"]), "Other");
        assert_eq!(f.git(&["rev-parse", "ahead"]), other);
        assert_eq!(f.git(&["rev-parse", "refs/remotes/origin/main"]), other);
    }

    #[test]
    fn visual_rebase_fast_forwards_a_behind_only_branch_and_preserves_ignored_files() {
        let f = Fixture::new();
        let base = f.commit(".gitignore", "ignored.txt\n", "Base");
        f.git(&["checkout", "-b", "ahead", &base]);
        fs::write(Path::new(&f.path).join("ignored.txt"), "Tracked remotely").unwrap();
        f.git(&["add", "-f", "ignored.txt"]);
        f.git(&["commit", "-m", "Remote change"]);
        let remote = f.git(&["rev-parse", "HEAD"]);
        f.git(&["update-ref", "refs/remotes/origin/main", &remote]);
        f.git(&["checkout", "main"]);
        let reviewed = plan(&f.path, &remote).unwrap();
        assert!(reviewed.fast_forward);
        assert!(reviewed.commits.is_empty());
        let file = Path::new(&f.path).join("ignored.txt");
        fs::write(&file, "Keep this local file").unwrap();
        assert!(f.start(&remote, &[]).is_err());
        assert_eq!(f.git(&["rev-parse", "HEAD"]), base);
        assert_eq!(fs::read_to_string(&file).unwrap(), "Keep this local file");
        fs::remove_file(file).unwrap();
        assert!(matches!(
            f.start(&remote, &[]).unwrap(),
            Outcome::FastForwarded
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), remote);
        assert_eq!(f.git(&["symbolic-ref", "--short", "HEAD"]), "main");
        assert_eq!(f.git(&["rev-parse", "refs/remotes/origin/main"]), remote);
        assert!(!is_active(&GitRepository::open(&f.path).unwrap()));
    }

    #[test]
    fn visual_rebase_onto_origin_resolves_conflicts_without_moving_remote_tip() {
        let f = Fixture::new();
        let base = f.commit("notes.txt", "Base\n", "Base");
        let local = f.commit("notes.txt", "Local\n", "Local edit");
        f.git(&["checkout", "-b", "ahead", &base]);
        let remote = f.commit("notes.txt", "Remote\n", "Remote edit");
        f.git(&["update-ref", "refs/remotes/origin/main", &remote]);
        f.git(&["checkout", "main"]);
        let steps = [step(&local, Action::Pick)];
        assert!(matches!(
            f.start(&remote, &steps).unwrap(),
            Outcome::Conflict
        ));
        assert!(matches!(
            follow_up(&f.path, FollowUp::Abort, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), local);
        assert!(matches!(
            f.start(&remote, &steps).unwrap(),
            Outcome::Conflict
        ));
        fs::write(Path::new(&f.path).join("notes.txt"), "Resolved\n").unwrap();
        f.git(&["add", "notes.txt"]);
        assert!(matches!(
            follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD^"]), remote);
        assert_eq!(f.git(&["rev-parse", "refs/remotes/origin/main"]), remote);
        assert_eq!(f.git(&["show", "HEAD:notes.txt"]), "Resolved");
    }

    #[test]
    fn visual_rebase_can_drop_every_commit() {
        let f = Fixture::new();
        let base = f.commit("base", "base", "Base");
        let a = f.commit("a", "a", "A");
        assert!(matches!(
            f.start(&base, &[step(&a, Action::Drop)]).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), base);
        assert!(!Path::new(&f.path).join("a").exists());
    }

    #[test]
    fn visual_rebase_edit_can_continue_after_amend_and_abort_restores_original_tip() {
        let f = Fixture::new();
        let base = f.commit("base", "root", "Base");
        let a = f.commit("a", "A", "Edit me");
        let b = f.commit("b", "B", "Next");
        let steps = [step(&a, Action::Edit), step(&b, Action::Pick)];
        assert!(matches!(f.start(&base, &steps).unwrap(), Outcome::Editing));
        assert!(is_active(&GitRepository::open(&f.path).unwrap()));
        assert!(can_amend(&GitRepository::open(&f.path).unwrap()));
        assert!(matches!(
            follow_up(&f.path, FollowUp::Abort, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "main"]), b);
        assert!(matches!(f.start(&base, &steps).unwrap(), Outcome::Editing));
        fs::write(Path::new(&f.path).join("a"), "Edited").unwrap();
        f.git(&["add", "a"]);
        super::super::git_service::GitService::amend_commit(&f.path, "Edited commit", false, false)
            .unwrap();
        assert!(matches!(
            follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(
            f.git(&["log", "-1", "--format=%s", "HEAD^"]),
            "Edited commit"
        );
        assert_eq!(f.git(&["show", "HEAD:a"]), "Edited");
        assert_eq!(f.git(&["symbolic-ref", "--short", "HEAD"]), "main");
    }

    #[test]
    fn visual_rebase_conflict_can_resume_with_reword_and_skip() {
        let f = Fixture::new();
        let base = f.commit("file", "base\n", "Base");
        let a = f.commit("file", "first\n", "First");
        let b = f.commit("file", "second\n", "Second");
        let mut reword = step(&b, Action::Reword);
        reword.message = Some("Second, resolved".into());
        let steps = [reword, step(&a, Action::Drop)];
        assert!(matches!(f.start(&base, &steps).unwrap(), Outcome::Conflict));
        fs::write(Path::new(&f.path).join("file"), "resolved\n").unwrap();
        f.git(&["add", "file"]);
        assert!(matches!(
            follow_up(&f.path, FollowUp::Continue, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["log", "-1", "--format=%s"]), "Second, resolved");
        f.git(&["reset", "--hard", &b]);
        assert!(matches!(f.start(&base, &steps).unwrap(), Outcome::Conflict));
        assert!(matches!(
            follow_up(&f.path, FollowUp::Skip, false, None).unwrap(),
            Outcome::Completed
        ));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), base);
    }

    #[test]
    fn visual_rebase_refuses_dirty_stale_invalid_and_non_linear_plans() {
        let f = Fixture::new();
        let base = f.commit("base", "base", "Base");
        let a = f.commit("a", "a", "A");
        assert!(f
            .start(&base, &[step(&a, Action::Fixup)])
            .unwrap_err()
            .contains("preceding"));
        assert!(
            start(&f.path, &base, &base, "main", &[step(&a, Action::Drop)])
                .unwrap_err()
                .contains("HEAD moved")
        );
        fs::write(Path::new(&f.path).join("dirty"), "keep me").unwrap();
        assert!(f
            .start(&base, &[step(&a, Action::Drop)])
            .unwrap_err()
            .contains("stash"));
        f.git(&["checkout", "-b", "other", &base]);
        let other = f.commit("other", "other", "Other");
        f.git(&["checkout", "main"]);
        let onto_plan = plan(&f.path, &other).unwrap();
        assert_eq!(onto_plan.onto, other);
        assert_eq!(onto_plan.upstream, base);
        assert_eq!(
            onto_plan
                .commits
                .iter()
                .map(|commit| commit.sha.as_str())
                .collect::<Vec<_>>(),
            vec![a.as_str()]
        );
        f.git(&["merge", "--no-ff", "--no-edit", "other"]);
        assert!(plan(&f.path, &base).err().unwrap().contains("merge commit"));
    }
}
