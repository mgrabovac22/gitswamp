use std::path::Path;

use git2::{DiffOptions, Oid, Patch, Repository};

use crate::models::{commit_composition::ChangePreview, DiffLine};

use super::{chunks, git_err, Change};

const MAX_BLOB_BYTES: usize = 128 * 1024;
const MAX_LINES: usize = 160;
const MAX_LINE_BYTES: usize = 720;

pub(super) fn file(repo: &Repository, change: &Change) -> Result<ChangePreview, String> {
    let mut preview = ChangePreview::default();
    if [change.old, change.new]
        .into_iter()
        .flatten()
        .any(|(_, mode)| mode == 0o160000)
    {
        return Ok(preview);
    }
    let odb = repo.odb().map_err(git_err)?;
    for (id, _) in [change.old, change.new].into_iter().flatten() {
        if odb.read_header(id).map_err(git_err)?.0 > MAX_BLOB_BYTES {
            preview.limited = true;
            return Ok(preview);
        }
    }
    if change.old.map(|(id, _)| id) == change.new.map(|(id, _)| id) {
        return Ok(preview);
    }
    let load = |id: Option<Oid>| id.map(|id| repo.find_blob(id)).transpose().map_err(git_err);
    let old = load(change.old.map(|(id, _)| id))?;
    let new = load(if change.chunk.is_some() {
        Some(chunks::blob(repo, &[change])?)
    } else {
        change.new.map(|(id, _)| id)
    })?;
    if old.as_ref().is_some_and(|blob| blob.is_binary())
        || new.as_ref().is_some_and(|blob| blob.is_binary())
    {
        preview.binary = true;
        return Ok(preview);
    }
    let old_path = change.old_path.as_deref().unwrap_or(&change.path);
    let mut options = DiffOptions::new();
    options.context_lines(2);
    let patch = Patch::from_buffers(
        old.as_ref().map_or(&[], |blob| blob.content()),
        Some(Path::new(old_path)),
        new.as_ref().map_or(&[], |blob| blob.content()),
        Some(Path::new(&change.path)),
        Some(&mut options),
    )
    .map_err(git_err)?;
    for index in 0..patch.num_hunks() {
        let (hunk, count) = patch.hunk(index).map_err(git_err)?;
        if preview.lines.len() + 1 >= MAX_LINES {
            preview.limited = true;
            break;
        }
        preview.lines.push(DiffLine {
            line_type: "hunk".into(),
            old_line_no: None,
            new_line_no: None,
            content: excerpt(hunk.header(), &mut preview.limited),
        });
        for row in 0..count {
            if preview.lines.len() >= MAX_LINES {
                preview.limited = true;
                return Ok(preview);
            }
            let line = patch.line_in_hunk(index, row).map_err(git_err)?;
            let line_type = match line.origin() {
                '+' => "addition",
                '-' => "deletion",
                ' ' => "context",
                _ => continue,
            };
            preview.lines.push(DiffLine {
                line_type: line_type.into(),
                old_line_no: line.old_lineno(),
                new_line_no: line.new_lineno(),
                content: excerpt(line.content(), &mut preview.limited),
            });
        }
    }
    Ok(preview)
}

fn excerpt(bytes: &[u8], limited: &mut bool) -> String {
    let end = bytes.len().min(MAX_LINE_BYTES);
    *limited |= end < bytes.len();
    String::from_utf8_lossy(&bytes[..end])
        .trim_end_matches(['\r', '\n'])
        .to_owned()
}
