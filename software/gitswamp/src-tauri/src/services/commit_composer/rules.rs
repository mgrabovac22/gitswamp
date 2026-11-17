pub(super) mod behavior;
mod cohesion;
mod evidence;
mod messages;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::Serialize;

use super::{Change, Group, Unit};
use behavior::{Behavior, Signal};
use evidence::{Action, Evidence, Topic};
use messages::{draft, explain_group};

const SUGGESTED_GROUPS: usize = super::MAX_GROUPS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Kind {
    Source,
    Test,
    Docs,
    Style,
    Build,
    Lock,
    Ci,
    Migration,
    Schema,
    Config,
    Asset,
    Generated,
    Git,
    Other,
}

impl Kind {
    pub(super) fn inspect_text(self) -> bool {
        !matches!(self, Self::Asset | Self::Lock | Self::Generated)
    }
}

const SOURCE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs", "vue", "svelte", "astro", "rs", "py",
    "pyi", "kt", "kts", "java", "cs", "fs", "fsx", "go", "rb", "php", "swift", "m", "mm", "c", "h",
    "cc", "cpp", "cxx", "hpp", "hh", "dart", "ex", "exs", "scala", "clj", "cljs", "zig", "lua",
    "r", "jl", "hs", "ml", "mli", "erl", "sh", "bash", "zsh", "ps1", "bat", "cmd", "html", "htm",
    "razor", "blade", "sql", "groovy", "sol", "nim", "cr", "gleam", "v", "vh", "vhd", "vhdl", "sv",
    "pas", "pp", "f90", "f95", "vb", "elm", "re", "res", "purs", "wgsl", "glsl", "hlsl", "cu",
    "cuh", "tcl", "awk", "pl", "pm", "cljc", "lisp", "el", "rkt", "scm", "ocaml", "ino", "ada",
    "adb", "ads", "d", "hx", "hbs", "ejs", "erb", "jinja", "j2", "pug", "twig", "liquid", "xaml",
    "ipynb", "cob", "cbl", "mo", "raku", "for", "f03", "c3", "odin", "asm", "s", "wat", "qml",
    "svh", "psm1", "psd1", "bas", "cls", "exp", "abap",
];

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
fn extension(name: &str) -> &str {
    name.rsplit_once('.').map_or("", |(_, ext)| ext)
}

