use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use git2::{BranchType, Oid, Repository, RepositoryState, Sort};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::models::{CommitInfo, FileStatusInfo, RepositoryOperationInfo};
use crate::repositories::git_repository::GitRepository;
use crate::services::{
    git_service::GitService, helpers::time_ago, secret_service::validate_commit,
};

const PAGE_SIZE: usize = 60;
static OPEN_WORKTREE: Mutex<()> = Mutex::new(());

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLocation {
    path: String,
    branch: String,
    is_primary: bool,
}

struct WorktreeEntry {
    path: PathBuf,
    branch: Option<String>,
}

fn location_string(path: &Path) -> Result<String, String> {
    let path = path.to_str().ok_or("Worktree path is not valid UTF-8.")?;
    Ok(if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    })
}

fn registered_worktrees(output: &str) -> Vec<WorktreeEntry> {
    output
        .split("\0\0")
        .filter_map(|record| {
            let mut fields = record.split('\0');
            let path = fields.next()?.strip_prefix("worktree ")?;
            Some(WorktreeEntry {
                path: PathBuf::from(path),
                branch: fields.find_map(|field| field.strip_prefix("branch ").map(str::to_owned)),
            })
        })
        .collect()
}

fn checked_out_tip(repo: &Repository, branch: &str) -> Result<Oid, String> {
    let expected = format!("refs/heads/{branch}");
    let head = repo.head().map_err(|error| error.message().to_owned())?;
    if head.name() != Some(expected.as_str()) {
        return Err(format!("This worktree is no longer on '{branch}'. Open it in a repository tab or switch it back before editing."));
    }
    head.target()
        .ok_or_else(|| "The branch has no commits yet.".to_owned())
}

pub fn validate_checkout(path: &str, branch: Option<&str>) -> Result<(), String> {
    if let Some(branch) = branch {
        checked_out_tip(&GitRepository::open(path)?, branch)?;
    }
    Ok(())
}

pub fn ensure_branch_available(repo: &Repository, branch: &str) -> Result<(), String> {
    let expected = format!("refs/heads/{branch}");
    if repo
        .head()
        .ok()
        .and_then(|head| head.name().map(str::to_owned))
        .as_deref()
        == Some(&expected)
        || (!repo.is_worktree()
            && repo
                .worktrees()
                .map_err(|error| error.to_string())?
                .is_empty())
    {
        return Ok(());
    }
    let path = repo
        .workdir()
        .and_then(Path::to_str)
        .ok_or("No working directory.")?;
    let list = GitRepository::git_cli(path, &["worktree", "list", "--porcelain", "-z"])?;
    if let Some(entry) = registered_worktrees(&list)
        .into_iter()
        .find(|entry| entry.branch.as_deref() == Some(&expected))
    {
        return Err(format!("Branch '{branch}' is already checked out at {}. Open that Branch Workspace or repository tab instead. No files were changed.", entry.path.display()));
    }
    Ok(())
}

