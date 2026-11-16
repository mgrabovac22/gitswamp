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
    "dialog",
    "panel",
    "modal",
    "page",
    "form",
    "view",
    "screen",
    "client",
    "endpoint",
    "routes",
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

fn owned_feature(path: &str) -> Option<&str> {
    let (mut parent, _) = path.rsplit_once('/')?;
    let project = workspace(path);
    let within = parent.strip_prefix(project)?.trim_start_matches('/');
    let mut parts = within.split('/');
    let kind = parts.find(|part| {
        matches!(
            *part,
            "features"
                | "modules"
                | "domain"
                | "domains"
                | "services"
                | "workflows"
                | "components"
                | "controllers"
                | "views"
                | "screens"
                | "routes"
        )
    })?;
    let feature = parts.next()?;
    let structural = |part| {
        matches!(
            part,
            "components"
                | "controllers"
                | "hooks"
                | "views"
                | "screens"
                | "tests"
                | "__tests__"
                | "fixtures"
                | "styles"
                | "assets"
                | "validators"
                | "adapters"
                | "models"
                | "rules"
        )
    };
    let broad = |part| {
        matches!(
            part,
            "core"
                | "common"
                | "shared"
                | "base"
                | "app"
                | "ui"
                | "api"
                | "utils"
                | "internal"
                | "repository"
                | "settings"
                | "services"
                | "lib"
        )
    };
    if structural(feature)
        || (broad(feature)
            && parts
                .next()
                .is_none_or(|part| broad(part) || structural(part)))
    {
        return None;
    }
    // The module's conventional subfolders are not separate features.
    while let Some((ancestor, label)) = parent.rsplit_once('/') {
        if !structural(label) {
            break;
        }
        parent = ancestor;
    }
    let (_, label) = parent.rsplit_once('/')?;
    (!broad(label) && label != kind).then_some(parent)
}

fn entrypoint_owner<'a>(path: &'a str, known: &BTreeSet<&str>) -> Option<&'a str> {
    let (stem, _) = path.rsplit_once('.')?;
    known.contains(stem).then_some(stem)
}

fn feature_terms(value: &str) -> impl Iterator<Item = String> + '_ {
    let description = readable_word(value);
    let words: Vec<_> = description
        .split_whitespace()
        .filter(|token| {
            token.len() >= 4
                && token.bytes().all(|ch| ch.is_ascii_alphanumeric())
                && !generic_stem(token)
                && !matches!(
                    *token,
                    "component"
                        | "handler"
                        | "function"
                        | "service"
                        | "repository"
                        | "controller"
                        | "provider"
                        | "routes"
                        | "endpoint"
                        | "tests"
                        | "test"
                        | "state"
                        | "value"
                        | "data"
                        | "client"
                        | "configure"
                        | "format"
                        | "render"
                        | "change"
                        | "update"
                        | "create"
                        | "remove"
                        | "build"
                        | "check"
                        | "fetch"
                        | "load"
                        | "save"
                        | "setup"
                        | "validate"
                        | "apply"
                        | "resolve"
                        | "restore"
                        | "cancel"
                        | "normalize"
                        | "default"
                        | "result"
                        | "error"
                        | "message"
                        | "request"
                        | "response"
                        | "option"
                        | "options"
                        | "index"
                        | "file"
                        | "testcase"
                        | "supports"
                        | "adding"
                        | "using"
                        | "with"
                        | "from"
                )
        })
        .map(|token| {
            token
                .strip_suffix('s')
                .filter(|word| word.len() > 3 && !word.ends_with('s'))
                .unwrap_or(token)
                .to_owned()
        })
        .collect();
    words.into_iter()
}

fn diff_feature_terms(file: &Change) -> impl Iterator<Item = String> + '_ {
    file.hints
        .changed_bodies()
        .map(|body| body.name.as_str())
        .chain(file.hints.evidence.facts().filter_map(|(topic, _, name)| {
            matches!(
                topic,
                Topic::Symbol
                    | Topic::Endpoint
                    | Topic::Operation
                    | Topic::Command
                    | Topic::Selector
                    | Topic::Section
            )
            .then_some(name)
        }))
        .flat_map(feature_terms)
}

#[derive(Default)]
struct FeatureEvidence<'a> {
    owner: Option<&'a str>,
    terms: BTreeSet<String>,
    companion: BTreeSet<String>,
}

struct FeatureGroup {
    root: usize,
    common: BTreeSet<String>,
    joined: bool,
}

