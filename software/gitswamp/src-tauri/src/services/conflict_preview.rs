use std::io::Read;
use std::path::{Component, Path};

use git2::IndexEntry;
use serde::Serialize;

use crate::repositories::git_repository::GitRepository;

const MAX_BYTES: usize = 256 * 1024;
const MAX_RANGES: usize = 8;
const MAX_CODE_LINES: usize = 7;
const MAX_LINE_BYTES: usize = 192;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PreviewLine {
    pub number: usize,
    pub text: String,
    pub conflicted: bool,
}

#[derive(Serialize)]
pub struct ConflictPreviewFile {
    pub path: String,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub ours_code: Vec<PreviewLine>,
    pub theirs_code: Vec<PreviewLine>,
    pub ranges: Vec<[usize; 2]>,
    pub partial: bool,
}

fn marker_width(line: &[u8], marker: u8) -> Option<usize> {
    let width = line.iter().take_while(|&&byte| byte == marker).count();
    (width >= 7
        && line
            .get(width)
            .is_none_or(|byte| byte.is_ascii_whitespace()))
    .then_some(width)
}

fn conflict_ranges(bytes: &[u8]) -> (Vec<[usize; 2]>, bool) {
    let mut ranges = Vec::new();
    let mut block = None;
    for (i, line) in bytes.split(|&byte| byte == b'\n').enumerate() {
        let line_number = i + 1;
        if let Some(width) = marker_width(line, b'<') {
            block = Some((line_number, width, false));
        } else if let Some((start, width, separator)) = block.as_mut() {
            if marker_width(line, b'=') == Some(*width) {
                *separator = true;
            } else if *separator && marker_width(line, b'>') == Some(*width) {
                if ranges.len() == MAX_RANGES {
                    return (ranges, true);
                }
                ranges.push([*start, line_number]);
                block = None;
            }
        }
    }
    (ranges, block.is_some())
}

fn read_text(root: &Path, relative: &Path) -> Option<(Vec<u8>, bool)> {
    let path = root.join(relative).canonicalize().ok()?;
    if !path.starts_with(root) || !path.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.contains(&0) {
        return None;
    }
    let truncated = bytes.len() > MAX_BYTES;
    if truncated {
        bytes.truncate(bytes[..MAX_BYTES].iter().rposition(|&byte| byte == b'\n')? + 1);
    }
    Some((bytes, truncated))
}

fn index_path(entry: Option<&IndexEntry>) -> Option<String> {
    std::str::from_utf8(&entry?.path).ok().map(str::to_owned)
}

fn preview_line(number: usize, bytes: &[u8], conflicted: bool) -> PreviewLine {
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    let mut text: String = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_LINE_BYTES)])
        .chars()
        .filter(|c| !c.is_control() || *c == '\t')
        .collect();
    if bytes.len() > MAX_LINE_BYTES {
        text.push_str("...");
    }
    PreviewLine {
        number,
        text,
        conflicted,
    }
}

fn excerpt(bytes: &[u8], [start, end]: [usize; 2]) -> (Vec<PreviewLine>, Vec<PreviewLine>) {
    let mut ours = Vec::new();
    let mut theirs = Vec::new();
    let mut width = 0;
    let mut incoming = false;
    let mut ancestor = false;
    // Include adjacent context, but never display marker lines or the diff3 ancestor.
    for (i, bytes) in bytes
        .strip_suffix(b"\n")
        .unwrap_or(bytes)
        .split(|&byte| byte == b'\n')
        .enumerate()
        .skip(start.saturating_sub(2))
    {
        let number = i + 1;
        if number > end + 1 {
            break;
        }
        if number < start || number == end + 1 {
            if [b'<', b'=', b'>', b'|']
                .iter()
                .any(|&marker| marker_width(bytes, marker).is_some())
            {
                continue;
            }
            let line = preview_line(number, bytes, false);
            if ours.len() < MAX_CODE_LINES {
                ours.push(line.clone());
            }
            if theirs.len() < MAX_CODE_LINES {
                theirs.push(line);
            }
        } else if number == start {
            width = marker_width(bytes, b'<').unwrap_or(0);
        } else if number < end {
            if marker_width(bytes, b'=') == Some(width) {
                incoming = true;
                ancestor = false;
            } else if marker_width(bytes, b'|') == Some(width) {
                ancestor = true;
            } else if !ancestor {
                let side = if incoming { &mut theirs } else { &mut ours };
                if side.len() < MAX_CODE_LINES {
                    side.push(preview_line(number, bytes, true));
                }
            }
        }
    }
    (ours, theirs)
}

