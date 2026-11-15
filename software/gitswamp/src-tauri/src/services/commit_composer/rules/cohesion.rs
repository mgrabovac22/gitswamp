use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use regex::Regex;

use super::{
    basename, directory, extension, generic_stem, readable_word, Change, Kind, Sets, Topic, Unit,
};

pub(super) struct Proposal {
    pub(super) units: Vec<usize>,
    pub(super) reason: Option<&'static str>,
    pub(super) feature: Option<String>,
}

#[derive(PartialEq, Eq)]
struct Feature<'a> {
    workspace: &'a str,
    namespace: String,
    name: String,
}

struct Named<'a> {
    feature: Feature<'a>,
    layer: Option<&'static str>,
}

const LAYERS: &[&str] = &[
    "request dto",
    "response dto",
    "query handler",
    "command handler",
    "controller",
    "handler",
    "service",
    "repository",
    "use case",
    "validator",
    "policy",
    "dto",
    "request",
    "response",
    "mapper",
    "component",
    "view",
    "screen",
    "client",
    "adapter",
];

fn workspace(path: &str) -> &str {
    let mut offset = 0;
    let mut parts = directory(path).split('/');
    for part in parts.by_ref() {
        offset += part.len() + 1;
        if matches!(
            part,
            "apps" | "packages" | "crates" | "services" | "projects" | "plugins" | "libs"
        ) {
            if let Some(name) = parts.next() {
                return &path[..offset + name.len()];
            }
            break;
        }
        if !matches!(part, "software" | "examples") {
            break;
        }
    }
    ""
}

fn locale(code: &str) -> bool {
    let lang = code.split(['-', '_']).next().unwrap_or("");
    matches!(lang.len(), 2 | 3) && lang.bytes().all(|ch| ch.is_ascii_lowercase())
}

fn localized(path: &str) -> bool {
    matches!(
        extension(path),
        "json" | "po" | "ftl" | "arb" | "yaml" | "yml"
    ) && directory(path)
        .split('/')
        .any(|dir| matches!(dir, "i18n" | "locales" | "translations"))
}

