use super::{
    area, basename, behavior_specific, component, covers, directory, generic_stem, main_behavior,
    primary, readable_word, Action, Behavior, Change, Group, Hints, Kind, Signal, Topic,
};

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

fn readable(path: &str) -> String {
    let name = basename(path);
    readable_word(name.rsplit_once('.').map_or(name, |(a, _)| a))
}

fn file_target(file: &Change) -> String {
    let path = &file.path;
    let name = readable(path);
    let parent = basename(directory(path));
    match (basename(path), parent) {
        ("mod.rs", parent) if !parent.is_empty() => {
            format!("{} module declarations", readable_word(parent))
        }
        (_, "commands") => format!("{name} command handlers"),
        (_, "models") => format!("{name} data model"),
        (_, "repositories") => format!("{name} Git operations"),
        (_, "services") => format!("{name} service logic"),
        _ => name,
    }
}

fn named_target(name: &str, path: &str, scope: &str) -> String {
    let symbol = readable_word(name);
    if symbol.contains(' ') {
        return symbol;
    }
    let source = readable(path);
    let scope = readable_word(scope);
    let context = [source, scope].into_iter().find(|candidate| {
        !candidate.is_empty()
            && !generic_stem(candidate)
            && !candidate.split_whitespace().any(|word| word == symbol)
            && !matches!(candidate.as_str(), "repo" | "core" | "src tauri")
    });
    match context {
        Some(context) => format!("{symbol} declaration in {context}"),
        None => symbol,
    }
}

fn useful_facts(file: &Change) -> impl Iterator<Item = (Topic, Action, &str)> {
    file.hints
        .evidence
        .facts()
        .filter(|(topic, _, name)| *topic != Topic::Symbol || !generic_stem(name))
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

fn subject_behavior<'a>(kind: Kind, files: &[&'a Change]) -> Option<Behavior<'a>> {
    files
        .iter()
        .filter(|file| file.kind == kind && file.status == "modified")
        .filter_map(|file| main_behavior(file))
        .filter(|change| !change.scope.is_empty() || behavior_specific(change.signal))
        .max_by_key(|change| {
            (
                behavior_specific(change.signal),
                !change.scope.is_empty(),
                change.strength,
            )
        })
}

fn precise_fact(topic: Topic) -> bool {
    matches!(
        topic,
        Topic::Column
            | Topic::Operation
            | Topic::Endpoint
            | Topic::Message
            | Topic::Field
            | Topic::Signal
            | Topic::Command
            | Topic::Constraint
    )
}

fn behavior_sentence(change: Behavior<'_>) -> String {
    let what = change.signal.label();
    let action = if change.updated {
        "Updates"
    } else if change.added {
        "Adds"
    } else {
        "Removes"
    };
    if change.scope.is_empty() {
        format!("{action} {what}.")
    } else {
        format!("{action} {what} in `{}`.", change.scope)
    }
}

fn label_kind(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Test => Some("tests or test fixtures"),
        Kind::Docs => Some("documentation"),
        Kind::Build | Kind::Lock => Some("build or dependency files"),
        Kind::Config => Some("configuration"),
        Kind::Schema | Kind::Migration => Some("schema or migration files"),
        Kind::Ci => Some("CI workflows"),
        Kind::Style => Some("styles"),
        _ => None,
    }
}

fn fact_sentence(topic: Topic, action: Action, name: &str) -> String {
    let verb = match action {
        Action::Add => "Adds",
        Action::Update => "Updates",
        Action::Remove => "Removes",
    };
    match topic {
        Topic::Symbol => format!("{verb} the `{name}` declaration."),
        Topic::Setting => format!("{verb} the `{name}` configuration key."),
        Topic::Section => format!("{verb} the \"{name}\" section."),
        Topic::Route => format!("{verb} a {} route.", name.to_ascii_uppercase()),
        _ => format!("{verb} the {} `{name}`.", topic.label()),
    }
}

