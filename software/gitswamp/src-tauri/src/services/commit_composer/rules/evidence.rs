use super::{basename, extension, Kind};

const LIMIT: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Topic {
    Table,
    Index,
    View,
    Route,
    Symbol,
    Resource,
    Selector,
    Dependency,
    Job,
    Target,
    Setting,
    Section,
}

impl Topic {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Index => "index",
            Self::View => "view",
            Self::Route => "route",
            Self::Symbol => "",
            Self::Resource => "resource",
            Self::Selector => "selector",
            Self::Dependency => "dependency",
            Self::Job => "CI job",
            Self::Target => "build target",
            Self::Setting => "setting",
            Self::Section => "section",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Add,
    Update,
    Remove,
}

impl Action {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Update => "update",
            Self::Remove => "remove",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Item {
    topic: Topic,
    name: String,
}

#[derive(Default)]
pub(super) struct Evidence {
    added: Vec<Item>,
    removed: Vec<Item>,
    explicit: Vec<(Topic, Action, String)>,
}

impl Evidence {
    pub(super) fn observe(
        &mut self,
        path: &str,
        kind: Kind,
        original: &str,
        lower: &str,
        added: bool,
    ) {
        if kind == Kind::Migration || extension(path).eq_ignore_ascii_case("sql") {
            if let Some((topic, action, name)) = sql(original, lower) {
                if added && self.explicit.len() < LIMIT {
                    let item = (topic, action, name.into());
                    if !self.explicit.contains(&item) {
                        self.explicit.push(item);
                    }
                }
                return;
            }
        }
        let feature = match kind {
            Kind::Source | Kind::Schema | Kind::Test | Kind::Migration => route(original, lower)
                .map(|name| (Topic::Route, name))
                .or_else(|| symbol(original, extension(path)).map(|name| (Topic::Symbol, name))),
            Kind::Build => dependency(path, original)
                .map(|name| (Topic::Dependency, name))
                .or_else(|| build_target(path, original).map(|name| (Topic::Target, name))),
            Kind::Config => resource(path, original)
                .map(|name| (Topic::Resource, name))
                .or_else(|| setting(original).map(|name| (Topic::Setting, name))),
            Kind::Ci => ci_job(original).map(|name| (Topic::Job, name)),
            Kind::Docs => heading(original).map(|name| (Topic::Section, name)),
            Kind::Style => selector(original).map(|name| (Topic::Selector, name)),
            _ => None,
        };
        if let Some((topic, name)) = feature {
            let entries = if added {
                &mut self.added
            } else {
                &mut self.removed
            };
            if entries.len() < LIMIT
                && !entries
                    .iter()
                    .any(|entry| entry.topic == topic && entry.name == name)
            {
                entries.push(Item {
                    topic,
                    name: name.into(),
                });
            }
        }
    }

