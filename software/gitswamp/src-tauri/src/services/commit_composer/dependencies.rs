use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use git2::{ObjectType, Oid, Repository, Tree, TreeWalkMode, TreeWalkResult};

use super::{git_err, Change, PATCH_FILE_LIMIT, PATCH_TOTAL_LIMIT};

const MAX_REFERENCES: usize = 64;
const MAX_CACHED_REFERENCES: usize = 4096;
const MAX_CHECKED_REFERENCES: usize = 16_384;
const MAX_REVERSE_PATHS: usize = 256;
const MAX_WALK_ENTRIES: usize = 4000;
const JS: &[&str] = &[
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".mts",
    ".cts",
    ".mjs",
    ".cjs",
    ".json",
    ".vue",
    ".svelte",
    ".css",
    ".scss",
    ".svg",
    "/index.ts",
    "/index.tsx",
    "/index.js",
    "/index.jsx",
    "/index.mjs",
    "/index.cjs",
];
const PY: &[&str] = &[".py", ".pyi", "/__init__.py", "/__init__.pyi"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Resolver {
    JavaScript,
    Python,
    Include,
    Rust,
    Php,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Import {
    target: String,
    resolver: Resolver,
    symbol: Option<String>,
}

pub(super) struct Problem {
    target: String,
    source: String,
    message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

fn path_from(directory: &str, input: &str) -> Option<String> {
    let mut parts: Vec<_> = directory.split('/').filter(|p| !p.is_empty()).collect();
    for part in input.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name if !name.contains(['\\', ':', '?', '#']) => parts.push(name),
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn quoted(text: &str) -> Option<&str> {
    let text = text.trim_start();
    let quote = text.as_bytes().first().copied()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let (value, _) = text[1..].split_once(quote as char)?;
    (!value.is_empty() && value.len() <= 240).then_some(value)
}

fn identifier(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 80
        && text.as_bytes()[0].is_ascii_alphabetic()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
}

fn family(path: &str) -> Option<Resolver> {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("js" | "jsx" | "ts" | "tsx" | "mts" | "cts" | "mjs" | "cjs" | "vue" | "svelte") => {
            Some(Resolver::JavaScript)
        }
        Some("py" | "pyi") => Some(Resolver::Python),
        Some("c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "m" | "mm") => {
            Some(Resolver::Include)
        }
        Some("rs") => Some(Resolver::Rust),
        Some("php") => Some(Resolver::Php),
        _ => None,
    }
}

fn imports(path: &str, bytes: &[u8], resolver: Resolver) -> Vec<Import> {
    let mut result = BTreeSet::new();
    let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
    let Ok(text) = std::str::from_utf8(bytes) else {
        return vec![];
    };
    let mut block_comment = false;
    let mut js_import = String::new();
    for raw in text.lines() {
        if result.len() >= MAX_REFERENCES {
            break;
        }
        let line = raw.trim();
        if block_comment {
            block_comment = !line.contains("*/");
            continue;
        }
        if line.starts_with("/*") {
            block_comment = !line.contains("*/");
            continue;
        }
        if line.len() > 600 || line.starts_with("//") {
            continue;
        }
        match resolver {
            Resolver::JavaScript => {
                if !js_import.is_empty() {
                    if js_import.len() + line.len() > 600 {
                        js_import.clear();
                        continue;
                    }
                    js_import.push(' ');
                    js_import.push_str(line);
                } else if line.starts_with("import ")
                    || line.starts_with("export {")
                    || line.starts_with("export *")
                {
                    js_import.push_str(line);
                }
                let specifier = if js_import.is_empty() {
                    line
                } else {
                    js_import.as_str()
                };
                let (prefix, value) = if let Some((prefix, tail)) = specifier.rsplit_once(" from ")
                {
                    (prefix, quoted(tail))
                } else if let Some(side_effect) = specifier.strip_prefix("import ") {
                    ("", quoted(side_effect))
                } else {
                    (
                        "",
                        ["require(", "import("].into_iter().find_map(|token| {
                            specifier.split_once(token).and_then(|(prefix, tail)| {
                                let prefix = prefix.trim_end();
                                if prefix.contains(['"', '\'', '`'])
                                    || prefix.ends_with(|c: char| {
                                        c.is_alphanumeric() || matches!(c, '_' | '$')
                                    })
                                {
                                    return None;
                                }
                                quoted(tail)
                            })
                        }),
                    )
                };
                if let Some(value) =
                    value.filter(|value| value.starts_with("./") || value.starts_with("../"))
                {
                    if let Some(target) = path_from(dir, value) {
                        result.insert(Import {
                            target: target.clone(),
                            resolver,
                            symbol: None,
                        });
                        if let Some((_, names)) = prefix
                            .split_once('{')
                            .and_then(|(head, tail)| Some((head, tail.split_once('}')?.0)))
                        {
                            for name in names
                                .split(',')
                                .filter_map(|name| {
                                    let name =
                                        name.trim().strip_prefix("type ").unwrap_or(name.trim());
                                    name.split_ascii_whitespace()
                                        .next()
                                        .filter(|name| identifier(name))
                                })
                                .take(12)
                            {
                                result.insert(Import {
                                    target: target.clone(),
                                    resolver,
                                    symbol: Some(name.into()),
                                });
                            }
                        }
                    }
                }
                if value.is_some() || (!js_import.is_empty() && !js_import.contains('{')) {
                    js_import.clear();
                }
            }
            Resolver::Python => {
                let Some((module, names)) = line
                    .strip_prefix("from ")
                    .and_then(|tail| tail.split_once(" import "))
                else {
                    continue;
                };
                let dots = module.bytes().take_while(|b| *b == b'.').count();
                if dots == 0 {
                    continue;
                }
                let prefix = "../".repeat(dots - 1) + &module[dots..].replace('.', "/");
                if !module[dots..].is_empty() {
                    if let Some(target) = path_from(dir, &prefix) {
                        result.insert(Import {
                            target,
                            resolver,
                            symbol: None,
                        });
                    }
                } else {
                    for name in names
                        .split(',')
                        .filter_map(|name| {
                            name.split_ascii_whitespace()
                                .next()
                                .filter(|name| identifier(name))
                        })
                        .take(12)
                    {
                        if let Some(target) = path_from(dir, &format!("{prefix}{name}")) {
                            result.insert(Import {
                                target,
                                resolver,
                                symbol: None,
                            });
                        }
                    }
                }
            }
            Resolver::Include => {
                let tail = line
                    .strip_prefix('#')
                    .map(str::trim_start)
                    .and_then(|tail| {
                        tail.strip_prefix("include")
                            .or_else(|| tail.strip_prefix("import"))
                    });
                if let Some(target) = tail
                    .and_then(quoted)
                    .and_then(|value| path_from(dir, value))
                {
                    result.insert(Import {
                        target,
                        resolver,
                        symbol: None,
                    });
                }
            }
            Resolver::Rust | Resolver::Php => {
                if let Some(target) =
                    literal_include(line, resolver).and_then(|value| path_from(dir, value))
                {
                    result.insert(Import {
                        target,
                        resolver,
                        symbol: None,
                    });
                }
            }
        }
    }
    result.into_iter().take(MAX_REFERENCES).collect()
}

fn literal_include(line: &str, resolver: Resolver) -> Option<&str> {
    if resolver == Resolver::Rust {
        for name in ["include!", "include_str!", "include_bytes!"] {
            let Some((before, after)) = line.split_once(name) else {
                continue;
            };
            // Require a standalone macro name and a literal, not concat! or OUT_DIR.
            if before.contains(['"', '\''])
                || before.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
            {
                continue;
            }
            return after.trim_start().strip_prefix('(').and_then(quoted);
        }
    }
    if resolver == Resolver::Php {
        let (keyword, rest) = line.split_once(|c: char| c == '(' || c.is_ascii_whitespace())?;
        if matches!(
            keyword,
            "require" | "require_once" | "include" | "include_once"
        ) {
            let rest = rest.trim_start().trim_start_matches('(').trim_start();
            return quoted(
                rest.strip_prefix("__DIR__")?
                    .trim_start()
                    .strip_prefix('.')?,
            )
            .and_then(|file| file.strip_prefix('/'));
        }
    }
    None
}

fn at(tree: Option<&Tree<'_>>, path: &str) -> Option<Oid> {
    tree?
        .get_path(Path::new(path))
        .ok()
        .filter(|e| e.kind() == Some(ObjectType::Blob))
        .map(|e| e.id())
}

fn resolve(tree: Option<&Tree<'_>>, import: &Import) -> Option<Oid> {
    let stem = &import.target;
    if let Some(oid) = at(tree, stem) {
        return Some(oid);
    }
    let suffixes = match import.resolver {
        Resolver::JavaScript => JS,
        Resolver::Python => PY,
        Resolver::Include | Resolver::Rust | Resolver::Php => &[],
    };
    if import.resolver == Resolver::JavaScript {
        if let Some(stem) = stem.strip_suffix(".js") {
            for suffix in [".ts", ".tsx", ".jsx"] {
                if let Some(oid) = at(tree, &format!("{stem}{suffix}")) {
                    return Some(oid);
                }
            }
        }
    }
    suffixes
        .iter()
        .find_map(|suffix| at(tree, &format!("{stem}{suffix}")))
}

fn exports(bytes: &[u8]) -> Option<BTreeSet<String>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut names = BTreeSet::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with("export *")
            || line.contains("module.exports")
            || line.starts_with("exports.")
        {
            return None;
        }
        let Some(line) = line.strip_prefix("export ") else {
            continue;
        };
        let line = line.strip_prefix("declare ").unwrap_or(line);
        let line = line.strip_prefix("async ").unwrap_or(line);
        let line = line
            .strip_prefix("type {")
            .map_or(line, |_| &line["type ".len()..]);
        if line.starts_with("default ") || line.starts_with("as namespace ") {
            continue;
        }
        if let Some(list) = line.strip_prefix('{').and_then(|tail| tail.split_once('}')) {
            for name in list
                .0
                .split(',')
                .map(|name| name.split(" as ").last().unwrap_or("").trim())
                .filter(|name| identifier(name))
            {
                names.insert(name.into());
            }
        } else if line.starts_with('{') {
            return None;
        } else if let Some((keyword, target)) = line.split_once(' ') {
            if matches!(
                keyword,
                "const"
                    | "let"
                    | "var"
                    | "function"
                    | "class"
                    | "interface"
                    | "type"
                    | "enum"
                    | "namespace"
            ) {
                if let Some(name) = target
                    .trim()
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
                    .next()
                    .filter(|name| identifier(name))
                {
                    names.insert(name.into());
                }
            }
        }
        if names.len() > MAX_REFERENCES {
            return None;
        }
    }
    (!names.is_empty()).then_some(names)
}

pub(super) struct Check<'a> {
    repo: &'a Repository,
    final_tree: Tree<'a>,
    base_tree: Option<Tree<'a>>,
    paths: Vec<String>,
    base: BTreeMap<String, BTreeSet<Import>>,
    cache: HashMap<(String, Oid), Vec<Import>>,
    exports: HashMap<Oid, Option<BTreeSet<String>>>,
    remaining: usize,
    cache_remaining: usize,
    checks_remaining: usize,
    pub(super) limited: bool,
}