fn stage_excerpt(repo: &git2::Repository, entry: Option<&IndexEntry>) -> Option<Vec<PreviewLine>> {
    let entry = entry?;
    let odb = repo.odb().ok()?;
    let (len, kind) = odb.read_header(entry.id).ok()?;
    if kind != git2::ObjectType::Blob || len == 0 || len > MAX_BYTES {
        return None;
    }
    let object = odb.read(entry.id).ok()?;
    let bytes = object.data();
    if bytes.contains(&0) {
        return None;
    }
    Some(
        bytes
            .strip_suffix(b"\n")
            .unwrap_or(bytes)
            .split(|&byte| byte == b'\n')
            .take(MAX_CODE_LINES)
            .enumerate()
            .map(|(i, line)| preview_line(i + 1, line, false))
            .collect(),
    )
}

pub fn preview(path: &str, file_path: &str) -> Result<Option<ConflictPreviewFile>, String> {
    let repo = GitRepository::open(path)?;
    let relative = Path::new(file_path);
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Ok(None);
    }
    let root = repo
        .workdir()
        .ok_or("No working tree.")?
        .canonicalize()
        .map_err(|_| "Working tree unavailable.")?;
    let index = repo.index().map_err(|e| e.message().to_string())?;
    let ours = index.get_path(relative, 2);
    let theirs = index.get_path(relative, 3);
    if ours.is_none() && theirs.is_none() && index.get_path(relative, 1).is_none() {
        return Ok(None);
    }
    let (bytes, truncated) = read_text(&root, relative).unwrap_or_default();
    let (ranges, partial) = conflict_ranges(&bytes);
    let (ours_code, theirs_code) = match ranges.first() {
        Some(range) => excerpt(&bytes, *range),
        None => (
            stage_excerpt(&repo, ours.as_ref()).unwrap_or_default(),
            stage_excerpt(&repo, theirs.as_ref()).unwrap_or_default(),
        ),
    };
    Ok(Some(ConflictPreviewFile {
        path: file_path.to_owned(),
        ours: index_path(ours.as_ref()),
        theirs: index_path(theirs.as_ref()),
        ours_code,
        theirs_code,
        ranges,
        partial: partial || truncated,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_complete_marker_ranges_and_excludes_diff3_ancestor_from_code() {
        let bytes = b"context\r\n<<<<<<< HEAD\r\nours\r\n||||||| base\r\nbase\r\n=======\r\ntheirs\r\n>>>>>>> topic\r\n<<<<<<<< next\nnext\n========\nnext\n>>>>>>>> other";
        assert_eq!(conflict_ranges(bytes), (vec![[2, 8], [9, 13]], false));
        let (ours, theirs) = excerpt(bytes, [2, 8]);
        assert_eq!(
            ours,
            vec![
                preview_line(1, b"context", false),
                preview_line(3, b"ours", true)
            ]
        );
        assert_eq!(
            theirs,
            vec![
                preview_line(1, b"context", false),
                preview_line(7, b"theirs", true)
            ]
        );
    }

    #[test]
    fn does_not_invent_conflict_lines_for_unfinished_blocks_or_plain_text() {
        assert_eq!(
            conflict_ranges(b"=======\nplain\n>>>>>>> end"),
            (vec![], false)
        );
        assert_eq!(
            conflict_ranges(b"<<<<<<< HEAD\n=======\npartial"),
            (vec![], true)
        );
        assert_eq!(conflict_ranges(b"<<<<<<<not-a-marker"), (vec![], false));
    }

    #[test]
    fn code_excerpt_caps_lines_and_long_rows_per_side() {
        let line = "x".repeat(MAX_LINE_BYTES * 2);
        let side = (line + "\n").repeat(MAX_CODE_LINES * 2);
        let text = format!("<<<<<<< HEAD\n{side}=======\n{side}>>>>>>> topic\n");
        let (ranges, _) = conflict_ranges(text.as_bytes());
        let (ours, theirs) = excerpt(text.as_bytes(), ranges[0]);
        for lines in [ours, theirs] {
            assert_eq!(lines.len(), MAX_CODE_LINES);
            assert!(lines
                .iter()
                .all(|line| line.conflicted && line.text.len() == MAX_LINE_BYTES + 3));
        }
    }
}
