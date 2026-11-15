use super::{basename, extension, identifier, key_name, Kind, Topic};

pub(super) fn identify<'a>(path: &str, kind: Kind, line: &'a str) -> Option<(Topic, &'a str)> {
    let name = basename(path);
    let ext = extension(name);
    let mut tokens = line.split_ascii_whitespace();
    let first = tokens.next()?;
    if kind == Kind::Build {
        if name.eq_ignore_ascii_case("CMakeLists.txt") || ext == "cmake" {
            let (command, args) = line.split_once('(')?;
            if [
                "add_library",
                "add_executable",
                "add_custom_target",
                "target_link_libraries",
                "target_sources",
                "target_include_directories",
                "target_compile_definitions",
            ]
            .iter()
            .any(|entry| command.trim_end().eq_ignore_ascii_case(entry))
            {
                return bare(
                    args.split(|c: char| c.is_whitespace() || c == ')')
                        .find(|part| !part.is_empty())?,
                )
                .map(|name| (Topic::Target, name));
            }
        }
        if name.eq_ignore_ascii_case("Justfile") {
            return bare(line.split_once(':')?.0).map(|name| (Topic::Target, name));
        }
    }
    if kind == Kind::Style {
        if matches!(first, "@keyframes" | "@-webkit-keyframes") {
            return bare(tokens.next()?).map(|name| (Topic::Symbol, name));
        }
        if let Some((name, _)) = line.split_once(':') {
            return bare(name.strip_prefix("--")?).map(|_| (Topic::Setting, name));
        }
    }
    if kind == Kind::Config && ext == "bicep" && matches!(first, "resource" | "module") {
        return bare(tokens.next()?).map(|name| (Topic::Resource, name));
    }
    if !matches!(
        kind,
        Kind::Source | Kind::Schema | Kind::Test | Kind::Migration
    ) {
        return None;
    }
    match ext {
        "graphql" | "gql" => match first {
            "query" | "mutation" | "subscription" => {
                identifier(tokens.next()?).map(|name| (Topic::Operation, name))
            }
            "extend" => identifier(tokens.nth(1)?).map(|name| (Topic::Symbol, name)),
            "type" | "input" | "interface" | "enum" | "scalar" | "union" | "directive"
            | "schema" | "fragment" => None,
            _ => {
                // Accept a typed field, not a selection or a value in an operation.
                let (field, ty) = line.split_once(':')?;
                let ty = ty.trim_start().trim_start_matches('[');
                if !ty.starts_with(|c: char| c.is_ascii_uppercase()) {
                    return None;
                }
                bare(field.trim_end()).map(|name| (Topic::Field, name))
            }
        },
        "proto" => match first {
            "rpc" => identifier(tokens.next()?).map(|name| (Topic::Endpoint, name)),
            "message" => identifier(tokens.next()?).map(|name| (Topic::Message, name)),
            "option" | "syntax" | "package" | "import" | "reserved" | "extensions" => None,
            _ => {
                let (decl, tag) = line.split_once('=')?;
                if !tag.trim_start().starts_with(|c: char| c.is_ascii_digit()) {
                    return None;
                }
                let (ty, name) = decl.trim_end().rsplit_once(char::is_whitespace)?;
                if ty.trim().is_empty() {
                    return None;
                }
                bare(name).map(|name| (Topic::Field, name))
            }
        },
        "prisma" if matches!(first, "model" | "enum" | "view") => {
            bare(tokens.next()?).map(|name| (Topic::Symbol, name))
        }
        "vhd" | "vhdl" => {
            if matches_ignore_case(first, &["signal", "variable", "constant", "component"]) {
                identifier(tokens.next()?).map(|name| (Topic::Signal, name))
            } else if matches_ignore_case(
                first,
                &["entity", "architecture", "function", "procedure"],
            ) {
                identifier(tokens.next()?).map(|name| (Topic::Symbol, name))
            } else {
                None
            }
        }
        "v" | "sv" | "vh" | "svh" => {
            if matches!(first, "module" | "interface" | "program") {
                return identifier(tokens.next()?).map(|name| (Topic::Symbol, name));
            }
            if !matches!(
                first,
                "input" | "output" | "inout" | "wire" | "reg" | "logic"
            ) {
                return None;
            }
            let decl = line.split(['=', ',', ';', ')']).next()?.trim_end();
            let (_, tail) = decl.split_once(char::is_whitespace)?;
            bare(tail.split_ascii_whitespace().last()?).map(|name| (Topic::Signal, name))
        }
        "ps1" | "psm1" | "psd1" if matches_ignore_case(first, &["function", "filter"]) => {
            bare(tokens.next()?).map(|name| (Topic::Command, name))
        }
        "tcl" | "exp" if first == "proc" => bare(tokens.next()?).map(|name| (Topic::Command, name)),
        "pl" | "pm" | "raku" if matches!(first, "sub" | "method") => {
            identifier(tokens.next()?).map(|name| (Topic::Symbol, name))
        }
        "erl" if first.contains('(') && line.ends_with("->") => {
            bare(line.split_once('(')?.0.trim_end()).map(|name| (Topic::Symbol, name))
        }
        "abap" if matches_ignore_case(first, &["method", "form", "function"]) => {
            identifier(tokens.next()?).map(|name| (Topic::Symbol, name))
        }
        "zig" | "nim" | "d" | "odin" if matches!(first, "proc" | "func") => {
            identifier(tokens.next()?).map(|name| (Topic::Symbol, name))
        }
        "odin"
            if line
                .split_once("::")
                .is_some_and(|(_, tail)| tail.trim_start().starts_with("proc(")) =>
        {
            bare(first).map(|name| (Topic::Symbol, name))
        }
        "vb" | "bas" | "cls" | "pas" | "pp" | "ada" | "adb" | "ads" | "f90" | "f95" | "f03" => {
            let mut words = std::iter::once(first).chain(tokens);
            let declaration = words.find(|word| {
                !matches_ignore_case(
                    word,
                    &[
                        "public",
                        "private",
                        "protected",
                        "friend",
                        "shared",
                        "async",
                        "pure",
                        "recursive",
                    ],
                )
            })?;
            if !matches_ignore_case(
                declaration,
                &["function", "sub", "subroutine", "procedure", "module"],
            ) {
                return None;
            }
            identifier(words.next()?).map(|name| (Topic::Symbol, name))
        }
        "elm" | "hs" | "purs"
            if line
                .split_once(':')
                .is_some_and(|(name, _)| bare(name.trim_end()).is_some()) =>
        {
            bare(line.split_once(':')?.0.trim_end()).map(|name| (Topic::Symbol, name))
        }
        _ => None,
    }
}

fn bare(name: &str) -> Option<&str> {
    key_name(name).filter(|parsed| *parsed == name && !name.contains(['/', '.', '@']))
}

fn matches_ignore_case(word: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| word.eq_ignore_ascii_case(candidate))
}