    pub(super) fn facts(&self) -> impl Iterator<Item = (Topic, Action, &str)> {
        self.explicit
            .iter()
            .map(|(topic, action, name)| (*topic, *action, name.as_str()))
            .chain(self.added.iter().map(|entry| {
                let action = if self.removed.contains(entry) {
                    Action::Update
                } else {
                    Action::Add
                };
                (entry.topic, action, entry.name.as_str())
            }))
            .chain(
                self.removed
                    .iter()
                    .filter(|entry| !self.added.contains(entry))
                    .map(|entry| (entry.topic, Action::Remove, entry.name.as_str())),
            )
    }
}

fn identifier(word: &str) -> Option<&str> {
    let word = word
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()?;
    (word.len() <= 80 && word.starts_with(|c: char| c == '_' || c.is_alphabetic())).then_some(word)
}

fn key_name(word: &str) -> Option<&str> {
    let word = word.trim().trim_matches(['"', '\'']);
    (word.len() <= 80
        && word.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '@')
        && word
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-/@".contains(&b)))
    .then_some(word)
}

pub(super) fn symbol<'a>(line: &'a str, ext: &str) -> Option<&'a str> {
    let mut words = line.split_ascii_whitespace();
    while let Some(word) = words.next() {
        match word {
            "pub" | "pub(crate)" | "pub(super)" | "public" | "protected" | "private" | "static"
            | "async" | "export" | "default" | "abstract" | "final" | "sealed" | "internal"
            | "inline" | "suspend" | "override" | "partial" | "unsafe" | "open" | "data"
            | "nonisolated" | "extern" | "local" | "mutating" => {}
            "fn" | "def" | "defp" | "defmodule" | "function" | "fun" | "class" | "interface"
            | "type" | "struct" | "enum" | "trait" | "record" | "protocol" | "module" | "model"
            | "message" | "rpc" | "service" | "query" | "mutation" | "fragment" | "contract"
            | "namespace" | "entity" | "architecture" | "procedure" | "subroutine" | "package" => {
                return identifier(words.next()?)
            }
            "func" => {
                let rest = line.split_once("func ")?.1.trim_start();
                return identifier(if rest.starts_with('(') {
                    rest.split_once(')')?.1.trim_start()
                } else {
                    rest
                });
            }
            "const" | "let" | "var" | "val"
                if matches!(
                    ext,
                    "ts" | "tsx"
                        | "js"
                        | "jsx"
                        | "vue"
                        | "svelte"
                        | "mts"
                        | "mjs"
                        | "kt"
                        | "swift"
                        | "scala"
                        | "rs"
                ) =>
            {
                let name = identifier(words.next()?)?;
                return if line.starts_with("export ")
                    || line.starts_with("pub ")
                    || (line.contains("=>")
                        && matches!(
                            ext,
                            "ts" | "tsx" | "js" | "jsx" | "vue" | "svelte" | "mts" | "mjs"
                        ))
                {
                    Some(name)
                } else {
                    None
                };
            }
            _ if word.starts_with("pub(") => {}
            _ => break,
        }
    }
    if matches!(ext, "clj" | "cljs" | "cljc" | "lisp" | "el" | "rkt" | "scm") {
        return [
            "(defn ",
            "(defn- ",
            "(defmacro ",
            "(defun ",
            "(defrecord ",
            "(defprotocol ",
        ]
        .into_iter()
        .find_map(|prefix| line.strip_prefix(prefix))
        .and_then(identifier);
    }
    if matches!(
        ext,
        "ex" | "exs" | "ml" | "mli" | "hs" | "ocaml" | "erl" | "gleam"
    ) {
        if let Some(name) = ["let rec ", "let ", "val ", "-module("]
            .into_iter()
            .find_map(|prefix| line.strip_prefix(prefix))
            .and_then(identifier)
        {
            return Some(name);
        }
    }
    if ext == "r" && line.contains("<- function(") {
        return identifier(line);
    }
    if matches!(ext, "sh" | "bash" | "zsh") && line.contains("()") {
        return line.split_once("()").and_then(|(name, tail)| {
            (tail.trim() == "{" && !name.trim().contains(char::is_whitespace))
                .then(|| identifier(name.trim()))
                .flatten()
        });
    }
    if matches!(
        ext,
        "c" | "h"
            | "cc"
            | "hh"
            | "cpp"
            | "cxx"
            | "hpp"
            | "ino"
            | "cu"
            | "cs"
            | "java"
            | "dart"
            | "m"
            | "mm"
            | "groovy"
            | "zig"
            | "glsl"
            | "hlsl"
            | "wgsl"
    ) {
        let (prefix, rest) = line.split_once('(')?;
        let (_, tail) = rest.rsplit_once(')')?;
        if !matches!(tail.trim(), "" | ";" | "{" | "const" | "const {")
            && !tail.trim_start().starts_with("throws ")
        {
            return None;
        }
        let (result, name) = prefix.trim_end().rsplit_once(char::is_whitespace)?;
        if result.contains(['=', '.', '"', '\'', '(', ')'])
            || matches!(
                result.trim(),
                "return" | "throw" | "new" | "await" | "yield" | "else"
            )
            || matches!(
                name,
                "if" | "while" | "for" | "switch" | "catch" | "sizeof" | "match" | "return"
            )
        {
            return None;
        }
        let name = name.trim_start_matches(['*', '&']);
        return identifier(name).filter(|parsed| *parsed == name);
    }
    None
}