pub(super) fn kind(path: &str, mode: u32) -> Kind {
    let path = path.to_ascii_lowercase();
    let name = basename(&path);
    let ext = extension(name);
    let has_dir = |names: &[&str]| {
        path.rsplit_once('/')
            .is_some_and(|(dir, _)| dir.split('/').any(|part| names.contains(&part)))
    };
    if mode == 0o160000
        || matches!(
            name,
            ".gitattributes" | ".gitignore" | ".gitmodules" | ".mailmap"
        )
    {
        return Kind::Git;
    }
    if path.starts_with(".github/workflows/")
        || path.starts_with(".github/actions/")
        || path.starts_with(".circleci/")
        || path.starts_with(".buildkite/")
        || path.starts_with(".woodpecker/")
        || path.starts_with(".azure/pipelines/")
        || name.ends_with(".gitlab-ci.yml")
        || name.starts_with("jenkinsfile")
        || matches!(
            name,
            ".gitlab-ci.yml" | "azure-pipelines.yml" | "bitbucket-pipelines.yml" | ".travis.yml"
        )
    {
        return Kind::Ci;
    }
    if matches!(
        name,
        "package-lock.json"
            | "npm-shrinkwrap.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "bun.lock"
            | "bun.lockb"
            | "cargo.lock"
            | "poetry.lock"
            | "uv.lock"
            | "pipfile.lock"
            | "go.sum"
            | "go.work.sum"
            | "gemfile.lock"
            | "composer.lock"
            | "pubspec.lock"
            | "packages.lock.json"
            | "package.resolved"
            | "podfile.lock"
            | "gradle.lockfile"
            | "mix.lock"
            | "flake.lock"
            | "pixi.lock"
            | "pdm.lock"
            | "deno.lock"
            | "vcpkg-lock.json"
            | "conan.lock"
            | ".terraform.lock.hcl"
            | "cartfile.resolved"
            | "renv.lock"
            | "gleam.lock"
            | "shard.lock"
            | "pylock.toml"
    ) {
        return Kind::Lock;
    }
    if matches!(
        name,
        "package.json"
            | "cargo.toml"
            | "pyproject.toml"
            | "pipfile"
            | "setup.py"
            | "setup.cfg"
            | "go.mod"
            | "go.work"
            | "gemfile"
            | "composer.json"
            | "pubspec.yaml"
            | "package.swift"
            | "podfile"
            | "pom.xml"
            | "makefile"
            | "cmakelists.txt"
            | "deno.json"
            | "deno.jsonc"
            | "mix.exs"
            | "build.gradle"
            | "build.gradle.kts"
            | "settings.gradle"
            | "settings.gradle.kts"
            | "libs.versions.toml"
            | "gradle.properties"
            | "directory.build.props"
            | "directory.packages.props"
            | "build.sbt"
            | "build.zig"
            | "flake.nix"
            | "docker-compose.yml"
            | "compose.yaml"
            | "justfile"
            | "taskfile.yml"
            | "taskfile.yaml"
            | "pnpm-workspace.yaml"
            | "turbo.json"
            | "nx.json"
            | "meson.build"
            | "meson_options.txt"
            | "build.bazel"
            | "workspace.bazel"
            | "module.bazel"
            | "vcpkg.json"
            | "conanfile.txt"
            | "conanfile.py"
            | "pdm.toml"
            | "pixi.toml"
            | "environment.yml"
            | "manifest.in"
            | "cartfile"
            | "gnumakefile"
            | "gleam.toml"
            | "shard.yml"
            | "project.toml"
            | "buf.yaml"
            | "buf.gen.yaml"
            | "devcontainer.json"
    ) || name.starts_with("dockerfile")
        || name.starts_with("requirements") && ext == "txt"
        || name.starts_with("tsconfig")
        || name.starts_with("vite.config.")
        || name.starts_with("webpack.config.")
        || matches!(
            ext,
            "csproj" | "fsproj" | "vbproj" | "sln" | "slnx" | "cmake" | "gradle" | "xcconfig"
        )
    {
        return Kind::Build;
    }
    if has_dir(&["migration", "migrations", "alembic", "flyway", "liquibase"])
        || name.starts_with("migration_")
    {
        return Kind::Migration;
    }
    if matches!(
        ext,
        "proto" | "graphql" | "gql" | "prisma" | "avsc" | "thrift" | "fbs"
    ) || matches!(
        name,
        "schema.sql"
            | "openapi.yml"
            | "openapi.yaml"
            | "openapi.json"
            | "swagger.json"
            | "swagger.yaml"
            | "openapi.ts"
            | "asyncapi.yaml"
            | "asyncapi.json"
            | "schema.rb"
            | "structure.sql"
    ) {
        return Kind::Schema;
    }
    if name.ends_with(".min.js")
        || name.ends_with(".min.css")
        || name.ends_with(".pb.go")
        || name.ends_with("_pb2.py")
        || name.ends_with("_pb2_grpc.py")
        || name.ends_with(".designer.cs")
        || name.ends_with("modelsnapshot.cs")
        || name.ends_with("_pb.ts")
        || name.ends_with("_pb.js")
        || name.ends_with(".gen.ts")
        || name.ends_with(".g.dart")
        || name.ends_with(".freezed.dart")
        || name.contains(".generated.")
        || has_dir(&["generated", "__generated__"])
    {
        return Kind::Generated;
    }
    if has_dir(&[
        "tests",
        "test",
        "__tests__",
        "spec",
        "specs",
        "__mocks__",
        "__snapshots__",
        "fixtures",
        "testdata",
        "androidtest",
        "e2e",
        "cypress",
        "playwright",
        "snapshots",
        "golden",
        "bench",
        "benches",
    ]) || name.starts_with("test_")
        || name.ends_with("_test.go")
        || name.contains(".spec.")
        || name.contains(".test.")
        || name.contains(".stories.")
        || name.ends_with(".snap")
        || name.ends_with("_spec.rb")
        || name.ends_with(".feature")
        || Path::new(name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.ends_with("tests") || stem.ends_with("test"))
    {
        return Kind::Test;
    }
    if matches!(
        ext,
        "md" | "mdx" | "rst" | "adoc" | "asciidoc" | "tex" | "org"
    ) || name.starts_with("readme")
        || name.starts_with("changelog")
        || name.starts_with("license")
        || has_dir(&["docs", "documentation", "man"]) && !SOURCE_EXTENSIONS.contains(&ext)
    {
        return Kind::Docs;
    }
    if matches!(ext, "css" | "scss" | "sass" | "less" | "styl") {
        return Kind::Style;
    }
    if SOURCE_EXTENSIONS.contains(&ext) {
        return Kind::Source;
    }
    if matches!(
        ext,
        "png"
            | "jpg"
            | "jpeg"
            | "svg"
            | "gif"
            | "webp"
            | "avif"
            | "ico"
            | "pdf"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "mp3"
            | "wav"
            | "mp4"
            | "webm"
            | "zip"
            | "gz"
            | "jar"
            | "exe"
            | "dll"
            | "bin"
            | "db"
            | "db3"
    ) {
        return Kind::Asset;
    }
    if matches!(
        ext,
        "json"
            | "yaml"
            | "yml"
            | "toml"
            | "ini"
            | "xml"
            | "plist"
            | "properties"
            | "conf"
            | "env"
            | "editorconfig"
            | "tf"
            | "hcl"
            | "bicep"
    ) || name.starts_with(".env")
        || name.contains(".config.")
    {
        return Kind::Config;
    }
    Kind::Other
}