fn name_of(file: &Change) -> Option<Named<'_>> {
    let localization = localized(&file.path);
    if file.chunk.is_some()
        || !matches!(
            file.kind,
            Kind::Source | Kind::Test | Kind::Style | Kind::Docs
        ) && !localization
    {
        return None;
    }
    let path = &file.path;
    let workspace = workspace(path);
    let within = directory(path)
        .strip_prefix(workspace)?
        .trim_start_matches('/');
    let candidate = basename(path)
        .rsplit_once('.')
        .map_or(basename(path), |(base, _)| base);
    let candidate = candidate
        .rsplit_once('.')
        .filter(|(_, suffix)| localization && locale(suffix))
        .map_or(candidate, |(base, _)| base);
    let text = readable_word(candidate);
    let mut words: Vec<_> = text.split_ascii_whitespace().collect();
    if file.kind == Kind::Test {
        let alias = |word| {
            matches!(
                word,
                "test" | "tests" | "spec" | "specs" | "stories" | "fixture" | "fixtures" | "snap"
            )
        };
        while words.first().is_some_and(|word| alias(*word)) {
            words.remove(0);
        }
        while words.last().is_some_and(|word| alias(*word)) {
            words.pop();
        }
    }
    if file.kind == Kind::Docs
        && words
            .last()
            .is_some_and(|word| matches!(*word, "guide" | "usage" | "api"))
    {
        words.pop();
    }
    if file.kind == Kind::Style && words.last() == Some(&"module") {
        words.pop();
    }
    let mut layer = None;
    if matches!(file.kind, Kind::Source | Kind::Test | Kind::Style) {
        for role in LAYERS {
            let width = role.split_ascii_whitespace().count();
            if words.len() > width
                && words[words.len() - width..]
                    .iter()
                    .copied()
                    .eq(role.split_ascii_whitespace())
            {
                words.truncate(words.len() - width);
                layer = Some(*role);
                break;
            }
        }
    }
    if layer.is_none()
        && matches!(file.kind, Kind::Source | Kind::Test)
        && matches!(extension(path), "js" | "jsx" | "ts" | "tsx")
        && words.len() > 1
        && words[0] == "use"
    {
        words.remove(0);
        layer = Some("hook");
    }
    let name = words.join("-");
    if name.len() < 4
        || name.len() > 80
        || generic_stem(&name)
        || matches!(
            name.as_str(),
            "readme"
                | "messages"
                | "strings"
                | "common"
                | "shared"
                | "base"
                | "default"
                | "fixtures"
                | "settings"
        )
    {
        return None;
    }
    let mut remove_locale = false;
    let namespace = within
        .split('/')
        .filter(|part| {
            if matches!(*part, "i18n" | "locales" | "translations") && localization {
                remove_locale = true;
                return false;
            }
            if std::mem::take(&mut remove_locale) && locale(part) {
                return false;
            }
            !matches!(
                *part,
                "" | "src"
                    | "lib"
                    | "app"
                    | "main"
                    | "java"
                    | "kotlin"
                    | "swift"
                    | "test"
                    | "tests"
                    | "__tests__"
                    | "spec"
                    | "specs"
                    | "e2e"
                    | "integration"
                    | "fixtures"
                    | "__snapshots__"
                    | "snapshots"
                    | "testdata"
                    | "docs"
                    | "documentation"
                    | "features"
                    | "components"
                    | "views"
                    | "screens"
                    | "styles"
                    | "controllers"
                    | "hooks"
                    | "handlers"
                    | "services"
                    | "repositories"
                    | "usecases"
                    | "use_cases"
                    | "dto"
                    | "dtos"
                    | "validators"
                    | "models"
                    | "policies"
                    | "mappers"
            )
        })
        .collect::<Vec<_>>()
        .join("/");
    Some(Named {
        feature: Feature {
            workspace,
            namespace,
            name,
        },
        layer,
    })
}

fn table_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