fn join_owned_features(
    files: &[Change],
    units: &[Unit],
    sets: &mut Sets,
) -> BTreeMap<usize, (String, bool)> {
    let owners: Vec<_> = files.iter().map(|file| owned_feature(&file.path)).collect();
    let known: BTreeSet<_> = owners.iter().copied().flatten().collect();
    let mut by_root = BTreeMap::new();
    for unit in units {
        for index in &unit.changes {
            let file = &files[*index];
            if file.kind == Kind::Docs && owners[*index].is_none() {
                continue;
            }
            let owner = if file.chunk.is_none()
                && (matches!(
                    file.kind,
                    Kind::Source | Kind::Test | Kind::Style | Kind::Asset | Kind::Docs
                ) || localized(&file.path))
            {
                entrypoint_owner(&file.path, &known)
                    .or(owners[*index])
                    .filter(|owner| {
                        file.old_path.as_deref().is_none_or(|old| {
                            entrypoint_owner(old, &known).or_else(|| owned_feature(old))
                                == Some(*owner)
                        })
                    })
            } else {
                None
            };
            let evidence = by_root
                .entry(sets.root(unit.id))
                .or_insert_with(|| FeatureEvidence {
                    owner,
                    ..Default::default()
                });
            if evidence.owner != owner {
                evidence.owner = None;
            }
            if evidence.owner.is_some() {
                evidence.terms.extend(diff_feature_terms(file));
                evidence.companion.extend(feature_terms(
                    basename(&file.path)
                        .rsplit_once('.')
                        .map_or(basename(&file.path), |(name, _)| name),
                ));
            }
        }
    }
    let mut by_module: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (root, mut evidence) in by_root {
        let Some(owner) = evidence.owner else {
            continue;
        };
        for word in feature_terms(basename(owner)) {
            evidence.terms.remove(&word);
            evidence.companion.remove(&word);
        }
        by_module.entry(owner).or_default().push((root, evidence));
    }
    let mut joined_modules = BTreeMap::new();
    for (owner, candidates) in by_module {
        let mut features: Vec<FeatureGroup> = Vec::new();
        let mut without_diff = Vec::new();
        for (root, evidence) in candidates {
            if evidence.terms.is_empty() {
                without_diff.push((root, evidence.companion));
                continue;
            }
            let mut matches = features
                .iter_mut()
                .filter(|feature| !feature.common.is_disjoint(&evidence.terms));
            if let (Some(feature), None) = (matches.next(), matches.next()) {
                // Keep a common term across all members; one mixed diff cannot bridge two features.
                feature.common.retain(|term| evidence.terms.contains(term));
                sets.join(feature.root, root);
                feature.joined = true;
            } else {
                features.push(FeatureGroup {
                    root,
                    common: evidence.terms,
                    joined: false,
                });
            }
        }
        let mut fallback = None;
        for (root, names) in without_diff {
            let matching = if features.len() == 1 {
                features.first_mut()
            } else {
                let mut matches = features
                    .iter_mut()
                    .filter(|feature| !feature.common.is_disjoint(&names));
                match (matches.next(), matches.next()) {
                    (Some(feature), None) => Some(feature),
                    _ => None,
                }
            };
            if let Some(feature) = matching {
                sets.join(feature.root, root);
                feature.joined = true;
            } else if let Some(first) = fallback {
                sets.join(first, root);
                joined_modules.insert(sets.root(first), (readable_word(basename(owner)), false));
            } else {
                fallback = Some(root);
            }
        }
        for feature in features.into_iter().filter(|feature| feature.joined) {
            let label = if feature.common.len() == 1 {
                feature.common.into_iter().next().unwrap()
            } else {
                readable_word(basename(owner))
            };
            joined_modules.insert(sets.root(feature.root), (label, true));
        }
    }
    joined_modules
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
    let mut joined_modules = join_owned_features(files, units, &mut sets);
    let mut grouped: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for unit in units {
        grouped.entry(sets.root(unit.id)).or_default().push(unit.id);
    }
    grouped.into_iter().map(|(id, units)| {
        let module = joined_modules.remove(&id);
        let feature = if let Some((label, _)) = &module {
            Some(label.clone())
        } else if units.len() > 1 {
            anchors.iter().find(|(id, _)| units.contains(id))
                .map(|(_, feature)| feature.name.replace('-', " "))
        } else {
            None
        };
        let reason = if units.len() > 1 {
            let mut joined = units.iter().flat_map(|id| unit_files(*id));
            if joined.any(|(_, file)| file.kind == Kind::Migration) {
                Some("Keep the migration with the implementation that references the same table.")
            } else if matches!(module, Some((_, true))) {
                Some("Keep changes with matching diff evidence together for review.")
            } else if module.is_some() {
                Some("Keep changes in the same feature module together for review.")
            } else {
                Some("Keep the matching implementation and companion changes together for review.")
            }
        } else {
            None
        };
        Proposal { units, reason, feature }
    }).collect()
}