#[derive(Default)]
pub(super) struct Hints {
    links: Vec<String>,
    dependency_names: Vec<String>,
    additions: u16,
    removals: u16,
    added_api: u16,
    removed_api: u16,
    guards: u16,
    performance: u16,
    formatting: u16,
    evidence: Evidence,
    pub(super) behavior: behavior::Summary,
    relative_links: Vec<String>,
    data_tables: Vec<String>,
    bodies: Vec<Body>,
}

struct Body {
    name: String,
    added: u32,
    removed: u32,
    lines: u16,
}

impl Hints {
    pub(super) fn removes_symbol(&self) -> bool {
        self.evidence
            .facts()
            .any(|(topic, action, _)| topic == Topic::Symbol && action == Action::Remove)
    }

    pub(super) fn observe_body(&mut self, line: behavior::Line<'_>, added: bool) {
        self.behavior.observe(line, added);
        if line.signature == 0 || line.scope.is_empty() {
            return;
        }
        let index = match self.bodies.iter().position(|body| body.name == line.scope) {
            Some(index) => index,
            None if self.bodies.len() < 6 => {
                self.bodies.push(Body {
                    name: line.scope.into(),
                    added: 0,
                    removed: 0,
                    lines: 0,
                });
                self.bodies.len() - 1
            }
            None => return,
        };
        let body = &mut self.bodies[index];
        let side = if added {
            &mut body.added
        } else {
            &mut body.removed
        };
        *side = side.wrapping_add(line.signature);
        body.lines = body.lines.saturating_add(1);
    }

    fn changed_bodies(&self) -> impl Iterator<Item = &Body> {
        self.bodies.iter().filter(|body| body.added != body.removed)
    }
}