pub(super) fn referenced_tables(line: &str) -> impl Iterator<Item = String> + '_ {
    static TABLES: OnceLock<Regex> = OnceLock::new();
    let candidate = line.contains(['"', '\'']) || line.contains('`');
    candidate
        .then(|| {
            TABLES.get_or_init(|| {
                Regex::new(
                    r#"(?ix)
        (?:["'`](?:select\b[^"'`]*?\bfrom|insert\s+into|delete\s+from|update)\s+
         |(?:\b__tablename__\s*=\s*|\btable\s*\(\s*(?:name\s*=\s*)?)["'])
        ([a-z_][a-z0-9_]*(?:\.[a-z_][a-z0-9_]*)?)(?:[\s"'`]|$)"#,
                )
                .expect("static table reference pattern")
            })
        })
        .into_iter()
        .flat_map(move |regex| regex.captures_iter(line).take(3))
        .filter_map(|capture| capture.get(1).filter(|name| name.len() <= 80))
        .map(|name| table_name(name.as_str()))
}

pub(super) fn group(files: &[Change], units: &[Unit]) -> Vec<Proposal> {
    let mut sets = Sets::new(units.len());
    let mut indexed: BTreeMap<_, Vec<_>> = BTreeMap::new();
    let names: Vec<_> = files.iter().map(name_of).collect();
    let unit_files = |id: usize| {
        units[id]
            .changes
            .iter()
            .map(|index| (*index, &files[*index]))
    };
    let mut anchors = Vec::new();
    for unit in units {
        let mut source = unit_files(unit.id).filter(|(_, file)| file.kind == Kind::Source);
        let Some((index, _)) = source.next() else {
            continue;
        };
        let Some(named) = &names[index] else { continue };
        if source.any(|(i, _)| {
            names[i]
                .as_ref()
                .is_none_or(|other| other.feature != named.feature)
        }) {
            continue;
        }
        let feature = &named.feature;
        indexed
            .entry((feature.workspace, feature.name.as_str()))
            .or_default()
            .push((unit.id, named));
        anchors.push((unit.id, feature));
    }
    for same_name in indexed.values() {
        let mut scopes: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (id, named) in same_name {
            scopes
                .entry(&named.feature.namespace)
                .or_default()
                .push((*id, named.layer));
        }
        for siblings in scopes.values() {
            let layers: BTreeSet<_> = siblings.iter().map(|(_, layer)| *layer).collect();
            if siblings.len() > 1 && layers.len() == siblings.len() {
                for (id, _) in &siblings[1..] {
                    sets.join(siblings[0].0, *id);
                }
            }
        }
    }
    let mut migration_units: BTreeMap<_, usize> = BTreeMap::new();
    for unit in units {
        if let Some((_, file)) = unit_files(unit.id).find(|(_, file)| file.kind == Kind::Migration)
        {
            *migration_units.entry(workspace(&file.path)).or_default() += 1;
        }
    }
    for unit in units {
        let descriptors: Vec<_> = unit_files(unit.id).collect();
        if descriptors
            .iter()
            .any(|(_, file)| file.kind == Kind::Source)
        {
            continue;
        }
        if let Some((_, first)) = descriptors
            .iter()
            .find(|(_, file)| file.kind == Kind::Migration)
        {
            let root = workspace(&first.path);
            if migration_units.get(root) != Some(&1) {
                continue;
            }
            let tables: BTreeSet<_> = descriptors
                .iter()
                .filter(|(_, file)| file.kind == Kind::Migration)
                .flat_map(|(_, file)| file.hints.evidence.facts())
                .filter(|(topic, _, _)| *topic == Topic::Table)
                .map(|(_, _, name)| table_name(name))
                .collect();
            if tables.len() != 1
                || descriptors.iter().any(|(_, file)| {
                    !matches!(file.kind, Kind::Migration | Kind::Schema | Kind::Generated)
                })
            {
                continue;
            }
            let Some(table) = tables.first() else {
                continue;
            };
            let candidates: BTreeSet<_> = anchors
                .iter()
                .filter_map(|(id, candidate)| {
                    (candidate.workspace == root
                        && unit_files(*id).any(|(_, file)| file.hints.data_tables.contains(table)))
                    .then(|| sets.root(*id))
                })
                .collect();
            if candidates.len() == 1 {
                sets.join(*candidates.first().unwrap(), unit.id);
            }
        } else {
            let mut matched = BTreeSet::new();
            for (i, _) in descriptors {
                let Some(named) = &names[i] else {
                    matched.clear();
                    break;
                };
                let feature = &named.feature;
                let Some(candidates) = indexed.get(&(feature.workspace, feature.name.as_str()))
                else {
                    matched.clear();
                    break;
                };
                let roots: BTreeSet<_> = candidates
                    .iter()
                    .filter_map(|(id, anchor)| {
                        (feature.namespace.is_empty()
                            || feature.namespace == anchor.feature.namespace)
                            .then(|| sets.root(*id))
                    })
                    .collect();
                if roots.len() != 1 {
                    matched.clear();
                    break;
                }
                matched.extend(roots);
            }
            if matched.len() == 1 {
                sets.join(*matched.first().unwrap(), unit.id);
            }
        }
    }
    let mut grouped: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for unit in units {
        grouped.entry(sets.root(unit.id)).or_default().push(unit.id);
    }
    grouped.into_values().map(|units| {
        let feature = if units.len() > 1 {
            anchors.iter().find(|(id, _)| units.contains(id))
                .map(|(_, feature)| feature.name.replace('-', " "))
        } else {
            None
        };
        let reason = if units.len() > 1 {
            let mut joined = units.iter().flat_map(|id| unit_files(*id));
            if joined.any(|(_, file)| file.kind == Kind::Migration) {
                Some("Keep the migration with the implementation that references the same table.")
            } else {
                Some("Keep the matching implementation and companion changes together for review.")
            }
        } else {
            None
        };
        Proposal { units, reason, feature }
    }).collect()
}
