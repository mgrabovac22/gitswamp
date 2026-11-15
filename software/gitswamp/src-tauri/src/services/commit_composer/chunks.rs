use std::ops::Range;

use git2::{Error, Oid, Patch, Repository};
use serde::Serialize;

use super::{git_err, rules::behavior::Cursor, Change};

pub(super) const MAX_SPLIT_BYTES: usize = 512 * 1024;
pub(super) const MAX_EXTRA_CHANGES: usize = 120;
const MAX_FILE_HUNKS: usize = 16;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Chunk {
    pub(super) old_start: u32,
    pub(super) old_lines: u32,
    pub(super) new_start: u32,
    pub(super) new_lines: u32,
    pub(super) scope: String,
    #[serde(skip)]
    pub(super) linked_scopes: Vec<String>,
    #[serde(skip)]
    pub(super) old: Range<usize>,
    #[serde(skip)]
    pub(super) new: Range<usize>,
}

fn offsets(bytes: &[u8]) -> Vec<usize> {
    let mut result = vec![0];
    for (i, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            result.push(i + 1);
        }
    }
    if result.last() != Some(&bytes.len()) {
        result.push(bytes.len());
    }
    result
}

fn byte_range(offsets: &[usize], start: u32, count: u32) -> Option<Range<usize>> {
    let start = if count == 0 {
        start
    } else {
        start.checked_sub(1)?
    } as usize;
    Some(*offsets.get(start)?..*offsets.get(start + count as usize)?)
}

fn same_scope(scope: &mut Option<String>, next: &str) -> bool {
    if next.is_empty() {
        return false;
    }
    if let Some(scope) = scope {
        scope == next
    } else {
        *scope = Some(next.into());
        true
    }
}

pub(super) fn identify(
    path: &str,
    patch: &Patch<'_>,
    before: &[u8],
    after: &[u8],
) -> Result<Option<Vec<Chunk>>, Error> {
    let count = patch.num_hunks();
    if !(2..=MAX_FILE_HUNKS).contains(&count) || before.contains(&0) || after.contains(&0) {
        return Ok(None);
    }
    let (old_offsets, new_offsets) = (offsets(before), offsets(after));
    let (mut previous, mut next) = (Cursor::new(path, before), Cursor::new(path, after));
    let mut chunks = Vec::with_capacity(count);
    for hunk in 0..count {
        let (range, lines) = patch.hunk(hunk)?;
        if lines > 200 {
            return Ok(None);
        }
        let mut scope = None;
        for line in 0..lines {
            let line = patch.line_in_hunk(hunk, line)?;
            let (cursor, number) = match line.origin() {
                '+' => (&mut next, line.new_lineno()),
                '-' => (&mut previous, line.old_lineno()),
                _ => continue,
            };
            let Some(number) = number else {
                return Ok(None);
            };
            if !same_scope(&mut scope, cursor.scope_at(number)) {
                return Ok(None);
            }
        }
        let (Some(scope), Some(old), Some(new)) = (
            scope,
            byte_range(&old_offsets, range.old_start(), range.old_lines()),
            byte_range(&new_offsets, range.new_start(), range.new_lines()),
        ) else {
            return Ok(None);
        };
        if chunks
            .last()
            .is_some_and(|last: &Chunk| last.old.end > old.start || last.new.end > new.start)
        {
            return Ok(None);
        }
        chunks.push(Chunk {
            old_start: range.old_start(),
            old_lines: range.old_lines(),
            new_start: range.new_start(),
            new_lines: range.new_lines(),
            scope,
            linked_scopes: vec![],
            old,
            new,
        });
    }
    let mut scopes = chunks.iter().map(|chunk| chunk.scope.as_str());
    if let Some(first) = scopes.next() {
        if scopes.all(|scope| scope == first) {
            return Ok(None);
        }
    }
    if materialize(before, after, chunks.iter())? != after {
        return Ok(None);
    }
    let scopes: Vec<_> = chunks.iter().map(|chunk| chunk.scope.clone()).collect();
    for chunk in &mut chunks {
        for scope in &scopes {
            if scope == &chunk.scope || chunk.linked_scopes.contains(scope) {
                continue;
            }
            let mentions = [&before[chunk.old.clone()], &after[chunk.new.clone()]]
                .into_iter()
                .any(|bytes| {
                    bytes
                        .split(|b| !(b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$')))
                        .any(|token| token == scope.as_bytes())
                });
            if mentions {
                chunk.linked_scopes.push(scope.clone());
            }
        }
    }
    Ok(Some(chunks))
}

fn materialize<'a>(
    before: &[u8],
    after: &[u8],
    chunks: impl IntoIterator<Item = &'a Chunk>,
) -> Result<Vec<u8>, Error> {
    let mut result = Vec::with_capacity(before.len().max(after.len()));
    let mut offset = 0;
    for chunk in chunks {
        result.extend_from_slice(
            before
                .get(offset..chunk.old.start)
                .ok_or_else(|| Error::from_str("Overlapping commit hunks."))?,
        );
        result.extend_from_slice(
            after
                .get(chunk.new.clone())
                .ok_or_else(|| Error::from_str("Invalid commit hunk range."))?,
        );
        offset = chunk.old.end;
    }
    result.extend_from_slice(
        before
            .get(offset..)
            .ok_or_else(|| Error::from_str("Invalid commit hunk."))?,
    );
    Ok(result)
}

pub(super) fn blob(repo: &Repository, active: &[&Change]) -> Result<Oid, String> {
    let first = active.first().ok_or("No hunks selected.")?;
    let old = first.old.ok_or("Hunk has no original blob.")?.0;
    let new = first.new.ok_or("Hunk has no resulting blob.")?.0;
    let before = repo.find_blob(old).map_err(git_err)?;
    let after = repo.find_blob(new).map_err(git_err)?;
    let mut hunks: Vec<_> = active
        .iter()
        .filter_map(|file| file.chunk.as_ref())
        .collect();
    hunks.sort_unstable_by_key(|chunk| (chunk.old.start, chunk.new.start));
    let bytes = materialize(before.content(), after.content(), hunks).map_err(git_err)?;
    repo.blob(&bytes).map_err(git_err)
}

pub(super) fn related(left: &Change, right: &Change) -> bool {
    let (Some(a), Some(b)) = (&left.chunk, &right.chunk) else {
        return true;
    };
    a.scope == b.scope || a.linked_scopes.contains(&b.scope) || b.linked_scopes.contains(&a.scope)
}