impl Hints {
    pub(super) fn observe(
        &mut self,
        path: &str,
        kind: Kind,
        bytes: &[u8],
        addition: bool,
        preview: bool,
    ) {
        let Ok(line) = std::str::from_utf8(bytes.get(..bytes.len().min(700)).unwrap_or(bytes))
        else {
            return;
        };
        let line = line.trim();
        let lower = line.to_ascii_lowercase();
        let ext = extension(path);
        self.evidence.observe(path, kind, line, &lower, addition);
        if line.is_empty()
            || line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with("* ")
            || line.starts_with('#')
                && !matches!(
                    ext,
                    "h" | "c"
                        | "cpp"
                        | "cc"
                        | "hpp"
                        | "cs"
                        | "swift"
                        | "cu"
                        | "cuh"
                        | "m"
                        | "mm"
                        | "ino"
                )
        {
            self.formatting = self.formatting.saturating_add(1);
            return;
        }
        if preview && kind == Kind::Source && self.data_tables.len() < 6 {
            for table in cohesion::referenced_tables(line) {
                if !self.data_tables.contains(&table) {
                    self.data_tables.push(table);
                    if self.data_tables.len() == 6 {
                        break;
                    }
                }
            }
        }
        if self.dependency_names.len() < 24 && kind == Kind::Build {
            let assignment = line.split_once('=').or_else(|| line.split_once(':'));
            let dependency = assignment
                .map(|(key, _)| key.trim().trim_matches(['"', '\'']))
                .or_else(|| {
                    line.split_whitespace().next().filter(|_| {
                        basename(path) == "go.mod" || basename(path).starts_with("requirements")
                    })
                });
            if let Some(key) = dependency.filter(|key| {
                key.len() > 1
                    && key.len() < 100
                    && key
                        .bytes()
                        .all(|ch| ch.is_ascii_alphanumeric() || b"@_./-".contains(&ch))
                    && !matches!(
                        *key,
                        "name"
                            | "version"
                            | "description"
                            | "scripts"
                            | "dependencies"
                            | "devDependencies"
                            | "license"
                            | "private"
                            | "edition"
                            | "module"
                            | "go"
                            | "lockfileVersion"
                            | "requires"
                    )
            }) {
                let key = stem(basename(key));
                if !self.dependency_names.contains(&key) {
                    self.dependency_names.push(key);
                }
            }
        }
        let api = [
            "export ",
            "pub ",
            "public ",
            "def ",
            "async def ",
            "func ",
            "function ",
            "class ",
            "interface ",
            "struct ",
            "enum ",
            "trait ",
            "impl ",
            "type ",
            "fn ",
            "fun ",
            "create table ",
            "create index ",
            "rpc ",
            "message ",
            "mutation ",
            "query ",
        ]
        .iter()
        .any(|prefix| lower.starts_with(prefix));
        if addition {
            self.additions = self.additions.saturating_add(1);
            self.added_api = self.added_api.saturating_add(api as u16);
            if [
                "if ",
                "if(",
                "guard ",
                "catch ",
                "except ",
                "unwrap_or",
                "return err",
                "return none",
                "is_none()",
                "== null",
                "?? ",
            ]
            .iter()
            .any(|token| lower.contains(token))
            {
                self.guards = self.guards.saturating_add(1);
            }
            if [
                "memoiz",
                "lazyload",
                "lazy_load",
                "cache.",
                "with_capacity(",
                "buffered",
                "batch_size",
                "create index ",
            ]
            .iter()
            .any(|token| lower.contains(token))
            {
                self.performance = self.performance.saturating_add(1);
            }
        } else {
            self.removals = self.removals.saturating_add(1);
            self.removed_api = self.removed_api.saturating_add(api as u16);
        }

        if self.links.len() >= 24
            || ![
                "import ",
                "from ",
                "require(",
                "using ",
                "use ",
                "mod ",
                "include ",
                "#include",
                "#import",
                "@import ",
                "@use ",
                "@forward ",
                "require_relative ",
                "part ",
                "extends ",
                "alias ",
                "require ",
                "extern crate ",
                "source ",
                "include!(",
                "import(",
            ]
            .iter()
            .any(|prefix| lower.starts_with(prefix))
                && !lower.contains("require(")
                && !lower.contains(" from ")
                && !lower.contains("import(")
        {
            return;
        }
        for token in line.split(|ch: char| {
            !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
        }) {
            if (token.starts_with("./")
                || token.starts_with("../")
                || (lower.starts_with("#include")
                    && token.contains('.')
                    && !token.contains(':')
                    && !token.starts_with('/')))
                && self.relative_links.len() < 16
            {
                let mut parts: Vec<_> = directory(path)
                    .split('/')
                    .filter(|part| !part.is_empty())
                    .collect();
                let mut valid = true;
                for part in token.split('/') {
                    match part {
                        "" | "." => {}
                        ".." => {
                            valid &= parts.pop().is_some();
                        }
                        value => parts.push(value),
                    }
                }
                if valid && !parts.is_empty() {
                    let resolved = parts.join("/");
                    if !self.relative_links.contains(&resolved) {
                        self.relative_links.push(resolved);
                    }
                }
            }
            if matches!(ext, "py" | "pyi")
                && lower.starts_with("from ")
                && token.starts_with('.')
                && !token.contains('/')
                && self.relative_links.len() < 16
            {
                let leading = token.bytes().take_while(|b| *b == b'.').count();
                let mut parts: Vec<_> = directory(path)
                    .split('/')
                    .filter(|part| !part.is_empty())
                    .collect();
                if leading <= parts.len() + 1 {
                    parts.truncate(parts.len() + 1 - leading);
                    parts.extend(token[leading..].split('.').filter(|part| !part.is_empty()));
                    let resolved = parts.join("/");
                    if !resolved.is_empty() && !self.relative_links.contains(&resolved) {
                        self.relative_links.push(resolved);
                    }
                }
            }
            let token = token.trim_matches('.');
            if token.len() < 2
                || token.len() > 120
                || matches!(
                    token,
                    "from"
                        | "import"
                        | "export"
                        | "public"
                        | "using"
                        | "crate"
                        | "super"
                        | "self"
                        | "require"
                        | "mod"
                        | "use"
                        | "include"
                        | "static"
                        | "default"
                        | "as"
                )
            {
                continue;
            }
            let canonical = if token.contains('/') {
                stem(basename(token))
            } else {
                token
                    .rsplit("::")
                    .next()
                    .unwrap_or(token)
                    .rsplit('.')
                    .next()
                    .unwrap_or(token)
                    .to_ascii_lowercase()
                    .replace(['-', '_'], "")
            };
            if canonical.len() >= 2 && !self.links.contains(&canonical) {
                self.links.push(canonical);
            }
            if self.links.len() == 24 {
                break;
            }
        }
    }
}

