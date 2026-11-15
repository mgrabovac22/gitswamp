use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use git2::{Oid, Repository, RepositoryState, StatusOptions};
use serde::{Deserialize, Serialize};

use crate::repositories::git_repository::GitRepository;

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
            &["merge", "--ff-only", "--no-autostash", "--no-overwrite-ignore", &plan.onto],
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
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let comment = prepare(temp.path(), &plan, steps)?;
    let onto = plan.onto.as_str();
    let upstream = plan.upstream.as_str();
    // Pin the selected range and disable config that could change the plan or other refs.
    run(
        &repo,
        path,
        temp.path(),
        comment,
        &[
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
            "--onto",
            onto,
            upstream,
        ],
    )
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
        GitRepository::git_cli(path, &["rebase", "--abort"])?;
        return Ok(Outcome::Completed);
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
            fixture.git(&["clone", "--no-hardlinks", "--no-local", "--", source, "."]);
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
        assert!(matches!(f.start(&remote, &[]).unwrap(), Outcome::FastForwarded));
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
        assert!(matches!(f.start(&remote, &steps).unwrap(), Outcome::Conflict));
        assert!(matches!(follow_up(&f.path, FollowUp::Abort, false, None).unwrap(), Outcome::Completed));
        assert_eq!(f.git(&["rev-parse", "HEAD"]), local);
        assert!(matches!(f.start(&remote, &steps).unwrap(), Outcome::Conflict));
        fs::write(Path::new(&f.path).join("notes.txt"), "Resolved\n").unwrap();
        f.git(&["add", "notes.txt"]);
        assert!(matches!(follow_up(&f.path, FollowUp::Continue, false, None).unwrap(), Outcome::Completed));
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