pub fn open(path: &str, branch_ref: &str) -> Result<WorkspaceLocation, String> {
    let _guard = OPEN_WORKTREE
        .lock()
        .map_err(|_| "Could not lock worktree creation.")?;
    let repo = GitRepository::open(path)?;
    let (branch, tracking) = if let Some(branch) = branch_ref.strip_prefix("refs/heads/") {
        repo.find_branch(branch, BranchType::Local)
            .map_err(|_| format!("Local branch '{branch}' was not found."))?;
        (branch, None)
    } else if let Some(remote_ref) = branch_ref.strip_prefix("refs/remotes/") {
        let (_, branch) = remote_ref
            .split_once('/')
            .ok_or("Invalid remote branch reference.")?;
        if branch == "HEAD"
            || !git2::Branch::name_is_valid(branch).map_err(|error| error.to_string())?
        {
            return Err("Select a remote branch, not the remote's HEAD reference.".to_owned());
        }
        repo.find_reference(branch_ref)
            .and_then(|reference| reference.peel_to_commit())
            .map_err(|_| format!("Remote branch '{remote_ref}' was not found."))?;
        match repo.find_branch(branch, BranchType::Local) {
            Ok(local) => {
                if local
                    .upstream()
                    .ok()
                    .and_then(|upstream| upstream.get().name().map(str::to_owned))
                    .as_deref()
                    != Some(branch_ref)
                {
                    return Err(format!("Local branch '{branch}' already exists with a different upstream. Open that local branch separately."));
                }
                (branch, None)
            }
            Err(error) if error.code() == git2::ErrorCode::NotFound => (branch, Some(branch_ref)),
            Err(error) => return Err(error.to_string()),
        }
    } else {
        return Err("Select a local or remote branch reference.".to_owned());
    };

    let list = GitRepository::git_cli(path, &["worktree", "list", "--porcelain", "-z"])?;
    let worktrees = registered_worktrees(&list);
    let main = worktrees
        .first()
        .ok_or("Git did not return the repository's main worktree.")?;
    let expected = format!("refs/heads/{branch}");
    if let Some(entry) = worktrees
        .iter()
        .find(|entry| entry.branch.as_deref() == Some(&expected))
    {
        let location = location_string(&entry.path)?;
        let existing = GitRepository::open(&location).map_err(|_| format!("The worktree for '{branch}' is registered at {location} but cannot be opened. Repair or move it back before reopening."))?;
        checked_out_tip(&existing, branch)?;
        return Ok(WorkspaceLocation {
            path: location,
            branch: branch.to_owned(),
            is_primary: entry.path == main.path,
        });
    }

    // Keep linked checkouts outside the original working directory and never overwrite a folder.
    let parent = main
        .path
        .parent()
        .ok_or("The main worktree has no parent directory.")?;
    let storage = parent.join(".gitswamp-worktrees");
    let hash = format!(
        "{:x}",
        Sha256::digest(format!("{}\0{expected}", main.path.display()))
    );
    let destination = storage.join(&hash[..16]);
    if Path::symlink_metadata(&destination).is_ok() {
        return Err(format!(
            "Workspace path already exists at {}. It was left untouched.",
            destination.display()
        ));
    }
    std::fs::create_dir_all(&storage)
        .map_err(|error| format!("Could not create the worktree directory: {error}"))?;
    let location = location_string(&destination)?;
    if let Some(upstream) = tracking {
        GitRepository::git_cli(
            path,
            &[
                "worktree", "add", "--track", "-b", branch, "--", &location, upstream,
            ],
        )?;
    } else {
        GitRepository::git_cli(
            path,
            &[
                "worktree",
                "add",
                "--no-guess-remote",
                "--",
                &location,
                branch,
            ],
        )?;
    }
    checked_out_tip(&GitRepository::open(&location)?, branch)?;
    Ok(WorkspaceLocation {
        path: location,
        branch: branch.to_owned(),
        is_primary: false,
    })
}

pub fn remove(path: &str, worktree_path: &str, branch: &str) -> Result<(), String> {
    let _guard = OPEN_WORKTREE
        .lock()
        .map_err(|_| "Could not lock the worktree registry.")?;
    let source = GitRepository::open(path)?;
    let source_root = source
        .workdir()
        .ok_or("The source repository has no working directory.")?;
    let source_root = source_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let target = Path::new(worktree_path)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let list = GitRepository::git_cli(path, &["worktree", "list", "--porcelain", "-z"])?;
    let entries = registered_worktrees(&list);
    let main = entries
        .first()
        .ok_or("Git did not return the main worktree.")?;
    let primary = main
        .path
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if target == primary || target == source_root {
        return Err("The original worktree cannot be removed here.".to_owned());
    }
    let expected = format!("refs/heads/{branch}");
    let entry = entries
        .iter()
        .skip(1)
        .find(|entry| {
            entry.branch.as_deref() == Some(&expected)
                && entry.path.canonicalize().ok().as_ref() == Some(&target)
        })
        .ok_or(
            "The worktree no longer matches this branch or is not registered in this repository.",
        )?;
    let location = location_string(&entry.path)?;
    let worktree = GitRepository::open(&location)?;
    checked_out_tip(&worktree, branch)?;
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).include_ignored(true);
    if worktree.state() != RepositoryState::Clean
        || !worktree
            .statuses(Some(&mut options))
            .map_err(|error| error.to_string())?
            .is_empty()
    {
        return Err("This worktree has staged, unstaged, untracked or ignored files, or an active Git operation. Keep the worktree and save or move its files before removing it.".to_owned());
    }
    // Git owns the registry and performs a final check. Never force removal.
    GitRepository::git_cli(path, &["worktree", "remove", "--", &location]).map(|_| ())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStatus {
    head_sha: String,
    operation: Option<RepositoryOperationInfo>,
    files: Vec<FileStatusInfo>,
}

