use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use git2::{ObjectType, Repository, Tree, TreeWalkMode, TreeWalkResult};

use super::{git_err, relative};

const MAX_SOURCE_BYTES: usize = 512 * 1024 * 1024;
const MAX_BLOB_BYTES: usize = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 60_000;
const LOG_BYTES: usize = 4096;
const COMMIT_TIMEOUT: Duration = Duration::from_secs(300);
const SERIES_TIMEOUT: Duration = Duration::from_secs(900);

pub(super) enum CheckError {
    Rejected(String),
    Unavailable(String),
}

impl CheckError {
    pub(super) fn message(self) -> String {
        match self {
            Self::Rejected(message) | Self::Unavailable(message) => message,
        }
    }
}

impl From<String> for CheckError {
    fn from(message: String) -> Self {
        Self::Unavailable(message)
    }
}

impl From<&str> for CheckError {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

pub(super) struct Verification<'a> {
    command: &'a str,
    deadline: Instant,
}

impl<'a> Verification<'a> {
    pub(super) fn new(command: &'a str) -> Result<Self, String> {
        let command = command.trim();
        if command.is_empty() || command.len() > 2048 || command.chars().any(char::is_control) {
            return Err("Enter a build command on one line (up to 2048 bytes).".into());
        }
        Ok(Self {
            command,
            deadline: Instant::now() + SERIES_TIMEOUT,
        })
    }

    pub(super) fn check(
        &self,
        repo: &Repository,
        tree: &Tree<'_>,
        number: usize,
    ) -> Result<(), CheckError> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("Build verification timed out before completing the series.".into());
        }
        let folder = tempfile::Builder::new()
            .prefix("gitswamp-verify-")
            .tempdir()
            .map_err(|err| err.to_string())?;
        export_tree(repo, tree, folder.path())
            .map_err(|error| format!("Commit {number}: {error}"))?;
        let timeout = self
            .deadline
            .saturating_duration_since(Instant::now())
            .min(COMMIT_TIMEOUT);
        run(self.command, folder.path(), timeout).map_err(|error| match error {
            CheckError::Rejected(message) => {
                CheckError::Rejected(format!("Commit {number}: {message}"))
            }
            CheckError::Unavailable(message) => {
                CheckError::Unavailable(format!("Commit {number}: {message}"))
            }
        })
    }
}

fn export_tree(repo: &Repository, tree: &Tree<'_>, destination: &Path) -> Result<(), String> {
    let odb = repo.odb().map_err(git_err)?;
    let mut entries = 0;
    let mut bytes = 0usize;
    let mut result = Ok(());
    tree.walk(TreeWalkMode::PreOrder, |parent, entry| {
        result = (|| {
            entries += 1;
            if entries > MAX_ENTRIES || parent.len() > 4096 {
                return Err("The build snapshot exceeds the checkout limit.".into());
            }
            let name = entry
                .name()
                .ok_or("Build verification requires UTF-8 paths.")?;
            let path = format!("{parent}{name}");
            let path = relative(path.as_bytes())?;
            let target = destination.join(path);
            match entry.filemode() {
                0o040000 => fs::create_dir(&target).map_err(|err| err.to_string()),
                mode @ (0o100644 | 0o100755) => {
                    let (size, kind) = odb.read_header(entry.id()).map_err(git_err)?;
                    bytes = bytes.saturating_add(size);
                    if kind != ObjectType::Blob || bytes > MAX_SOURCE_BYTES {
                        return Err("The build snapshot exceeds 512 MiB.".into());
                    }
                    if size > MAX_BLOB_BYTES {
                        return Err(format!(
                            "Build verification cannot export a file larger than 32 MiB: {path}"
                        ));
                    }
                    let blob = repo.find_blob(entry.id()).map_err(git_err)?;
                    let mut file = fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&target)
                        .map_err(|err| err.to_string())?;
                    std::io::Write::write_all(&mut file, blob.content())
                        .map_err(|err| err.to_string())?;
                    #[cfg(unix)]
                    if mode == 0o100755 {
                        use std::os::unix::fs::PermissionsExt;
                        file.set_permissions(fs::Permissions::from_mode(0o755))
                            .map_err(|err| err.to_string())?;
                    }
                    let _ = mode;
                    Ok(())
                }
                _ => Err(format!(
                    "Build verification cannot export a symlink or submodule: {path}"
                )),
            }
        })();
        if result.is_err() {
            TreeWalkResult::Abort
        } else {
            TreeWalkResult::Ok
        }
    })
    .map_err(git_err)?;
    result
}

fn retain_tail(mut reader: impl Read + Send + 'static) -> mpsc::Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut tail = Vec::with_capacity(LOG_BYTES);
        let mut chunk = [0; 2048];
        while let Ok(n) = reader.read(&mut chunk) {
            if n == 0 {
                break;
            }
            let excess = (tail.len() + n).saturating_sub(LOG_BYTES);
            if excess != 0 {
                tail.drain(..excess);
            }
            tail.extend_from_slice(&chunk[..n]);
        }
        let _ = tx.send(tail);
    });
    rx
}

fn run(command: &str, directory: &Path, timeout: Duration) -> Result<(), CheckError> {
    if timeout.is_zero() {
        return Err("Build verification timed out.".into());
    }
    #[cfg(windows)]
    let mut process = {
        use std::os::windows::process::CommandExt;
        let mut process = Command::new("cmd.exe");
        process
            .args(["/d", "/c"])
            .raw_arg(command)
            .creation_flags(crate::constants::CREATE_NO_WINDOW);
        process
    };
    #[cfg(unix)]
    let mut process = {
        use std::os::unix::process::CommandExt;
        let mut process = Command::new("/bin/sh");
        process.args(["-c", command]).process_group(0);
        process
    };
    let mut child = process
        .current_dir(directory)
        .env("CI", "true")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_COMMON_DIR")
        .env(
            "GIT_CEILING_DIRECTORIES",
            directory.parent().unwrap_or(directory),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Could not start build verification: {err}"))?;
    let stdout = retain_tail(
        child
            .stdout
            .take()
            .ok_or("Could not capture build output.")?,
    );
    let stderr = retain_tail(
        child
            .stderr
            .take()
            .ok_or("Could not capture build errors.")?,
    );
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < timeout => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                stop(&mut child);
                return Err("Build verification timed out.".into());
            }
            Err(error) => {
                stop(&mut child);
                return Err(format!("Could not monitor build verification: {error}").into());
            }
        }
    };
    if status.success() {
        return Ok(());
    }
    let mut output = stdout
        .recv_timeout(Duration::from_millis(150))
        .unwrap_or_default();
    let errors = stderr
        .recv_timeout(Duration::from_millis(150))
        .unwrap_or_default();
    if !output.is_empty() && !errors.is_empty() {
        output.push(b'\n');
    }
    output.extend(errors);
    let clean: String = String::from_utf8_lossy(&output[output.len().saturating_sub(LOG_BYTES)..])
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();
    let tail = clean.trim();
    let message = if tail.is_empty() {
        format!("Build verification failed ({status}).")
    } else {
        format!("Build verification failed ({status}).\n{tail}")
    };
    // A shell that cannot start the requested command cannot validate a regroup.
    match status.code() {
        None | Some(126 | 127 | 9009) => Err(CheckError::Unavailable(message)),
        _ => Err(CheckError::Rejected(message)),
    }
}

fn stop(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill.exe")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(crate::constants::CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(unix)]
    {
        let group = format!("-{}", child.id());
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &group])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}