fn stem(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    let mut base = name
        .rsplit_once('.')
        .map_or(name.as_str(), |(base, _)| base);
    for prefix in ["test_", "test-", "spec_", "spec-"] {
        base = base.strip_prefix(prefix).unwrap_or(base);
    }
    for suffix in [
        ".spec",
        ".test",
        ".stories",
        ".module",
        "_test",
        "-test",
        "_spec",
        "-spec",
        "_pb2_grpc",
        "_pb2",
        "_grpc",
        "tests",
        "test",
        "spec",
        ".generated",
        ".pb",
        ".freezed",
        ".g",
    ] {
        base = base
            .strip_suffix(suffix)
            .filter(|value| !value.is_empty())
            .unwrap_or(base);
    }
    base.replace(['-', '_'], "")
}

fn generic_stem(name: &str) -> bool {
    matches!(
        name,
        "index"
            | "main"
            | "lib"
            | "mod"
            | "init"
            | "utils"
            | "helpers"
            | "types"
            | "config"
            | "app"
    )
}
fn directory(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn area(path: &str) -> String {
    let parts: Vec<_> = path.split('/').collect();
    let dir = &parts[..parts.len() - 1];
    for (i, part) in dir.iter().enumerate() {
        if matches!(
            *part,
            "packages"
                | "apps"
                | "crates"
                | "modules"
                | "services"
                | "libs"
                | "plugins"
                | "projects"
        ) && dir.get(i + 1).is_some()
            && (i == 0 || matches!(dir[i - 1], "software" | "src"))
        {
            return dir[..i + 2].join("/");
        }
    }
    for (i, part) in dir.iter().enumerate().rev() {
        if matches!(*part, "src" | "tests" | "test" | "docs" | "lib" | "spec") {
            if i > 0 {
                return dir[..i].join("/");
            }
            return String::new();
        }
    }
    String::new()
}

fn component(path: &str) -> String {
    let module = area(path);
    let rest = path
        .strip_prefix(&module)
        .unwrap_or(path)
        .trim_start_matches('/');
    let segments: Vec<_> = directory(rest).split('/').collect();
    let useful = segments.into_iter().rev().find(|value| {
        !matches!(
            *value,
            "" | "src"
                | "lib"
                | "app"
                | "main"
                | "source"
                | "tests"
                | "test"
                | "java"
                | "kotlin"
                | "com"
                | "org"
                | "features"
                | "components"
                | "views"
                | "styles"
                | "__tests__"
                | "docs"
                | "migrations"
        )
    });
    let module = basename(&module);
    useful
        .or_else(|| (!module.is_empty()).then_some(module))
        .unwrap_or("")
        .to_ascii_lowercase()
}

struct Sets(Vec<usize>);
impl Sets {
    fn new(size: usize) -> Self {
        Self((0..size).collect())
    }
    fn root(&mut self, i: usize) -> usize {
        if self.0[i] != i {
            self.0[i] = self.root(self.0[i]);
        }
        self.0[i]
    }
    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.root(a), self.root(b));
        self.0[a.max(b)] = a.min(b);
    }
}

fn covers(prefix: &str, path: &str) -> bool {
    prefix.is_empty()
        || path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with('/'))
}
fn endpoints(file: &Change) -> impl Iterator<Item = &str> {
    std::iter::once(file.path.as_str()).chain(file.old_path.as_deref())
}

fn matches_import(path: &str, import: &str) -> bool {
    path == import
        || path.strip_prefix(import).is_some_and(|tail| {
            (tail.starts_with('.') && !tail[1..].contains('/'))
                || matches!(
                    tail,
                    "/index.ts" | "/index.tsx" | "/index.js" | "/mod.rs" | "/__init__.py"
                )
        })
}