fn sql<'a>(line: &'a str, lower: &str) -> Option<(Topic, Action, &'a str)> {
    let mut lower = lower.split_ascii_whitespace();
    let mut action = match lower.next()? {
        "create" => Action::Add,
        "alter" => Action::Update,
        "drop" => Action::Remove,
        _ => return None,
    };
    let mut word = lower.next()?;
    let mut offset = 2;
    if word == "or" && lower.next()? == "replace" {
        word = lower.next()?;
        offset += 2;
        action = Action::Update;
    }
    if matches!(word, "unique" | "materialized" | "temporary") {
        word = lower.next()?;
        offset += 1;
    }
    let topic = match word {
        "table" => Topic::Table,
        "index" => Topic::Index,
        "view" => Topic::View,
        "function" | "procedure" | "trigger" | "type" => Topic::Symbol,
        _ => return None,
    };
    let mut original = line.split_ascii_whitespace().skip(offset);
    let mut name = original.next()?;
    if name.eq_ignore_ascii_case("concurrently") {
        name = original.next()?;
    }
    if name.eq_ignore_ascii_case("if") {
        let next = original.next()?;
        if next.eq_ignore_ascii_case("not") {
            original.next()?;
        }
        name = original.next()?;
    }
    let name = name
        .trim_start_matches(['"', '`', '['])
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
        .next()?;
    (!name.is_empty() && name.len() <= 80).then_some((topic, action, name))
}

fn route<'a>(line: &'a str, lower: &str) -> Option<&'a str> {
    let prefix = ["app.", "router.", "@app.", "@router.", "server.", "route::"]
        .into_iter()
        .find(|prefix| lower.starts_with(prefix))?;
    let rest = &line[prefix.len()..];
    let (method, args) = rest.split_once('(')?;
    let first = args.trim_start().as_bytes().first().copied()?;
    (matches!(first, b'\'' | b'"')
        && ["get", "post", "put", "patch", "delete", "head", "options"]
            .iter()
            .any(|verb| method.eq_ignore_ascii_case(verb)))
    .then_some(method)
}

fn dependency<'a>(path: &str, line: &'a str) -> Option<&'a str> {
    let name = basename(path);
    if name.ends_with(".csproj") || name.ends_with(".fsproj") || name.ends_with(".props") {
        return [
            "<PackageReference Include=\"",
            "<PackageReference Update=\"",
            "<PackageVersion Include=\"",
        ]
        .into_iter()
        .find_map(|prefix| line.strip_prefix(prefix))
        .and_then(|tail| tail.split_once('"'))
        .and_then(|(name, _)| key_name(name));
    }
    if name.starts_with("requirements") {
        let (key, _) = line.split_once(['=', '<', '>', '!', '~', '['])?;
        return key_name(key);
    }
    if name == "go.mod" {
        let line = line.strip_prefix("require ").unwrap_or(line);
        let mut fields = line.split_ascii_whitespace();
        let name = key_name(fields.next()?)?;
        return (name.contains('/') && fields.next()?.starts_with("v")).then_some(name);
    }
    if matches!(
        name,
        "build.gradle" | "build.gradle.kts" | "build.sbt" | "gemfile" | "podfile"
    ) || matches!(name, "Gemfile" | "Podfile")
    {
        let first = line.split(['"', '\'']).nth(1)?;
        let prefix = line.split(['"', '\'']).next()?.trim_end_matches([' ', '(']);
        if ![
            "implementation",
            "api",
            "compileOnly",
            "runtimeOnly",
            "testImplementation",
            "kapt",
            "ksp",
            "gem",
            "pod",
        ]
        .contains(&prefix)
        {
            return None;
        }
        let key = if matches!(prefix, "gem" | "pod") {
            first
        } else {
            first.split(':').nth(1)?
        };
        return key_name(key);
    }
    if !matches!(
        name,
        "package.json"
            | "Cargo.toml"
            | "cargo.toml"
            | "composer.json"
            | "pubspec.yaml"
            | "deno.json"
            | "Pipfile"
    ) {
        return None;
    }
    let (key, value) = line.split_once(['=', ':'])?;
    let key = key_name(key)?;
    let text = value.trim_start().trim_start_matches(['"', '\'']);
    if matches!(
        key,
        "name"
            | "version"
            | "edition"
            | "rust-version"
            | "license"
            | "repository"
            | "description"
            | "main"
            | "type"
            | "private"
    ) {
        return None;
    }
    (text.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '^' | '~' | '>' | '<' | '*'))
        || [
            "workspace:",
            "file:",
            "npm:",
            "jsr:",
            "{ version",
            "{version",
        ]
        .iter()
        .any(|prefix| text.starts_with(prefix)))
    .then_some(key)
}