impl<'a> Check<'a> {
    pub(super) fn new(
        repo: &'a Repository,
        head: Option<Oid>,
        final_id: Oid,
        changes: &[Change],
    ) -> Result<Self, String> {
        let base_tree = head
            .map(|id| repo.find_commit(id).and_then(|commit| commit.tree()))
            .transpose()
            .map_err(git_err)?;
        let final_tree = repo.find_tree(final_id).map_err(git_err)?;
        let mut paths: BTreeSet<_> = changes
            .iter()
            .flat_map(|change| [Some(change.path.as_str()), change.old_path.as_deref()])
            .flatten()
            .filter(|path| family(path).is_some())
            .map(str::to_owned)
            .collect();
        // A removal can break a caller that has no diff of its own.
        let reverse_search = changes.iter().any(|file| {
            file.old.is_some()
                && (file.new.is_none()
                    || file.old_path.is_some()
                    || (file.hints.removes_symbol()
                        && family(&file.path) == Some(Resolver::JavaScript)))
        });
        let mut limited = false;
        if reverse_search {
            if let Some(base) = &base_tree {
                let mut entries = 0;
                let mut extra = 0;
                base.walk(TreeWalkMode::PreOrder, |root, entry| {
                    entries += 1;
                    if entries > MAX_WALK_ENTRIES || extra >= MAX_REVERSE_PATHS {
                        limited = true;
                        return TreeWalkResult::Abort;
                    }
                    if let Some(name) = entry.name().filter(|name| {
                        entry.kind() == Some(ObjectType::Blob) && family(name).is_some()
                    }) {
                        if paths.insert(format!("{root}{name}")) {
                            extra += 1;
                        }
                    }
                    TreeWalkResult::Ok
                })
                .map_err(git_err)?;
            }
        }
        let mut check = Self {
            repo,
            final_tree,
            base_tree,
            paths: paths.into_iter().collect(),
            base: BTreeMap::new(),
            cache: HashMap::new(),
            exports: HashMap::new(),
            remaining: PATCH_TOTAL_LIMIT,
            cache_remaining: MAX_CACHED_REFERENCES,
            checks_remaining: MAX_CHECKED_REFERENCES,
            limited,
        };
        for path in check.paths.clone() {
            let Some(id) = at(check.base_tree.as_ref(), &path) else {
                continue;
            };
            let missing = check
                .read_imports(&path, id)
                .into_iter()
                .filter(|import| {
                    let target = resolve(check.base_tree.as_ref(), import);
                    target.is_none()
                        || import.symbol.as_ref().is_some_and(|name| {
                            check.exported(target.unwrap(), name) == Some(false)
                        })
                })
                .collect();
            check.base.insert(path, missing);
        }
        Ok(check)
    }