pub(super) fn atomic_units(files: &[Change]) -> Vec<Unit> {
    let mut sets = Sets::new(files.len());
    let areas: Vec<_> = files.iter().map(|file| area(&file.path)).collect();
    let stems: Vec<_> = files
        .iter()
        .map(|file| stem(basename(&file.path)))
        .collect();
    let mut unique_stems = HashMap::new();
    for (i, _) in files
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f.kind, Kind::Source | Kind::Schema))
    {
        unique_stems
            .entry((areas[i].as_str(), stems[i].as_str()))
            .and_modify(|entry| *entry = None)
            .or_insert(Some(i));
    }
    for (i, file) in files.iter().enumerate() {
        for link in &file.hints.links {
            if !generic_stem(link) {
                if let Some(Some(target)) = unique_stems.get(&(areas[i].as_str(), link.as_str())) {
                    sets.join(i, *target);
                }
            }
        }
        for (j, other) in files.iter().enumerate().take(i) {
            let attrs = |attr: &Change, candidate: &Change| {
                basename(&attr.path) == ".gitattributes"
                    && candidate.new.is_some()
                    && covers(directory(&attr.path), &candidate.path)
            };
            if must_move_together(file, other) || attrs(file, other) || attrs(other, file) {
                sets.join(i, j);
                continue;
            }
            if file.path == other.path {
                continue;
            }
            let imports = |source: &Change, target: &Change| {
                matches!(
                    target.kind,
                    Kind::Source
                        | Kind::Schema
                        | Kind::Test
                        | Kind::Style
                        | Kind::Asset
                        | Kind::Generated
                ) && source
                    .hints
                    .relative_links
                    .iter()
                    .any(|import| endpoints(target).any(|path| matches_import(path, import)))
            };
            if imports(file, other) || imports(other, file) {
                sets.join(i, j);
                continue;
            }
            if areas[i] != areas[j] {
                continue;
            }
            let (a, b) = (file.kind, other.kind);
            let build = matching_build_files(file, other) || matching_build_files(other, file);
            let native_pair = a == Kind::Source
                && b == Kind::Source
                && stems[i] == stems[j]
                && !generic_stem(&stems[i])
                && directory(&file.path) == directory(&other.path)
                && ["h", "hpp", "hh", "hxx"].contains(&extension(&file.path))
                    != ["h", "hpp", "hh", "hxx"].contains(&extension(&other.path))
                && ["c", "cc", "cpp", "cxx", "m", "mm", "cu"]
                    .iter()
                    .any(|ext| *ext == extension(&file.path) || *ext == extension(&other.path));
            let generated = matches!(
                (a, b),
                (Kind::Generated, Kind::Schema) | (Kind::Schema, Kind::Generated)
            ) && stems[i] == stems[j];
            let template = stems[i] == stems[j]
                && directory(&file.path) == directory(&other.path)
                && (is_template(&file.path) || is_template(&other.path))
                && matches!(
                    (a, b),
                    (Kind::Source, Kind::Source)
                        | (Kind::Source, Kind::Style)
                        | (Kind::Style, Kind::Source)
                );
            let migration = matches!(
                (a, b),
                (Kind::Schema, Kind::Migration)
                    | (Kind::Migration, Kind::Schema)
                    | (Kind::Migration, Kind::Migration)
            ) && migration_root(&file.path) == migration_root(&other.path);
            let possible_move = matches!(
                (file.status, other.status),
                ("added", "deleted") | ("deleted", "added")
            ) && stems[i] == stems[j]
                && !generic_stem(&stems[i])
                && extension(&file.path) == extension(&other.path)
                && (a != Kind::Migration && b != Kind::Migration
                    || migration_root(&file.path) == migration_root(&other.path));
            let manifest_import = |manifest: &Change, source: &Change| {
                manifest.kind == Kind::Build
                    && source.kind == Kind::Source
                    && manifest
                        .hints
                        .dependency_names
                        .iter()
                        .any(|name| source.hints.links.contains(name))
            };
            let source_and_manifest = manifest_import(file, other) || manifest_import(other, file);
            if build
                || native_pair
                || generated
                || template
                || migration
                || possible_move
                || source_and_manifest
            {
                sets.join(i, j);
            }
        }
    }
    collect_units(sets, files.len())
}

fn must_move_together(file: &Change, other: &Change) -> bool {
    let gitlink = |a: &Change, b: &Change| {
        basename(&a.path) == ".gitmodules"
            && b.new.or(b.old).is_some_and(|(_, mode)| mode == 0o160000)
    };
    if file.path == other.path && file.chunk.is_some() && other.chunk.is_some() {
        super::chunks::related(file, other)
    } else {
        endpoints(file).any(|a| endpoints(other).any(|b| covers(a, b) || covers(b, a)))
            || gitlink(file, other)
            || gitlink(other, file)
    }
}