pub fn status(path: &str, branch: &str) -> Result<WorkspaceStatus, String> {
    let repo = GitRepository::open(path)?;
    let head = checked_out_tip(&repo, branch)?;
    Ok(WorkspaceStatus {
        head_sha: head.to_string(),
        operation: GitService::repository_operation(&repo, branch),
        files: GitService::status(path)?,
    })
}

pub fn history(path: &str, tip: &str, offset: usize) -> Result<Vec<CommitInfo>, String> {
    let repo = GitRepository::open(path)?;
    let mut walk = repo.revwalk().map_err(|error| error.to_string())?;
    let tip = Oid::from_str(tip).map_err(|error| error.to_string())?;
    walk.set_sorting(Sort::TIME)
        .map_err(|error| error.to_string())?;
    walk.push(tip).map_err(|error| error.to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    walk.skip(offset)
        .take(PAGE_SIZE + 1)
        .map(|item| {
            let commit = repo
                .find_commit(item.map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?;
            let sha = commit.id().to_string();
            let timestamp = commit.time().seconds();
            let author = commit.author();
            let committer = commit.committer();
            Ok(CommitInfo {
                short_sha: sha[..7].to_owned(),
                sha,
                message: commit
                    .message()
                    .unwrap_or_default()
                    .trim()
                    .chars()
                    .take(4096)
                    .collect(),
                author_name: author.name().unwrap_or("Unknown").to_owned(),
                author_email: author.email().unwrap_or_default().to_owned(),
                committer_name: committer.name().unwrap_or("Unknown").to_owned(),
                committer_email: committer.email().unwrap_or_default().to_owned(),
                timestamp,
                time_ago: time_ago(now, timestamp),
                parent_shas: commit
                    .parent_ids()
                    .map(|parent| parent.to_string())
                    .collect(),
                refs: Vec::new(),
            })
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum WorkspaceAction {
    Stage {
        paths: Vec<String>,
    },
    Unstage {
        paths: Vec<String>,
    },
    Discard {
        paths: Vec<String>,
    },
    Commit {
        message: String,
        allow_empty: bool,
        secret_check: bool,
        secret_approval: Option<String>,
    },
    Resolve {
        file_path: Option<String>,
        strategy: String,
    },
    Abort {
        operation: String,
    },
}

pub fn mutate(path: &str, branch: &str, action: WorkspaceAction) -> Result<(), String> {
    let repo = GitRepository::open(path)?;
    checked_out_tip(&repo, branch)?;
    match action {
        WorkspaceAction::Stage { paths } => GitService::stage_files(path, &paths),
        WorkspaceAction::Unstage { paths } => GitService::unstage_files(path, &paths),
        WorkspaceAction::Discard { paths } => {
            if repo.state() != RepositoryState::Clean {
                return Err(
                    "Complete or abort the active Git operation before discarding files."
                        .to_owned(),
                );
            }
            GitService::discard_files(path, &paths)
        }
        WorkspaceAction::Commit {
            message,
            allow_empty,
            secret_check,
            secret_approval,
        } => {
            if !matches!(
                repo.state(),
                RepositoryState::Clean | RepositoryState::Merge | RepositoryState::CherryPick
            ) {
                return Err(
                    "Complete the active Git operation in a repository tab before committing."
                        .to_owned(),
                );
            }
            validate_commit(path, secret_check, secret_approval.as_deref())?;
            GitService::create_commit_with_options(path, &message, allow_empty).map(|_| ())
        }
        WorkspaceAction::Resolve {
            file_path,
            strategy,
        } => match file_path {
            Some(file) => GitService::resolve_conflict_file(path, &file, &strategy),
            None => GitService::resolve_all_conflicts(path, &strategy),
        },
        WorkspaceAction::Abort { operation } => match (operation.as_str(), repo.state()) {
            ("merge", RepositoryState::Merge) => GitService::abort_merge(path, false).map(|_| ()),
            ("cherry-pick", RepositoryState::CherryPick | RepositoryState::CherryPickSequence) => {
                GitService::abort_cherry_pick(path).map(|_| ())
            }
            _ => Err("The scheduled operation is no longer active.".to_owned()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("worktree-tests")
                .join(nonce.to_string());
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn commit(repo: &Repository, message: &str, parents: &[&git2::Commit<'_>]) -> Oid {
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            parents,
        )
        .unwrap()
    }

    #[test]
    fn workspaces_keep_separate_heads_indexes_and_untracked_files_when_reopened() {
        let fixture = Fixture::new();
        let main_path = fixture.0.join("source repository");
        let repo = Repository::init(&main_path).unwrap();
        let main = main_path.to_str().unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let base = commit(&repo, "Base", &[]);
        let base_commit = repo.find_commit(base).unwrap();
        let topic = repo.branch("feature/topic", &base_commit, false).unwrap();
        repo.reference("refs/tags/feature/topic", base, false, "test")
            .unwrap();
        let primary = open(main, "refs/heads/main").unwrap();
        assert!(primary.is_primary);
        std::fs::write(main_path.join("main-only.txt"), "main draft").unwrap();

        let workspace = open(main, "refs/heads/feature/topic").unwrap();
        assert!(!workspace.is_primary);
        let path = Path::new(&workspace.path);
        assert!(!path.starts_with(&main_path));
        assert!(!path.join("main-only.txt").exists());
        std::fs::write(path.join("staged.txt"), "staged version").unwrap();
        assert!(mutate(
            &workspace.path,
            "main",
            WorkspaceAction::Stage {
                paths: vec!["staged.txt".into()]
            }
        )
        .is_err());
        mutate(
            &workspace.path,
            "feature/topic",
            WorkspaceAction::Stage {
                paths: vec!["staged.txt".into()],
            },
        )
        .unwrap();
        std::fs::write(path.join("staged.txt"), "unstaged version").unwrap();
        std::fs::write(path.join("untracked.txt"), "local draft").unwrap();
        let reopened = open(main, "refs/heads/feature/topic").unwrap();
        assert_eq!(workspace.path, reopened.path);
        let saved = status(&reopened.path, "feature/topic").unwrap();
        assert!(saved
            .files
            .iter()
            .any(|file| file.path == "staged.txt" && file.staged));
        assert!(saved
            .files
            .iter()
            .any(|file| file.path == "staged.txt" && !file.staged));
        assert!(saved
            .files
            .iter()
            .any(|file| file.path == "untracked.txt" && !file.staged));
        assert_eq!(status(main, "main").unwrap().files.len(), 1);

        repo.config().unwrap().set_str("user.name", "Test").unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "test@example.com")
            .unwrap();
        mutate(
            &workspace.path,
            "feature/topic",
            WorkspaceAction::Commit {
                message: "Feature change".into(),
                allow_empty: false,
                secret_check: false,
                secret_approval: None,
            },
        )
        .unwrap();
        let branch_tip = status(&workspace.path, "feature/topic").unwrap().head_sha;
        let feature_history = history(&workspace.path, &branch_tip, 0).unwrap();
        assert_eq!(
            feature_history
                .iter()
                .map(|item| item.message.as_str())
                .collect::<Vec<_>>(),
            ["Feature change", "Base"]
        );
        assert_eq!(history(main, &base.to_string(), 0).unwrap().len(), 1);
        assert!(GitService::commits(main, 20)
            .unwrap()
            .iter()
            .any(|item| item.sha == branch_tip
                && item.refs.iter().any(|name| name == "feature/topic")));
        assert_eq!(repo.head().unwrap().name(), Some("refs/heads/main"));
        assert_eq!(repo.refname_to_id("refs/heads/main").unwrap(), base);
        assert!(GitService::checkout_branch(main, "feature/topic")
            .unwrap_err()
            .contains("already checked out"));
        assert!(!main_path.join("staged.txt").exists());
        assert_eq!(topic.get().name(), Some("refs/heads/feature/topic"));
        assert_eq!(
            std::fs::read_to_string(main_path.join("main-only.txt")).unwrap(),
            "main draft"
        );

        repo.remote("origin", "https://example.invalid/project.git")
            .unwrap();
        repo.reference("refs/remotes/origin/remote-only", base, false, "test")
            .unwrap();
        let remote = open(main, "refs/remotes/origin/remote-only").unwrap();
        assert_eq!(remote.branch, "remote-only");
        let tracked = repo.find_branch("remote-only", BranchType::Local).unwrap();
        assert_eq!(
            tracked.upstream().unwrap().get().name(),
            Some("refs/remotes/origin/remote-only")
        );
        assert_eq!(
            open(main, "refs/heads/remote-only").unwrap().path,
            remote.path
        );
    }

    #[test]
    fn removal_protects_original_and_local_files_and_keeps_commits() {
        let fixture = Fixture::new();
        let main_path = fixture.0.join("main");
        let repo = Repository::init(&main_path).unwrap();
        let main = main_path.to_str().unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let base = commit(&repo, "Base", &[]);
        repo.branch("topic", &repo.find_commit(base).unwrap(), false)
            .unwrap();
        let linked = open(main, "refs/heads/topic").unwrap();
        let root = Path::new(&linked.path);
        assert!(remove(main, main, "main").is_err());
        assert!(remove(&linked.path, main, "main").is_err());
        assert!(remove(&linked.path, &linked.path, "topic").is_err());
        assert!(remove(main, &linked.path, "main").is_err());

        std::fs::write(root.join("draft.txt"), "local work").unwrap();
        assert!(remove(main, &linked.path, "topic")
            .unwrap_err()
            .contains("untracked"));
        assert_eq!(
            std::fs::read_to_string(root.join("draft.txt")).unwrap(),
            "local work"
        );
        let linked_repo = GitRepository::open(&linked.path).unwrap();
        GitService::stage_file(&linked.path, "draft.txt").unwrap();
        assert!(remove(main, &linked.path, "topic").is_err());
        let tip = commit(
            &linked_repo,
            "Keep this commit",
            &[&linked_repo.find_commit(base).unwrap()],
        );
        let exclude = repo.path().join("info").join("exclude");
        std::fs::write(exclude, "ignored.txt\n").unwrap();
        std::fs::write(root.join("ignored.txt"), "keep ignored files").unwrap();
        assert!(remove(main, &linked.path, "topic")
            .unwrap_err()
            .contains("ignored"));
        std::fs::remove_file(root.join("ignored.txt")).unwrap();
        drop(linked_repo);

        remove(main, &linked.path, "topic").unwrap();
        assert!(!root.exists());
        assert_eq!(repo.refname_to_id("refs/heads/topic").unwrap(), tip);
        assert_eq!(repo.head().unwrap().target(), Some(base));
        assert!(GitService::commits(main, 20)
            .unwrap()
            .iter()
            .any(|item| item.sha == tip.to_string()));
        assert!(GitService::checkout_branch(main, "topic").is_ok());
    }
}