    fn read(&mut self, oid: Oid) -> Option<git2::Blob<'a>> {
        let (size, _) = self.repo.odb().ok()?.read_header(oid).ok()?;
        if size > PATCH_FILE_LIMIT || size > self.remaining {
            self.limited = true;
            return None;
        }
        self.remaining -= size;
        self.repo
            .find_blob(oid)
            .ok()
            .filter(|blob| !blob.is_binary())
    }

    fn read_imports(&mut self, path: &str, oid: Oid) -> Vec<Import> {
        let key = (path.to_owned(), oid);
        if let Some(value) = self.cache.get(&key) {
            return value.clone();
        }
        let value = self
            .read(oid)
            .and_then(|blob| family(path).map(|f| imports(path, blob.content(), f)))
            .unwrap_or_default();
        if self.cache.len() < 768 && value.len() <= self.cache_remaining {
            self.cache_remaining -= value.len();
            self.cache.insert(key, value.clone());
        }
        value
    }

    fn exported(&mut self, oid: Oid, symbol: &str) -> Option<bool> {
        if !self.exports.contains_key(&oid) {
            let exports = self.read(oid).and_then(|blob| exports(blob.content()));
            if self.exports.len() < 256 {
                self.exports.insert(oid, exports);
            } else {
                self.limited = true;
                return exports.map(|names| names.contains(symbol));
            }
        }
        self.exports
            .get(&oid)?
            .as_ref()
            .map(|names| names.contains(symbol))
    }

    pub(super) fn verify(&mut self, tree: &Tree<'_>) -> Option<Problem> {
        for path in self.paths.clone() {
            let Some(id) = at(Some(tree), &path) else {
                continue;
            };
            for import in self.read_imports(&path, id) {
                if self.checks_remaining == 0 {
                    self.limited = true;
                    return None;
                }
                self.checks_remaining -= 1;
                let target = resolve(Some(tree), &import);
                let final_target = resolve(Some(&self.final_tree), &import);
                let already_missing = self
                    .base
                    .get(&path)
                    .is_some_and(|refs| refs.contains(&import));
                let absent = match (target, import.symbol.as_deref()) {
                    (None, None) => {
                        (!already_missing && import.resolver == Resolver::JavaScript)
                            || final_target.is_some()
                            || resolve(self.base_tree.as_ref(), &import).is_some()
                    }
                    (Some(target), Some(symbol)) => {
                        self.exported(target, symbol) == Some(false)
                            && (!already_missing
                                || final_target
                                    .is_some_and(|id| self.exported(id, symbol) == Some(true)))
                    }
                    _ => false,
                };
                if absent {
                    let message = match import.symbol.as_ref() {
                        Some(name) => {
                            format!("{path} imports {name} before {} exports it.", import.target)
                        }
                        None => format!(
                            "{path} references {} before it is available.",
                            import.target
                        ),
                    };
                    return Some(Problem {
                        target: import.target,
                        source: path,
                        message,
                    });
                }
            }
        }
        None
    }
}

impl Problem {
    pub(super) fn is_provider(&self, change: &Change) -> bool {
        change.path == self.source
            || change.old_path.as_deref() == Some(self.source.as_str())
            || change.path == self.target
            || change
                .path
                .strip_prefix(&self.target)
                .is_some_and(|tail| JS.contains(&tail) || PY.contains(&tail))
            || self.target.strip_suffix(".js").is_some_and(|stem| {
                change
                    .path
                    .strip_prefix(stem)
                    .is_some_and(|tail| matches!(tail, ".ts" | ".tsx" | ".jsx"))
            })
    }
}