pub(super) fn movable_units(files: &[Change]) -> Vec<Unit> {
    let mut sets = Sets::new(files.len());
    for (i, file) in files.iter().enumerate() {
        for (j, other) in files[..i].iter().enumerate() {
            if must_move_together(file, other) {
                sets.join(i, j);
            }
        }
    }
    collect_units(sets, files.len())
}

fn collect_units(mut sets: Sets, count: usize) -> Vec<Unit> {
    let mut units: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..count {
        units.entry(sets.root(i)).or_default().push(i);
    }
    units
        .into_values()
        .enumerate()
        .map(|(id, changes)| Unit { id, changes })
        .collect()
}

fn is_template(path: &str) -> bool {
    matches!(
        extension(path),
        "html" | "htm" | "razor" | "xaml" | "vue" | "svelte" | "blade" | "hbs" | "qml" | "ui"
    )
}

fn migration_root(path: &str) -> &str {
    let dir = directory(path);
    let mut at = 0;
    for part in dir.split('/') {
        if matches!(
            part,
            "migrations" | "migration" | "alembic" | "flyway" | "liquibase"
        ) {
            return dir[..at].trim_end_matches('/');
        }
        at += part.len() + 1;
    }
    dir
}

fn matching_build_files(manifest: &Change, companion: &Change) -> bool {
    if manifest.kind != Kind::Build || directory(&manifest.path) != directory(&companion.path) {
        return false;
    }
    let basename_a = basename(&manifest.path).to_ascii_lowercase();
    let basename_b = basename(&companion.path).to_ascii_lowercase();
    let lockfiles: &[&str] = match basename_a.as_str() {
        "package.json" => &[
            "package-lock.json",
            "npm-shrinkwrap.json",
            "pnpm-lock.yaml",
            "yarn.lock",
            "bun.lock",
            "bun.lockb",
        ],
        "cargo.toml" => &["cargo.lock"],
        "pyproject.toml" | "pdm.toml" => &["poetry.lock", "uv.lock", "pdm.lock"],
        "go.mod" => &["go.sum"],
        "go.work" => &["go.work.sum"],
        "gemfile" => &["gemfile.lock"],
        "pipfile" => &["pipfile.lock"],
        "composer.json" => &["composer.lock"],
        "pubspec.yaml" => &["pubspec.lock"],
        "package.swift" => &["package.resolved"],
        "podfile" => &["podfile.lock"],
        "mix.exs" => &["mix.lock"],
        "deno.json" | "deno.jsonc" => &["deno.lock"],
        "flake.nix" => &["flake.lock"],
        "pixi.toml" => &["pixi.lock"],
        "gleam.toml" => &["gleam.lock", "manifest.toml"],
        "shard.yml" => &["shard.lock"],
        "conanfile.py" | "conanfile.txt" => &["conan.lock"],
        "vcpkg.json" => &["vcpkg-lock.json"],
        "cartfile" => &["cartfile.resolved"],
        "build.gradle" | "build.gradle.kts" => &[
            "gradle.lockfile",
            "settings.gradle",
            "settings.gradle.kts",
            "gradle.properties",
        ],
        name if name.ends_with(".csproj") || name.ends_with(".fsproj") => &["packages.lock.json"],
        _ => &[],
    };
    lockfiles.contains(&basename_b.as_str())
}

fn primary(files: &[&Change]) -> Kind {
    const PRIORITY: &[Kind] = &[
        Kind::Source,
        Kind::Migration,
        Kind::Schema,
        Kind::Build,
        Kind::Lock,
        Kind::Ci,
        Kind::Test,
        Kind::Docs,
        Kind::Style,
        Kind::Config,
        Kind::Git,
        Kind::Generated,
        Kind::Asset,
        Kind::Other,
    ];
    PRIORITY
        .iter()
        .copied()
        .find(|kind| files.iter().any(|file| file.kind == *kind))
        .unwrap_or(Kind::Other)
}

fn main_behavior(file: &Change) -> Option<Behavior<'_>> {
    let mut best = None;
    let mut score = 0;
    for change in file.hints.behavior.changes() {
        let current = u16::from(change.strength.min(5)) * 4
            + u16::from(!change.scope.is_empty()) * 3
            + u16::from(change.added)
            + u16::from(behavior_specific(change.signal));
        if current > score {
            score = current;
            best = Some(change);
        }
    }
    best
}

fn behavior_specific(signal: Signal) -> bool {
    !matches!(
        signal,
        Signal::ErrorHandling | Signal::RequestHandling | Signal::FileIo | Signal::Rendering
    )
}

