mod evidence;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::Serialize;

use super::{Change, Group, Unit};
use evidence::{Action, Evidence, Topic};

const SUGGESTED_GROUPS: usize = 8;

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
    relative_links: Vec<String>,
}

impl Hints {
    pub(super) fn observe(&mut self, path: &str, kind: Kind, bytes: &[u8], addition: bool) {
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

fn valid_scope(value: &str) -> String {
    readable_word(value)
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(24)
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn scope_for(files: &[&Change], kind: Kind) -> String {
    let relevant: Vec<_> = files.iter().filter(|file| file.kind == kind).collect();
    let mut candidates = relevant.iter().map(|file| component(&file.path));
    let first = candidates.next().unwrap_or_default();
    let same = candidates.all(|component| component == first);
    let common = relevant[0]
        .path
        .split('/')
        .take_while(|segment| !segment.is_empty())
        .scan(String::new(), |prefix, segment| {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(segment);
            Some(prefix.clone())
        })
        .take_while(|prefix| {
            relevant
                .iter()
                .all(|file| covers(prefix, directory(&file.path)))
        })
        .last()
        .unwrap_or_default();
    let common = common
        .rsplit('/')
        .find(|name| !matches!(*name, "" | "src" | "lib" | "source" | "main"));
    let module = area(&relevant[0].path);
    let same_module = relevant.iter().all(|file| area(&file.path) == module);
    let name = basename(&relevant[0].path);
    let file_stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let fallback = match kind {
        Kind::Source => "core",
        Kind::Test => "tests",
        Kind::Docs => "docs",
        Kind::Style => "ui",
        Kind::Build | Kind::Lock => "deps",
        Kind::Ci => "ci",
        Kind::Schema | Kind::Migration => "data",
        Kind::Config => "config",
        Kind::Asset => "assets",
        Kind::Git => "git",
        Kind::Generated => "generated",
        Kind::Other => "repo",
    };
    let selected = [
        (same && !first.is_empty()).then_some(first.as_str()),
        common,
        (same_module && !module.is_empty()).then_some(basename(&module)),
        (relevant.len() == 1 && !generic_stem(file_stem)).then_some(file_stem),
    ]
    .into_iter()
    .flatten()
    .map(valid_scope)
    .find(|value| !value.is_empty());
    selected.unwrap_or_else(|| fallback.into())
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

fn atomic_units(files: &[Change]) -> Vec<Unit> {
    let mut sets = Sets::new(files.len());
    let areas: Vec<_> = files.iter().map(|file| area(&file.path)).collect();
    let stems: Vec<_> = files
        .iter()
        .map(|file| stem(basename(&file.path)))
        .collect();
    let components: Vec<_> = files.iter().map(|file| component(&file.path)).collect();
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
            let intersects =
                endpoints(file).any(|a| endpoints(other).any(|b| covers(a, b) || covers(b, a)));
            let attrs = |attr: &Change, candidate: &Change| {
                basename(&attr.path) == ".gitattributes"
                    && candidate.new.is_some()
                    && covers(directory(&attr.path), &candidate.path)
            };
            let gitlink = |a: &Change, b: &Change| {
                basename(&a.path) == ".gitmodules"
                    && b.new.or(b.old).is_some_and(|(_, mode)| mode == 0o160000)
            };
            if intersects
                || attrs(file, other)
                || attrs(other, file)
                || gitlink(file, other)
                || gitlink(other, file)
            {
                sets.join(i, j);
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
            let related_source =
                |kind| matches!(kind, Kind::Source | Kind::Schema | Kind::Migration);
            let build = matching_build_files(file, other) || matching_build_files(other, file);
            let companions = stems[i] == stems[j]
                && !generic_stem(&stems[i])
                && (!matches!(a, Kind::Migration) && !matches!(b, Kind::Migration)
                    || migration_root(&file.path) == migration_root(&other.path))
                && (related_source(a)
                    || related_source(b)
                    || matches!((a, b), (Kind::Test, Kind::Test)))
                && !matches!(a, Kind::Build | Kind::Lock | Kind::Ci | Kind::Git)
                && !matches!(b, Kind::Build | Kind::Lock | Kind::Ci | Kind::Git);
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
            let supporting_config = matches!(
                (a, b),
                (Kind::Config, Kind::Source) | (Kind::Source, Kind::Config)
            ) && !components[i].is_empty()
                && components[i] == components[j]
                && directory(&file.path) == directory(&other.path);
            let possible_move = matches!(
                (file.status, other.status),
                ("added", "deleted") | ("deleted", "added")
            ) && stems[i] == stems[j]
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
                || companions
                || generated
                || template
                || migration
                || supporting_config
                || possible_move
                || source_and_manifest
            {
                sets.join(i, j);
            }
        }
    }
    let mut units: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..files.len() {
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

fn group_key(files: &[&Change]) -> (u8, String, Kind) {
    let kind = primary(files);
    let anchor = files.iter().find(|f| f.kind == kind).unwrap();
    let module = area(&anchor.path);
    let key = match kind {
        Kind::Source | Kind::Test | Kind::Style | Kind::Asset | Kind::Other => {
            format!("{module}/{}", component(&anchor.path))
        }
        Kind::Migration => migration_root(&anchor.path).into(),
        _ => module,
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
    (order, key, kind)
}

fn title(kind: Kind, files: &[&Change]) -> (&'static str, &'static str) {
    match kind {
        Kind::Docs => ("Docs", "Document"),
        Kind::Test => ("Test", "Cover"),
        Kind::Build | Kind::Lock => ("Build", "Update dependencies and build files for"),
        Kind::Ci => ("Ci", "Configure workflows for"),
        Kind::Style => ("Style", "Adjust styles for"),
        Kind::Migration => ("Feat", "Migrate"),
        Kind::Schema => ("Feat", "Revise contracts for"),
        Kind::Config => ("Chore", "Configure"),
        Kind::Git => ("Chore", "Configure Git paths for"),
        Kind::Generated => ("Build", "Regenerate"),
        Kind::Asset => ("Chore", "Refresh assets for"),
        Kind::Other => ("Chore", "Revise files for"),
        Kind::Source => {
            let source: Vec<_> = files
                .iter()
                .filter(|file| file.kind == Kind::Source)
                .collect();
            let sum = |field: fn(&Hints) -> u16| {
                source
                    .iter()
                    .map(|file| usize::from(field(&file.hints)))
                    .sum::<usize>()
            };
            if source.iter().all(|file| file.status == "deleted") {
                ("Refactor", "Remove")
            } else if source.iter().all(|file| {
                file.status == "renamed" || file.old.map(|v| v.0) == file.new.map(|v| v.0)
            }) {
                ("Refactor", "Move or reorganize")
            } else if source.iter().any(|file| file.status == "added")
                || sum(|h| h.added_api) > sum(|h| h.removed_api)
            {
                ("Feat", "Extend")
            } else if sum(|h| h.performance) >= 2 {
                ("Refactor", "Revise caching or batching in")
            } else if sum(|h| h.guards) >= 2 {
                ("Refactor", "Update guards in")
            } else if sum(|h| h.additions) + sum(|h| h.removals) == 0 && sum(|h| h.formatting) > 0 {
                ("Style", "Tidy")
            } else {
                ("Refactor", "Revise")
            }
        }
    }
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

fn readable(path: &str) -> String {
    let name = basename(path);
    readable_word(name.rsplit_once('.').map_or(name, |(a, _)| a))
}

fn useful_facts(file: &Change) -> impl Iterator<Item = (Topic, Action, &str)> {
    file.hints
        .evidence
        .facts()
        .filter(|(topic, _, name)| *topic != Topic::Symbol || !generic_stem(name))
}

fn visible_fact(topic: Topic, action: Action, name: &str) -> String {
    let label = topic.label();
    if label.is_empty() {
        format!("{} {name}", action.label())
    } else if topic == Topic::Route {
        format!("{} {} {label}", action.label(), name.to_ascii_uppercase())
    } else {
        format!("{} {label} {name}", action.label())
    }
}

fn subject_fact<'a>(kind: Kind, files: &[&'a Change]) -> Option<(Topic, Action, &'a str)> {
    files
        .iter()
        .filter(|file| file.kind == kind)
        .flat_map(|file| useful_facts(file))
        .min_by_key(|(topic, action, _)| {
            (
                *topic,
                match action {
                    Action::Update => 0,
                    Action::Add => 1,
                    Action::Remove => 2,
                },
            )
        })
}

fn draft(files: &[&Change], unit_ids: Vec<usize>) -> Group {
    let kind = primary(files);
    let anchor = files.iter().find(|file| file.kind == kind).unwrap();
    let (mut prefix, mut verb) = title(kind, files);
    let scope = scope_for(files, kind);
    let anchor_target = readable(&anchor.path);
    let scope_label = readable_word(&scope);
    let target = if anchor.status == "renamed" && files.len() == 1 {
        verb = "Rename";
        format!(
            "{} to {anchor_target}",
            anchor.old_path.as_deref().map(readable).unwrap_or_default()
        )
    } else if let Some((topic, action, name)) = subject_fact(kind, files) {
        verb = match (topic, action) {
            (Topic::Table | Topic::Index | Topic::View, Action::Add) => "Create",
            (Topic::Table | Topic::Index | Topic::View, Action::Remove) => "Drop",
            (Topic::Section, _) => "Document",
            (_, Action::Add) => "Add",
            (_, Action::Update) => "Update",
            (_, Action::Remove) => "Remove",
        };
        if kind == Kind::Source && prefix == "Feat" && action != Action::Add {
            prefix = "Refactor";
        }
        match topic {
            Topic::Symbol | Topic::Section => readable_word(name),
            Topic::Route => format!("{} route in {anchor_target}", name.to_ascii_uppercase()),
            _ => format!("{} {name}", topic.label()),
        }
    } else if files.len() > 1 && !matches!(scope.as_str(), "core" | "repo") {
        scope_label
    } else {
        anchor_target
    };
    let target = if target.is_empty() {
        "repository files"
    } else {
        &target
    };
    let start = format!("{prefix}({scope}): {verb} ");
    let room = 72usize.saturating_sub(start.len());
    let mut bounded = String::new();
    for ch in target.chars() {
        if bounded.encode_utf16().count() + ch.len_utf16() > room {
            break;
        }
        bounded.push(ch);
    }
    if bounded.len() < target.len() {
        if let Some((words, _)) = bounded.rsplit_once(' ') {
            bounded.truncate(words.len());
        }
    }
    let subject = format!("{start}{bounded}")
        .trim_end_matches([' ', '.'])
        .to_owned();
    let mut description = String::new();
    let mut displayed: Vec<_> = files.iter().copied().enumerate().collect();
    displayed.sort_by_key(|(i, file)| (file.kind != kind, useful_facts(file).next().is_none(), *i));
    for (_, file) in displayed.into_iter().take(8) {
        if let Some(old) = &file.old_path {
            description.push_str(&format!("- Rename {old} to {}\n", file.path));
        } else {
            let action = match file.status {
                "added" => "Add",
                "deleted" => "Remove",
                "typechange" => "Replace",
                _ => "Update",
            };
            description.push_str(&format!("- {action} {}", file.path));
            let mut clues: Vec<_> = useful_facts(file)
                .take(4)
                .map(|(topic, action, name)| visible_fact(topic, action, name))
                .collect();
            if file.added + file.removed > 0 {
                clues.push(format!("+{}/-{} lines", file.added, file.removed));
            }
            if file.partial {
                clues.push("staged changes only".into());
            }
            if let (Some((old_id, old_mode)), Some((new_id, new_mode))) = (file.old, file.new) {
                if old_mode != new_mode
                    && [old_mode, new_mode]
                        .iter()
                        .all(|mode| matches!(mode, 0o100644 | 0o100755))
                {
                    clues.push(
                        if new_mode == 0o100755 {
                            "make executable"
                        } else {
                            "clear executable bit"
                        }
                        .into(),
                    );
                }
                if old_id != new_id && new_mode == 0o160000 {
                    clues.push(format!(
                        "submodule {} to {}",
                        &old_id.to_string()[..7],
                        &new_id.to_string()[..7]
                    ));
                }
            }
            if !file.kind.inspect_text() && file.old.map(|v| v.0) != file.new.map(|v| v.0) {
                let detail = match file.kind {
                    Kind::Asset => "asset contents",
                    Kind::Lock => "dependency resolution",
                    Kind::Generated => "generated output",
                    _ => unreachable!(),
                };
                clues.push(detail.into());
            }
            if !clues.is_empty() {
                description.push_str(&format!(" ({})", clues.join("; ")));
            }
            description.push('\n');
        }
    }
    if files.len() > 8 {
        description.push_str(&format!(
            "- Include {} more related paths\n",
            files.len() - 8
        ));
    }
    let (mut added, mut modified, mut deleted, mut moved) = (0, 0, 0, 0);
    for file in files {
        match file.status {
            "added" => added += 1,
            "deleted" => deleted += 1,
            "renamed" => moved += 1,
            _ => modified += 1,
        }
    }
    if files.len() > 1 {
        let counts: Vec<_> = [
            (added, "added"),
            (modified, "updated"),
            (deleted, "removed"),
            (moved, "renamed"),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, status)| format!("{count} {status}"))
        .collect();
        description.push_str(&format!("\n{} paths: {}.", files.len(), counts.join(", ")));
    }
    Group {
        subject,
        description: description.trim_end().into(),
        units: unit_ids,
    }
}

pub(super) fn compose(files: &[Change]) -> (Vec<Unit>, Vec<Group>) {
    let units = atomic_units(files);
    let mut sets: BTreeMap<_, Vec<usize>> = BTreeMap::new();
    for unit in &units {
        let changes: Vec<_> = unit.changes.iter().map(|i| &files[*i]).collect();
        sets.entry(group_key(&changes)).or_default().push(unit.id);
    }
    let mut groups: Vec<_> = sets.into_iter().collect();
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
                    a.1.len() + b.1.len(),
                )
            })
            .map(|(i, _)| i)
            .unwrap();
        let (_, right) = groups.remove(position + 1);
        groups[position].1.extend(right);
    }
    let groups = groups
        .into_iter()
        .map(|(_, unit_ids)| {
            let changes: Vec<_> = unit_ids
                .iter()
                .flat_map(|i| units[*i].changes.iter())
                .map(|i| &files[*i])
                .collect();
            draft(&changes, unit_ids)
        })
        .collect();
    (units, groups)
}
