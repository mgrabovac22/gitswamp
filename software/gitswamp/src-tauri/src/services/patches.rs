use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Stdio;

use git2::{Oid, Repository, RepositoryState};
use serde::Deserialize;

use super::interactive_rebase;
use crate::repositories::git_repository::GitRepository as Git;

const MAX_IMPORT_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportScope {
    AllTracked,
    Staged,
    Unstaged,
}

fn failure(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn patch_path(path: &Path) -> Result<(), String> {
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| {
            ["patch", "diff"]
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
        })
    {
        return Err("Choose a .patch or .diff file.".into());
    }
    Ok(())
}

fn head(repo: &Repository) -> Option<Oid> {
    repo.head().ok().and_then(|reference| reference.target())
}

pub fn export(
    path: &str,
    destination: &Path,
    sha: Option<&str>,
    scope: ExportScope,
) -> Result<(), String> {
    patch_path(destination)?;
    let repo = Git::open(path)?;
    let destination = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        return Err("Select a destination using the file picker.".into());
    };
    let parent = fs::canonicalize(destination.parent().ok_or("Invalid destination folder.")?)
        .map_err(failure)?;
    let metadata = fs::canonicalize(repo.path()).map_err(failure)?;
    if parent.starts_with(metadata)
        || parent
            .components()
            .any(|part| part.as_os_str().eq_ignore_ascii_case(".git"))
    {
        return Err("Do not save a patch inside Git metadata.".into());
    }
    let mut arguments;
    let oid;
    if let Some(sha) = sha {
        let id = Oid::from_str(sha).map_err(|_| "Invalid commit SHA.")?;
        let commit = repo.find_commit(id).map_err(failure)?;
        if commit.parent_count() > 1 {
            return Err("A merge commit is not a single format-patch. Export the commits from its source branch instead.".into());
        }
        oid = id.to_string();
        arguments = vec![
            "format-patch",
            "-1",
            "--stdout",
            "--binary",
            "--full-index",
            "--no-ext-diff",
            "--no-textconv",
            "--no-signature",
            "--no-cover-letter",
            "--always",
            &oid,
            "--",
        ];
    } else {
        if repo.index().map_err(failure)?.has_conflicts() {
            return Err("Resolve conflicts before exporting working changes.".into());
        }
        arguments = vec![
            "diff",
            "--binary",
            "--full-index",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--no-relative",
            "--src-prefix=a/",
            "--dst-prefix=b/",
        ];
        match scope {
            ExportScope::AllTracked => {
                oid = head(&repo)
                    .map(Ok)
                    .unwrap_or_else(|| repo.treebuilder(None).and_then(|builder| builder.write()))
                    .map_err(failure)?
                    .to_string();
                arguments.push(&oid);
            }
            ExportScope::Staged => arguments.push("--cached"),
            ExportScope::Unstaged => {}
        }
        arguments.push("--");
    }
    let output = tempfile::NamedTempFile::new_in(parent).map_err(failure)?;
    let result = Git::git_command(path, &arguments, &[])
        .stdout(Stdio::from(output.reopen().map_err(failure)?))
        .output()
        .map_err(failure)?;
    if !result.status.success() {
        return Err(format!(
            "Git could not export this patch: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    if output.as_file().metadata().map_err(failure)?.len() == 0 {
        return Err("No tracked changes to export. Stage new files first.".into());
    }
    output.as_file().sync_all().map_err(failure)?;
    output.persist(destination).map_err(failure)?;
    Ok(())
}

pub fn import(
    path: &str,
    source: &Path,
    staged: bool,
    expected_head: Option<&str>,
) -> Result<(), String> {
    patch_path(source)?;
    let repo = Git::open(path)?;
    let expected = expected_head
        .map(Oid::from_str)
        .transpose()
        .map_err(failure)?;
    if head(&repo) != expected {
        return Err("HEAD changed. Reopen Import Patch for the current branch.".into());
    }
    if repo.index().map_err(failure)?.has_conflicts()
        || (repo.state() != RepositoryState::Clean
            && !(interactive_rebase::is_active(&repo) && interactive_rebase::can_amend(&repo)))
    {
        return Err("Finish the current Git operation before applying this patch.".into());
    }
    let file = File::open(source).map_err(failure)?;
    let stat = file.metadata().map_err(failure)?;
    if !stat.is_file() || stat.len() == 0 || stat.len() > MAX_IMPORT_BYTES {
        return Err("Choose a non-empty patch of at most 256 MiB.".into());
    }
    // Use the same bytes for both Git's check and its application, without holding them in RAM.
    let mut input = tempfile::NamedTempFile::new().map_err(failure)?;
    let size = io::copy(&mut file.take(MAX_IMPORT_BYTES + 1), &mut input).map_err(failure)?;
    if size > MAX_IMPORT_BYTES {
        return Err("The patch exceeds 256 MiB.".into());
    }
    input.flush().map_err(failure)?;
    let file = input
        .path()
        .to_str()
        .ok_or("The temporary patch path is not UTF-8.")?;
    let args = |operation| {
        let mut args = vec!["apply", operation, "--whitespace=nowarn"];
        if staged {
            args.push("--index");
        }
        args.extend(["--", file]);
        args
    };
    Git::git_cli_with_env(path, &args("--check"), &[]).map_err(|error| {
        format!("The patch does not apply to the current files. No changes were made.\n{error}")
    })?;
    if head(&repo) != expected {
        return Err("HEAD changed during the patch check. No changes were applied.".into());
    }
    Git::git_cli_with_env(path, &args("--apply"), &[]).map_err(|error| {
        format!("Git could not apply the patch. Review Working Changes before retrying.\n{error}")
    })?;
    Ok(())
}