fn group_key(files: &[&Change]) -> (u8, String, Kind, u8) {
    let kind = primary(files);
    let anchor = files.iter().find(|f| f.kind == kind).unwrap();
    let module = area(&anchor.path);
    let key = if let Some(chunk) = &anchor.chunk {
        format!("{}/{}", anchor.path, chunk.scope)
    } else {
        match kind {
            Kind::Source | Kind::Test | Kind::Style | Kind::Asset | Kind::Other => {
                format!("{module}/{}", component(&anchor.path))
            }
            Kind::Migration => migration_root(&anchor.path).into(),
            _ => module,
        }
    };
    let order = match kind {
        Kind::Git => 0,
        Kind::Build | Kind::Lock => 1,
        Kind::Schema | Kind::Migration | Kind::Config => 2,
        Kind::Source | Kind::Style | Kind::Asset | Kind::Other | Kind::Generated => 3,
        Kind::Test => 4,
        Kind::Ci => 5,
        Kind::Docs => 6,
    };
    let family = if kind == Kind::Source {
        let mut changes = files
            .iter()
            .filter(|file| file.kind == Kind::Source)
            .filter_map(|file| {
                main_behavior(file)
                    .filter(|change| !change.scope.is_empty() && behavior_specific(change.signal))
                    .map(|change| change.signal.family())
            });
        changes
            .next()
            .filter(|family| changes.all(|next| next == *family))
            .unwrap_or(0)
    } else {
        0
    };
    (order, key, kind, family)
}

fn readable_word(bare: &str) -> String {
    let mut result = String::new();
    let mut lower = false;
    let mut chars = bare.chars().peekable();
    let mut previous = None;
    while let Some(ch) = chars.next() {
        let acronym_end = ch.is_ascii_uppercase()
            && previous.is_some_and(|prev: char| prev.is_ascii_uppercase())
            && chars.peek().is_some_and(char::is_ascii_lowercase);
        if matches!(ch, '-' | '_' | '.') || (ch.is_ascii_uppercase() && lower) || acronym_end {
            result.push(' ');
        }
        if !matches!(ch, '-' | '_' | '.') {
            result.push(ch.to_ascii_lowercase());
        }
        lower = ch.is_ascii_lowercase();
        previous = Some(ch);
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn combined_with_reason(
    files: &[Change],
    units: &[Unit],
    selected: Vec<usize>,
    reason: &str,
) -> Group {
    explain_group(combined(files, units, selected), reason)
}

pub(super) fn combined(files: &[Change], units: &[Unit], selected: Vec<usize>) -> Group {
    let changes: Vec<_> = selected
        .iter()
        .flat_map(|id| units[*id].changes.iter().map(|id| &files[*id]))
        .collect();
    draft(&changes, selected, None)
}

pub(super) fn compose(files: &[Change]) -> (Vec<Unit>, Vec<Group>) {
    let units = atomic_units(files);
    let mut groups: Vec<_> = cohesion::group(files, &units)
        .into_iter()
        .map(|proposal| {
            let changes: Vec<_> = proposal
                .units
                .iter()
                .flat_map(|id| units[*id].changes.iter().map(|i| &files[*i]))
                .collect();
            (group_key(&changes), proposal)
        })
        .collect();
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    while groups.len() > SUGGESTED_GROUPS {
        let position = groups
            .windows(2)
            .enumerate()
            .min_by_key(|(_, pair)| {
                let (a, b) = (&pair[0], &pair[1]);
                (
                    a.0 .0 != b.0 .0,
                    a.0 .1 != b.0 .1,
                    a.0 .2 != b.0 .2,
                    a.0 .3 != b.0 .3,
                    a.1.units.len() + b.1.units.len(),
                )
            })
            .map(|(i, _)| i)
            .unwrap();
        let (_, right) = groups.remove(position + 1);
        groups[position].1.units.extend(right.units);
        groups[position].1.reason = None;
        groups[position].1.feature = None;
    }
    let groups = groups
        .into_iter()
        .map(|(_, proposal)| {
            let changes: Vec<_> = proposal
                .units
                .iter()
                .flat_map(|id| units[*id].changes.iter().map(|index| &files[*index]))
                .collect();
            let group = draft(&changes, proposal.units, proposal.feature.as_deref());
            if let Some(reason) = proposal.reason {
                explain_group(group, reason)
            } else {
                group
            }
        })
        .collect();
    (units, groups)
}