pub(super) fn draft(files: &[&Change], unit_ids: Vec<usize>, feature: Option<&str>) -> Group {
    let kind = primary(files);
    let anchor = files.iter().find(|file| file.kind == kind).unwrap();
    let (mut prefix, mut verb) = title(kind, files);
    let scope = scope_for(files, kind);
    let anchor_target = readable(&anchor.path);
    let scope_label = readable_word(&scope);
    let fact = subject_fact(kind, files);
    let representative = subject_behavior(kind, files);
    let lead_behavior = representative.filter(|change| {
        !fact.is_some_and(|(topic, _, _)| topic == Topic::Route || precise_fact(topic))
            && (!matches!(fact, Some((Topic::Symbol, Action::Add, _))) || !change.added)
    });
    let body = files
        .iter()
        .filter(|file| file.kind == kind && file.status == "modified")
        .flat_map(|file| file.hints.changed_bodies())
        .filter(|body| !generic_stem(&body.name))
        .max_by_key(|body| body.lines);
    let target = if anchor.status == "renamed" && files.len() == 1 {
        verb = "Rename";
        format!(
            "{} to {anchor_target}",
            anchor.old_path.as_deref().map(readable).unwrap_or_default()
        )
    } else if let Some(change) = lead_behavior {
        verb = if change.updated {
            "Update"
        } else if change.added {
            "Add"
        } else {
            "Remove"
        };
        if kind == Kind::Source {
            prefix = match change.signal {
                Signal::NullGuard | Signal::BoundsGuard if change.added => "Fix",
                Signal::Routing | Signal::EventHandling if change.added => "Feat",
                _ => "Refactor",
            };
        }
        let name = if change.scope.is_empty() {
            anchor_target.clone()
        } else {
            let routine = readable_word(change.scope);
            if ["build", "run", "apply", "plan", "verify", "load", "save"]
                .contains(&routine.as_str())
                && !anchor_target.split_whitespace().any(|word| word == routine)
                && !generic_stem(&anchor_target)
            {
                format!("{anchor_target} {routine}")
            } else {
                routine
            }
        };
        format!("{} in {name}", change.signal.label())
    } else if let Some((topic, action, name)) = fact {
        verb = match (topic, action) {
            (
                Topic::Table | Topic::Index | Topic::View | Topic::Trigger | Topic::Policy,
                Action::Add,
            ) => "Create",
            (
                Topic::Table | Topic::Index | Topic::View | Topic::Trigger | Topic::Policy,
                Action::Remove,
            ) => "Drop",
            (Topic::Section, _) => "Document",
            (Topic::Endpoint, Action::Add) => "Expose",
            (Topic::Command, Action::Add) => "Define",
            (_, Action::Add) => "Add",
            (_, Action::Update) => "Update",
            (_, Action::Remove) => "Remove",
        };
        if kind == Kind::Source && prefix == "Feat" && action != Action::Add {
            prefix = "Refactor";
        }
        match topic {
            Topic::Symbol => {
                let path = files
                    .iter()
                    .find(|file| {
                        file.kind == kind
                            && useful_facts(file).any(|item| item == (topic, action, name))
                    })
                    .map_or(anchor.path.as_str(), |file| file.path.as_str());
                named_target(name, path, &scope)
            }
            Topic::Section => readable_word(name),
            Topic::Route => format!("{} route in {anchor_target}", name.to_ascii_uppercase()),
            _ => format!("{} {name}", topic.label()),
        }
    } else if let Some(chunk) = &anchor.chunk {
        verb = "Update";
        readable_word(&chunk.scope)
    } else if let Some(body) = body {
        verb = "Revise";
        readable_word(&body.name)
    } else if let Some(feature) = feature {
        format!("{feature} implementation")
    } else if files.len() > 1 && !matches!(scope.as_str(), "core" | "repo") {
        scope_label.clone()
    } else {
        file_target(anchor)
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
    let mut described = Vec::new();
    if let Some(change) = lead_behavior {
        described.push((change.signal, change.scope, change.added, change.updated));
        description.push_str(&behavior_sentence(change));
    } else if let Some((topic, action, name)) = fact {
        description.push_str(&fact_sentence(topic, action, name));
    } else if let Some(body) = body {
        description.push_str(&format!("Changes the implementation in `{}`.", body.name));
    } else if files.len() == 1 && anchor.status == "renamed" {
        description.push_str(&format!(
            "Renames `{}` to `{}`.",
            anchor.old_path.as_deref().unwrap_or(&anchor.path),
            anchor.path
        ));
    }
    if let Some(change) = lead_behavior {
        use Signal::*;
        let reason = match change.signal {
            NullGuard if change.added => Some("This makes the missing-value path explicit before downstream code can use that value."),
            BoundsGuard if change.added => Some("This handles out-of-range input before the affected operation proceeds."),
            InputValidation if change.added => Some("This gives the affected input an explicit validation step."),
            Authorization if change.added => Some("This puts an explicit permission check on the affected path."),
            ResourceCleanup if change.added => Some("This gives the affected resource an explicit cleanup step."),
            Timeout if change.updated => Some("This changes how long the affected operation is allowed to wait."),
            DatabaseRead if change.added => Some("The affected path now reads data through a database query."),
            DatabaseWrite if change.added => Some("The affected path now includes an explicit database write step."),
            Transaction if change.added => Some("This places the affected database work under explicit transactional control."),
            Authentication if change.added => Some("The affected path now checks the caller's identity."),
            PasswordHandling if change.added => Some("This adds an explicit password-handling step to the authentication path."),
            TokenHandling if change.added => Some("This makes token processing explicit on the affected path."),
            SessionHandling if change.added => Some("This adds an explicit step to the session lifecycle."),
            BusinessRules if change.added => Some("The affected operation now evaluates an additional business rule."),
            Pagination if change.added => Some("This introduces page-based or cursor-based handling for the affected results."),
            DataFiltering if change.added => Some("The affected path now narrows results using an explicit filter."),
            DataOrdering if change.added => Some("The affected path now applies an explicit sort order."),
            FeatureFlags if change.added => Some("The affected behavior now consults a feature flag."),
            RateLimiting if change.added => Some("The affected operation now consults a rate limiter."),
            JobDispatch if change.added => Some("The affected path now hands work off to a background job."),
            DataRedaction if change.added => Some("Adds an explicit step to remove or mask sensitive data."),
            Idempotency if change.added => Some("The affected operation now has an explicit check for repeat execution."),
            Lifecycle if change.added => Some("Registers work to run at a lifecycle boundary."),
            Routing if change.added => Some("This exposes an additional route through the application's routing layer."),
            EventHandling if change.added => Some("This connects the affected component to its event source."),
            _ => None,
        };
        if let Some(reason) = reason {
            description.push(' ');
            description.push_str(reason);
        }
    }
    let mut details = 0;
    for file in files {
        for change in file
            .hints
            .behavior
            .changes()
            .filter(|change| !change.scope.is_empty() || behavior_specific(change.signal))
        {
            if details >= 3 {
                break;
            }
            let key = (change.signal, change.scope, change.added, change.updated);
            if described.contains(&key) {
                continue;
            }
            described.push(key);
            if !description.is_empty() {
                description.push(' ');
            }
            description.push_str(&behavior_sentence(change));
            details += 1;
        }
    }
    let mut structural = Vec::new();
    for (topic, action, name) in files.iter().flat_map(|file| useful_facts(file)) {
        if structural.len() == 2 {
            break;
        }
        let key = (topic, action, name);
        if Some(key) == fact && lead_behavior.is_none()
            || structural.contains(&key)
            || !precise_fact(topic)
        {
            continue;
        }
        structural.push(key);
        if !description.is_empty() {
            description.push(' ');
        }
        description.push_str(&fact_sentence(topic, action, name));
    }
    let mut supporting = Vec::new();
    for file in files.iter().filter(|file| file.kind != kind) {
        if let Some(label) = label_kind(file.kind) {
            if !supporting.contains(&label) {
                supporting.push(label);
            }
        }
    }
    if !supporting.is_empty() && !description.is_empty() {
        description.push_str(&format!(
            " The commit also includes {}.",
            supporting.join(", ")
        ));
    }
    if description.is_empty() {
        let target = if let Some(feature) = feature {
            feature
        } else if anchor_target.is_empty() {
            scope_label.as_str()
        } else {
            anchor_target.as_str()
        };
        let activity = match kind {
            Kind::Source => "implementation",
            Kind::Test => "test coverage or fixtures",
            Kind::Docs => "documentation",
            Kind::Style => "styles",
            Kind::Build | Kind::Lock => "build and dependency definitions",
            Kind::Config => "configuration",
            Kind::Ci => "CI workflow definitions",
            Kind::Schema | Kind::Migration => "data definitions",
            Kind::Git => "repository settings",
            Kind::Asset => "assets",
            Kind::Generated => "generated artifacts",
            Kind::Other => "repository files",
        };
        let action = match anchor.status {
            "added" => "Adds",
            "deleted" => "Removes",
            _ => "Updates",
        };
        description.push_str(&format!("{action} {activity} for {target}."));
    }
    let mut description = wrap_paragraph(&description, "");
    description.push_str("\n\nChanges by file:\n");
    let omitted = append_file_details(&mut description, files, kind);
    if omitted != 0 {
        description.push('\n');
        description.push_str(&wrap_paragraph(
            &format!(
                "The file list omits {omitted} additional change {}.",
                if omitted == 1 { "entry" } else { "entries" }
            ),
            "",
        ));
        description.push('\n');
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
        description.push_str(&format!(
            "\n{} changes: {}.",
            files.len(),
            counts.join(", ")
        ));
    }
    Group {
        subject,
        description: description.trim_end().into(),
        units: unit_ids,
    }
}

// Leave room for the grouping rationale, the change tally, and issue trailers.
const FILE_DETAILS_LIMIT: usize = 7400;

fn append_file_details(description: &mut String, files: &[&Change], primary: Kind) -> usize {
    let mut ordered = files.to_vec();
    ordered.sort_by_key(|file| (file.kind != primary, useful_facts(file).next().is_none()));
    let mut omitted = 0;
    for (position, file) in ordered.into_iter().enumerate() {
        let remaining = FILE_DETAILS_LIMIT.saturating_sub(description.len());
        if remaining == 0 {
            return omitted + files.len() - position;
        }
        let path = &file.path;
        let intro = match (file.old_path.as_deref(), file.status) {
            (Some(old), _) => format!("Renames `{old}` to `{path}`."),
            (_, "added") => format!("Adds `{path}`."),
            (_, "deleted") => format!("Removes `{path}`."),
            (_, "typechange") => format!("Replaces `{path}` with a different file type."),
            _ => format!("Updates `{path}`."),
        };
        let mut prose = intro;
        let mut entry = wrap_paragraph(&prose, "- ");
        if entry.len() + 1 > remaining {
            omitted += 1;
            continue;
        }
        let detail_space = remaining / (files.len() - position);
        for sentence in file_details(file) {
            let before = prose.len();
            prose.push(' ');
            prose.push_str(&sentence);
            let wrapped = wrap_paragraph(&prose, "- ");
            if wrapped.len() < detail_space {
                entry = wrapped;
            } else {
                prose.truncate(before);
            }
        }
        description.push_str(&entry);
        description.push('\n');
    }
    omitted
}

fn file_details(file: &Change) -> Vec<String> {
    let mut sentences = Vec::new();

    if let Some(chunk) = &file.chunk {
        let (side, start, count) = if chunk.new_lines == 0 {
            ("previous", chunk.old_start, chunk.old_lines)
        } else {
            ("new", chunk.new_start, chunk.new_lines)
        };
        let end = start.saturating_add(count.saturating_sub(1));
        sentences.push(if start == end {
            format!(
                "Changes `{}` at line {start} of the {side} version.",
                chunk.scope
            )
        } else {
            format!(
                "Changes `{}` at lines {start}-{end} of the {side} version.",
                chunk.scope
            )
        });
    }
    if file.partial {
        sentences
            .push("Includes only staged edits; unstaged edits remain in the working tree.".into());
    }
    if let (Some((old_id, old_mode)), Some((new_id, new_mode))) = (file.old, file.new) {
        if old_mode != new_mode
            && [old_mode, new_mode]
                .iter()
                .all(|mode| matches!(mode, 0o100644 | 0o100755))
        {
            sentences.push(
                if new_mode == 0o100755 {
                    "Makes the file executable."
                } else {
                    "Clears the executable bit."
                }
                .into(),
            );
        }
        if old_id != new_id && new_mode == 0o160000 {
            sentences.push(format!(
                "Moves the submodule pointer from {} to {}.",
                &old_id.to_string()[..7],
                &new_id.to_string()[..7]
            ));
        }
    }
    if let Some(behavior) = main_behavior(file)
        .filter(|change| !change.scope.is_empty() || behavior_specific(change.signal))
    {
        sentences.push(behavior_sentence(behavior));
    }

    let mut facts: Vec<_> = useful_facts(file).collect();
    facts.sort_by_key(|(topic, _, _)| !precise_fact(*topic));
    for (topic, action, name) in facts.into_iter().take(5) {
        sentences.push(fact_sentence(topic, action, name));
    }
    if sentences.is_empty() && file.status == "modified" {
        let names: Vec<_> = file
            .hints
            .changed_bodies()
            .filter(|body| !generic_stem(&body.name))
            .take(3)
            .map(|body| format!("`{}`", body.name))
            .collect();
        if !names.is_empty() {
            sentences.push(format!(
                "Changes the implementation in {}.",
                names.join(", ")
            ));
        }
    }
    if !file.kind.inspect_text() && file.old.map(|v| v.0) != file.new.map(|v| v.0) {
        let detail = match (file.kind, file.status) {
            (Kind::Lock, "added") => "Records the initial resolved dependency versions.",
            (Kind::Lock, "deleted") => "Removes the record of resolved dependency versions.",
            (Kind::Lock, _) => "Updates the recorded dependency resolution.",
            (Kind::Generated, "added") => "Includes new generated output.",
            (Kind::Generated, "deleted") => "Removes generated output.",
            (Kind::Generated, _) => "Refreshes generated output.",
            (Kind::Asset, "added" | "deleted") => "",
            (Kind::Asset, _) => "Changes the asset contents.",
            _ => unreachable!(),
        };
        if !detail.is_empty() {
            sentences.push(detail.into());
        }
    }
    if sentences.is_empty() && (file.added > 0 || file.removed > 0) {
        let count = |lines| if lines == 1 { "line" } else { "lines" };
        let summary = match (file.added, file.removed) {
            (added, 0) => format!("Adds {added} {}.", count(added)),
            (0, removed) => format!("Removes {removed} {}.", count(removed)),
            (added, removed) => format!(
                "Adds {added} {} and removes {removed} {}.",
                count(added),
                count(removed)
            ),
        };
        sentences.push(summary);
    }
    sentences
}

fn wrap_paragraph(text: &str, prefix: &str) -> String {
    let mut result = prefix.to_owned();
    let indent = prefix.len();
    let mut column = indent;
    for word in text.split_whitespace() {
        let width = word.chars().count();
        if column > indent {
            if column + 1 + width > 72 {
                result.push('\n');
                result.push_str(&" ".repeat(indent));
                column = indent;
            } else {
                result.push(' ');
                column += 1;
            }
        }
        result.push_str(word);
        column += width;
    }
    result
}

pub(super) fn explain_group(mut group: Group, reason: &str) -> Group {
    let paragraph = wrap_paragraph(reason, "");
    if let Some(split) = group.description.find("\n\n") {
        group
            .description
            .insert_str(split, &format!("\n\n{paragraph}"));
    } else {
        group.description.push_str(&format!("\n\n{paragraph}"));
    }
    group
}