fn build_target<'a>(path: &str, line: &'a str) -> Option<&'a str> {
    if !matches!(
        basename(path).to_ascii_lowercase().as_str(),
        "makefile" | "justfile" | "gnumakefile"
    ) {
        return None;
    }
    let (name, tail) = line.split_once(':')?;
    (!tail.starts_with('=')).then(|| key_name(name)).flatten()
}

fn setting(line: &str) -> Option<&str> {
    let (name, _) = line.split_once(['=', ':'])?;
    key_name(name)
}

fn resource<'a>(path: &str, line: &'a str) -> Option<&'a str> {
    if !matches!(extension(path), "tf" | "hcl") {
        return None;
    }
    let rest = [
        "resource ",
        "data ",
        "module ",
        "variable ",
        "output ",
        "provider ",
    ]
    .into_iter()
    .find_map(|prefix| line.strip_prefix(prefix))?;
    rest.split('"').nth(1).and_then(key_name)
}

fn selector(line: &str) -> Option<&str> {
    let name = line.strip_suffix('{')?.trim();
    if !name.starts_with('.') || name.len() > 65 {
        return None;
    }
    key_name(name.strip_prefix('.')?).filter(|key| !key.contains(['/', '@']))
}

fn ci_job(line: &str) -> Option<&str> {
    let name = line.strip_suffix(':')?;
    if [
        "name",
        "on",
        "jobs",
        "steps",
        "env",
        "with",
        "permissions",
        "strategy",
        "matrix",
        "resources",
        "trigger",
        "schedule",
        "runs-on",
        "push",
        "pull_request",
        "workflow_dispatch",
        "defaults",
        "run",
    ]
    .contains(&name)
    {
        return None;
    }
    key_name(name)
}

