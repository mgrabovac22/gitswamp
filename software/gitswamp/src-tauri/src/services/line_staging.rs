use std::collections::VecDeque;
use std::path::Path;

use git2::{Delta, DiffOptions, FileMode, Patch, Status};

use crate::models::DiffLine;
use crate::repositories::git_repository::GitRepository;
use crate::services::diff_service::DiffService;

const CONTEXT: usize = 3;
const STALE_LINE: &str = "The diff changed. Refresh it before applying this line.";

pub fn apply(
    path: &str,
    file_path: &str,
    staged: bool,
    hunk_index: usize,
    line_index: usize,
    expected: &DiffLine,
) -> Result<(), String> {
    let repo = GitRepository::open(path).map_err(|e| e.to_string())?;
    if repo
        .status_file(Path::new(file_path))
        .is_ok_and(|status| status.contains(Status::CONFLICTED))
    {
        return Err("Resolve the file conflict before staging individual lines.".into());
    }
    let mut options = DiffOptions::new();
    options
        .context_lines(CONTEXT as u32)
        .interhunk_lines(1)
        .disable_pathspec_match(true)
        .pathspec(file_path);
    let diff = if staged {
        let head = repo.head().and_then(|head| head.peel_to_tree()).ok();
        repo.diff_tree_to_index(head.as_ref(), None, Some(&mut options))
    } else {
        options.include_untracked(true).show_untracked_content(true);
        repo.diff_index_to_workdir(None, Some(&mut options))
    }
    .map_err(|e| e.to_string())?;
    let patch_index = diff
        .deltas()
        .position(|delta| {
            delta.new_file().path().or(delta.old_file().path()) == Some(Path::new(file_path))
        })
        .ok_or(STALE_LINE)?;
    let patch = Patch::from_diff(&diff, patch_index)
        .map_err(|e| e.to_string())?
        .ok_or("Line staging is only available for text files.")?;
    let delta = patch.delta();
    if !matches!(
        delta.status(),
        Delta::Modified | Delta::Added | Delta::Deleted | Delta::Untracked
    ) {
        return Err("Stage this file as a whole to include the file type or path change.".into());
    }
    let (source, other) = if staged {
        (delta.new_file(), delta.old_file())
    } else {
        (delta.old_file(), delta.new_file())
    };
    for file in [&source, &other] {
        if file.exists() && !matches!(file.mode(), FileMode::Blob | FileMode::BlobExecutable) {
            return Err("Line staging is only available for regular text files.".into());
        }
    }
    let (hunk, lines) = patch.hunk(hunk_index).map_err(|_| STALE_LINE)?;
    let selected = patch
        .line_in_hunk(hunk_index, line_index)
        .map_err(|_| STALE_LINE)?;
    let kind = match selected.origin() {
        '+' => "addition",
        '-' => "deletion",
        _ => return Err("Select an added or removed line.".into()),
    };
    if expected.line_type != kind
        || expected.old_line_no != selected.old_lineno()
        || expected.new_line_no != selected.new_lineno()
        || expected.content.as_bytes() != selected.content()
    {
        return Err(STALE_LINE.into());
    }

    let (start, count) = if staged {
        (hunk.new_start(), hunk.new_lines())
    } else {
        (hunk.old_start(), hunk.old_lines())
    };
    let mut offset = if count == 0 {
        start
    } else {
        start.saturating_sub(1)
    };
    let mut before = VecDeque::with_capacity(CONTEXT);
    let mut after = Vec::with_capacity(CONTEXT);
    for i in 0..lines {
        if i == line_index {
            continue;
        }
        let line = patch
            .line_in_hunk(hunk_index, i)
            .map_err(|e| e.to_string())?;
        let on_source = line.origin() == ' ' || orient(line.origin(), staged) == '-';
        if !on_source {
            continue;
        }
        if i < line_index {
            offset += 1;
            if before.len() == CONTEXT {
                before.pop_front();
            }
            before.push_back(line);
        } else {
            after.push(line);
            if after.len() == CONTEXT {
                break;
            }
        }
    }

    let change = orient(selected.origin(), staged);
    let context = before.len() as u32 + after.len() as u32;
    let old_count = context + u32::from(change == '-');
    let new_count = context + u32::from(change == '+');
    let start = offset - before.len() as u32;
    let creates_file = !source.exists();
    let removes_file = !other.exists() && new_count == 0;
    let mode = |mode| {
        if mode == FileMode::BlobExecutable {
            "100755"
        } else {
            "100644"
        }
    };
    let old_path = patch_path('a', file_path);
    let new_path = patch_path('b', file_path);
    let mut bytes = format!("diff --git {old_path} {new_path}\n").into_bytes();
    if creates_file {
        bytes.extend_from_slice(format!("new file mode {}\n", mode(other.mode())).as_bytes());
    } else if removes_file {
        bytes.extend_from_slice(format!("deleted file mode {}\n", mode(source.mode())).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "--- {}\n+++ {}\n@@ -{},{} +{},{} @@\n",
            if creates_file { "/dev/null" } else { &old_path },
            if removes_file { "/dev/null" } else { &new_path },
            start + u32::from(old_count != 0),
            old_count,
            start + u32::from(new_count != 0),
            new_count,
        )
        .as_bytes(),
    );

    // The patch retains only nearby index-side context, not the rest of the changed hunk.
    let mut old_left = old_count;
    let mut new_left = new_count;
    for (prefix, line) in before
        .iter()
        .map(|line| (' ', line))
        .chain(std::iter::once((change, &selected)))
        .chain(after.iter().map(|line| (' ', line)))
    {
        let in_old = prefix != '+';
        let in_new = prefix != '-';
        old_left -= u32::from(in_old);
        new_left -= u32::from(in_new);
        bytes.push(prefix as u8);
        bytes.extend_from_slice(line.content());
        if !line.content().ends_with(b"\n") {
            if (in_old && old_left != 0) || (in_new && new_left != 0) {
                return Err("This line changes the end-of-file newline. Stage or unstage its hunk together.".into());
            }
            bytes.extend_from_slice(b"\n\\ No newline at end of file\n");
        }
    }

    DiffService::apply_patch(
        path,
        &bytes,
        &["apply", "--cached", "--unidiff-zero", "--whitespace=nowarn"],
    )
}

fn orient(prefix: char, staged: bool) -> char {
    match (staged, prefix) {
        (true, '+') => '-',
        (true, '-') => '+',
        _ => prefix,
    }
}

fn patch_path(side: char, path: &str) -> String {
    use std::fmt::Write;
    let mut quoted = format!("\"{side}/");
    for ch in path.chars() {
        match ch {
            '"' | '\\' => {
                quoted.push('\\');
                quoted.push(ch);
            }
            ch if ch.is_ascii_control() => {
                let _ = write!(quoted, "\\{:03o}", ch as u32);
            }
            ch => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}