fn heading(line: &str) -> Option<&str> {
    let title = line.trim_start_matches('#').trim_start();
    if !line.starts_with('#') || title == line || title.len() > 70 {
        return None;
    }
    // Keep plain headings only; never copy URLs, code snippets or Markdown targets.
    (!title.is_empty()
        && title
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')))
    .then_some(title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_declarations_across_language_families() {
        for (extension, line, name) in [
            ("rs", "pub(crate) async fn audit_tree() {", "audit_tree"),
            ("py", "async def verify_branch(name):", "verify_branch"),
            (
                "go",
                "func (r *Repo) Fetch(ctx context.Context) error {",
                "Fetch",
            ),
            ("cpp", "bool isValid(int code) const {", "isValid"),
            (
                "java",
                "public String resolveBranch(String input) {",
                "resolveBranch",
            ),
            (
                "cs",
                "public static async Task RefreshCache() {",
                "RefreshCache",
            ),
            ("kt", "internal suspend fun loadStatus()", "loadStatus"),
            ("swift", "nonisolated func resolveTip() {", "resolveTip"),
            (
                "ts",
                "export const buildSeries = (paths) => {",
                "buildSeries",
            ),
            ("dart", "String renderCommit(int id) {", "renderCommit"),
            ("ex", "defp merge_entries(items) do", "merge_entries"),
            ("clj", "(defn compose [files]", "compose"),
            ("ml", "let rec walk tree =", "walk"),
            ("r", "review <- function(files) {", "review"),
            ("sh", "check_branch() {", "check_branch"),
            (
                "proto",
                "rpc ListBranches(ListRequest) returns (ListResponse);",
                "ListBranches",
            ),
            ("graphql", "type Commit {", "Commit"),
            ("sol", "contract Vault {", "Vault"),
            ("vhdl", "entity Comparator is", "Comparator"),
            ("f90", "subroutine compute_sum(x)", "compute_sum"),
        ] {
            assert_eq!(symbol(line, extension), Some(name), "{extension}: {line}");
        }
        assert_eq!(symbol("return close_branch(value);", "cpp"), None);
        assert_eq!(symbol("var apiKey = \"private-value\";", "js"), None);
    }

    #[test]
    fn distinguishes_updates_and_never_passes_config_values_to_messages() {
        let mut evidence = Evidence::default();
        for (line, added) in [
            ("retry_count=secret-old", false),
            ("retry_count=secret-new", true),
            ("legacy_host=sensitive", false),
        ] {
            evidence.observe("app.conf", Kind::Config, line, line, added);
        }
        let facts: Vec<_> = evidence.facts().collect();
        assert!(facts.contains(&(Topic::Setting, Action::Update, "retry_count")));
        assert!(facts.contains(&(Topic::Setting, Action::Remove, "legacy_host")));
        assert_eq!(facts.len(), 2);
        for (line, topic, action, name) in [
            (
                "ALTER TABLE accounts ADD COLUMN status TEXT;",
                Topic::Table,
                Action::Update,
                "accounts",
            ),
            (
                "CREATE OR REPLACE VIEW active_accounts AS SELECT 1;",
                Topic::View,
                Action::Update,
                "active_accounts",
            ),
            (
                "DROP INDEX IF EXISTS ix_accounts;",
                Topic::Index,
                Action::Remove,
                "ix_accounts",
            ),
        ] {
            assert!(sql(line, &line.to_ascii_lowercase())
                .is_some_and(|fact| fact == (topic, action, name)));
        }
    }

    #[test]
    fn recognizes_build_and_operational_topics_without_reading_values() {
        assert_eq!(
            dependency("go.mod", "github.com/org/tool v1.2.0"),
            Some("github.com/org/tool")
        );
        assert_eq!(
            dependency(
                "app.csproj",
                "<PackageReference Include=\"Serilog\" Version=\"4\" />"
            ),
            Some("Serilog")
        );
        assert_eq!(
            dependency("Cargo.toml", "serde = { version = \"1\" }"),
            Some("serde")
        );
        assert_eq!(
            dependency("requirements-dev.txt", "pytest>=8.0"),
            Some("pytest")
        );
        assert_eq!(
            dependency("build.gradle.kts", "implementation(\"org.example:util:1\")"),
            Some("util")
        );
        assert_eq!(
            build_target("Makefile", "test-all: build"),
            Some("test-all")
        );
        assert_eq!(setting("service_token: abc-secret"), Some("service_token"));
        assert_eq!(ci_job("workflow_dispatch:"), None);
        assert_eq!(ci_job("integration-tests:"), Some("integration-tests"));
        assert_eq!(
            resource("main.tf", "resource \"aws_vpc\" \"example\" {"),
            Some("aws_vpc")
        );
        assert_eq!(selector(".commit-row {"), Some("commit-row"));
        assert_eq!(
            route(
                "router.get('/secret/path', handler)",
                "router.get('/secret/path', handler)"
            ),
            Some("get")
        );
        assert_eq!(route("app.get(key)", "app.get(key)"), None);
        assert_eq!(
            heading("## Configure branch protection"),
            Some("Configure branch protection")
        );
        assert_eq!(heading("## link https://example.invalid/token"), None);
    }
}
