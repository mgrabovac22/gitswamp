use super::{evidence::symbol, extension};
use regex::bytes::{RegexSet, RegexSetBuilder};
use std::sync::OnceLock;

const MAX_SCOPES: usize = 12;
const MAX_DEPTH: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Signal {
    NullGuard,
    BoundsGuard,
    InputValidation,
    Authorization,
    ErrorHandling,
    Retry,
    Timeout,
    StateTransition,
    EventHandling,
    Routing,
    RequestHandling,
    Serialization,
    ResourceCleanup,
    Concurrency,
    Caching,
    Batching,
    MemoryHandling,
    FileIo,
    Rendering,
    TestAssertions,
}

const SIGNALS: [Signal; 20] = [
    Signal::NullGuard,
    Signal::BoundsGuard,
    Signal::InputValidation,
    Signal::Authorization,
    Signal::ErrorHandling,
    Signal::Retry,
    Signal::Timeout,
    Signal::StateTransition,
    Signal::EventHandling,
    Signal::Routing,
    Signal::RequestHandling,
    Signal::Serialization,
    Signal::ResourceCleanup,
    Signal::Concurrency,
    Signal::Caching,
    Signal::Batching,
    Signal::MemoryHandling,
    Signal::FileIo,
    Signal::Rendering,
    Signal::TestAssertions,
];

impl Signal {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::NullGuard => "null checks",
            Self::BoundsGuard => "bounds checks",
            Self::InputValidation => "input validation",
            Self::Authorization => "authorization checks",
            Self::ErrorHandling => "error handling",
            Self::Retry => "retry handling",
            Self::Timeout => "timeouts",
            Self::StateTransition => "state updates",
            Self::EventHandling => "event handlers",
            Self::Routing => "route registration",
            Self::RequestHandling => "request handling",
            Self::Serialization => "serialization",
            Self::ResourceCleanup => "resource cleanup",
            Self::Concurrency => "concurrency controls",
            Self::Caching => "caching",
            Self::Batching => "batch processing",
            Self::MemoryHandling => "memory allocation",
            Self::FileIo => "file I/O",
            Self::Rendering => "rendering",
            Self::TestAssertions => "test assertions",
        }
    }

    pub(super) fn family(self) -> u8 {
        match self {
            Self::NullGuard | Self::BoundsGuard | Self::InputValidation | Self::ErrorHandling => 1,
            Self::Authorization => 2,
            Self::Retry
            | Self::Timeout
            | Self::Routing
            | Self::RequestHandling
            | Self::Serialization => 3,
            Self::StateTransition | Self::EventHandling | Self::Rendering => 4,
            Self::ResourceCleanup | Self::Concurrency | Self::MemoryHandling | Self::FileIo => 5,
            Self::Caching | Self::Batching => 6,
            Self::TestAssertions => 7,
        }
    }
}

pub(in crate::services::commit_composer) fn supported(path: &str) -> bool {
    matches!(
        extension(path),
        "js" | "jsx"
            | "mjs"
            | "cjs"
            | "ts"
            | "tsx"
            | "mts"
            | "cts"
            | "py"
            | "pyi"
            | "cs"
            | "java"
            | "c"
            | "h"
            | "cc"
            | "cpp"
            | "cxx"
            | "hpp"
            | "hh"
            | "hxx"
    )
}

#[derive(Clone, Copy)]
pub(in crate::services::commit_composer) struct Line<'a> {
    pub(super) scope: &'a str,
    pub(super) signals: u32,
    signature: u32,
}

pub(in crate::services::commit_composer) struct Cursor<'a> {
    lines: std::slice::Split<'a, u8, fn(&u8) -> bool>,
    line: u32,
    ext: &'a str,
    frames: Vec<(usize, &'a str)>,
    pending: Option<&'a str>,
    depth: usize,
    comment: bool,
    triple: Option<u8>,
}

fn newline(byte: &u8) -> bool {
    *byte == b'\n'
}

impl<'a> Cursor<'a> {
    pub(in crate::services::commit_composer) fn new(path: &'a str, bytes: &'a [u8]) -> Self {
        Self {
            lines: bytes.split(newline),
            line: 0,
            ext: extension(path),
            frames: Vec::new(),
            pending: None,
            depth: 0,
            comment: false,
            triple: None,
        }
    }

    pub(in crate::services::commit_composer) fn line_at(&mut self, target: u32) -> Line<'a> {
        let mut result = Line {
            scope: "",
            signals: 0,
            signature: 0,
        };
        while self.line < target {
            let Some(raw) = self.lines.next() else { break };
            self.line += 1;
            let inspected = self.inspect(raw, self.line == target);
            if self.line == target {
                result = inspected;
            }
        }
        result
    }

    fn inspect(&mut self, raw: &'a [u8], changed: bool) -> Line<'a> {
        let python = matches!(self.ext, "py" | "pyi");
        let first = raw
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .unwrap_or(raw.len());
        let raw = &raw[first..];
        if raw.is_empty() {
            return Line {
                scope: "",
                signals: 0,
                signature: 0,
            };
        }
        let indent = first;
        let at_start = !self.comment && self.triple.is_none();
        let declaration = if at_start && raw.len() <= 512 {
            std::str::from_utf8(raw)
                .ok()
                .and_then(|text| declaration(text.trim_end(), self.ext))
        } else {
            None
        };
        if python && at_start && !raw.starts_with(b"#") {
            while self
                .frames
                .last()
                .is_some_and(|(depth, _)| *depth >= indent)
            {
                self.frames.pop();
            }
            if let Some(name) = declaration.filter(|_| self.frames.len() < MAX_DEPTH) {
                self.frames.push((indent, name));
            }
        }
        let mut scope = declaration
            .or(self.frames.last().map(|(_, name)| *name))
            .unwrap_or("");
        let mut code = [0u8; 512];
        let mut pos = 0;
        let mut opens = false;
        let mut terminated = false;
        let mut quote = None;
        let mut i = 0;
        while i < raw.len() {
            let b = raw[i];
            let pair = raw.get(i..i + 2);
            if self.comment {
                if pair == Some(b"*/") {
                    self.comment = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if let Some(delimiter) = self.triple {
                let width = if delimiter == b'`' { 1 } else { 3 };
                if delimiter == b'`' && b == b'\\' {
                    i += 2;
                } else if raw
                    .get(i..i + width)
                    .is_some_and(|chunk| chunk.iter().all(|byte| *byte == delimiter))
                {
                    self.triple = None;
                    i += width;
                } else {
                    i += 1;
                }
                continue;
            }
            if let Some(delimiter) = quote {
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == delimiter {
                    quote = None;
                }
                i += 1;
                continue;
            }
            if (!python && pair == Some(b"//")) || (python && b == b'#') {
                break;
            }
            if !python && pair == Some(b"/*") {
                self.comment = true;
                i += 2;
                if changed && pos < code.len() {
                    code[pos] = b' ';
                    pos += 1;
                }
                continue;
            }
            if matches!(b, b'"' | b'\'' | b'`') {
                if changed && pos < code.len() {
                    code[pos] = b' ';
                    pos += 1;
                }
                if b == b'`' && !python {
                    self.triple = Some(b);
                    i += 1;
                    continue;
                }
                if (python || matches!(self.ext, "java" | "cs"))
                    && raw.get(i..i + 3).is_some_and(|chunk| chunk == [b; 3])
                {
                    self.triple = Some(b);
                    i += 3;
                    continue;
                }
                quote = Some(b);
                i += 1;
                continue;
            }
            if !python {
                match b {
                    b'{' => {
                        self.depth += 1;
                        if !opens && self.frames.len() < MAX_DEPTH {
                            if let Some(name) = declaration.or(self.pending.take()) {
                                self.frames.push((self.depth, name));
                                scope = name;
                            }
                        }
                        opens = true;
                    }
                    b'}' => {
                        self.depth = self.depth.saturating_sub(1);
                        while self
                            .frames
                            .last()
                            .is_some_and(|(depth, _)| *depth > self.depth)
                        {
                            self.frames.pop();
                        }
                    }
                    b';' => terminated = true,
                    _ => {}
                }
            }
            if changed && pos < code.len() {
                code[pos] = b.to_ascii_lowercase();
                pos += 1;
            }
            i += 1;
        }
        if !python {
            if !opens && !terminated {
                self.pending = declaration.filter(|_| !raw.contains(&b'='));
            } else {
                self.pending = None;
            }
        }
        let signals = if changed && pos > 0 {
            classify(&code[..pos], self.ext)
        } else {
            0
        };
        let signature = if signals == 0 {
            0
        } else {
            code[..pos]
                .iter()
                .filter(|byte| !byte.is_ascii_whitespace())
                .fold(2166136261u32, |hash, byte| {
                    (hash ^ u32::from(*byte)).wrapping_mul(16777619)
                })
        };
        Line {
            scope,
            signals,
            signature,
        }
    }
}

fn declaration<'a>(line: &'a str, ext: &str) -> Option<&'a str> {
    if let Some(name) = symbol(line, ext) {
        let rest = line.split_once(name)?.1.trim_start();
        if !rest.starts_with('<')
            && !rest.starts_with('(')
            && !rest.starts_with('{')
            && !line.contains("=>")
            && !line.starts_with("class ")
            && !line.starts_with("public class ")
        {
            return None;
        }
        return Some(name);
    }
    if matches!(
        ext,
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts"
    ) && line.ends_with('{')
    {
        let (prefix, rest) = line.split_once('(')?;
        if !rest.contains(')') || prefix.contains(['=', '.', ':']) {
            return None;
        }
        let name = prefix.split_whitespace().last()?;
        return valid_callable(name).then_some(name);
    }
    None
}

fn valid_callable(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 80
        && name.as_bytes()[0].is_ascii_alphabetic()
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        && !matches!(
            name,
            "if" | "else" | "for" | "while" | "switch" | "catch" | "try" | "with" | "finally"
        )
}

fn classify(code: &[u8], ext: &str) -> u32 {
    let c_family = matches!(ext, "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx");
    let web = matches!(
        ext,
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts"
    );
    static RULES: OnceLock<RegexSet> = OnceLock::new();
    let rules = RULES.get_or_init(|| RegexSetBuilder::new([
        r"(?:\b(?:if|return)\b.*(?:\b(?:null|nullptr|undefined)\b|\bis\s+(?:not\s+)?none\b)|\?\?|\?\.)",
        r"(?:>=\s*(?:len\s*\(|sizeof\s*\(|limit\b|count\b|length\b|size\b|\w+\.length\b)|\bcheckindex\s*\(|\.isoutofbounds\s*\()",
        r"\b(?:validate\w*|isvalid|tryparse|isnan|isfinite|checkvalidity)\s*\(",
        r"(?:\b(?:haspermission|hasauthority|isuserinrole|canaccess|require_auth)\s*\(|\[authorize\b|@(?:preauthorize|secured|login_required)\b)",
        r"\b(?:catch\s*[(\{]|except[\s:]|throw\s+new\b|raise\s+|finally\s*[:\{]|perror\s*\()",
        r"(?:\b(?:retry|retrywhen|retrying|backoff)\s*\(|@retryable\b|\b(?:retry_policy|retry_count|max_retries)\b)",
        r"(?:\b(?:settimeout|wait_for|ortimeout|withtimeout|cancelafter)\s*\(|abortsignal\.timeout\s*\(|\btimeout\s*=\s*[^=])",
        r"(?:\b(?:setstate|dispatch|emitstate)\s*\(|\bstate\.(?:value|status)\s*=\s*[^=])",
        r"\b(?:addeventlistener|removeeventlistener|subscribe|unsubscribe|addlistener|removelistener)\s*\(",
        r"(?:\brouter\.(?:get|post|patch|put|delete)\s*\(|@(?:app\.(?:get|post)|router\.|getmapping|postmapping)|\[http(?:get|post)\b|\.map(?:get|post)\s*\()",
        r"(?:\b(?:fetch|sendasync|sendrequest|ensure_success_status|ensuresuccessstatuscode)\s*\(|\b(?:axios|requests|httpx|httpclient)\.)",
        r"(?:\bjson\.(?:parse|stringify|loads|dumps)\s*\(|\bjsonserializer\.|\.(?:readvalue|writevalueasstring|fromjson|tojson)\s*\()",
        r"(?:\.(?:dispose|disposeasync|release|close)\s*\(|\b(?:try\s*\(|using\s+var\b|with\s+open\s*\(|fclose\s*\(|free\s*\())",
        r"(?:\bstd::(?:mutex|atomic)|\b(?:lock_guard|unique_lock)\s*<|\bpthread_mutex_|\bsynchronized\b|\block\s*\(|\basyncio\.lock\s*\(|\binterlocked\.)",
        r"(?:\b(?:usememo|usecallback|memoize|getorcreate)\s*\(|@(?:lru_cache|cacheable|cache)\b|\bcache\.(?:get|set|put|invalidate)\s*\()",
        r"(?:\b(?:addbatch|executebatch|bulk_write|bulkcopy|executemany|chunks)\s*\(|\b(?:batch_size|batchsize)\b)",
        r"(?:\b(?:malloc|calloc|realloc|alloca|memcpy|memmove)\s*\(|\bstd::make_(?:unique|shared)\b)",
        r"(?:\b(?:fopen|fread|fwrite|read_all_text|readalltext|writealltext|readfilesync|writefilesync|read_text|write_text)\s*\(|\bstd::(?:ifstream|ofstream)|\bfiles\.(?:read|write))",
        r"\b(?:render|createelement|appendchild|setattribute|requestanimationframe|useeffect)\s*\(",
        r"(?:\bassert\s+|\b(?:assert|assert_eq|assertequals|asserttrue|assertfalse|expect)\s*\(|\bassert\.(?:equal|that|throws)\b|\bpytest\.raises\s*\()",
    ]).unicode(false).dfa_size_limit(128 * 1024).build().expect("static commit composer rules"));
    let mut mask = 0;
    for item in rules.matches(code).into_iter() {
        let signal = SIGNALS[item];
        if signal == Signal::MemoryHandling && !c_family || signal == Signal::Rendering && !web {
            continue;
        }
        mask |= 1 << item;
    }
    mask
}

#[derive(Default)]
pub(in crate::services::commit_composer) struct Summary(Vec<Scope>);

struct Scope {
    name: String,
    plus: [u8; SIGNALS.len()],
    minus: [u8; SIGNALS.len()],
    plus_signatures: [u32; SIGNALS.len()],
    minus_signatures: [u32; SIGNALS.len()],
}

impl Summary {
    pub(in crate::services::commit_composer) fn observe(&mut self, line: Line<'_>, added: bool) {
        if line.signals == 0 {
            return;
        }
        let scope = match self.0.iter().position(|entry| entry.name == line.scope) {
            Some(i) => &mut self.0[i],
            None if self.0.len() == MAX_SCOPES => return,
            None => {
                self.0.push(Scope {
                    name: line.scope.into(),
                    plus: [0; SIGNALS.len()],
                    minus: [0; SIGNALS.len()],
                    plus_signatures: [0; SIGNALS.len()],
                    minus_signatures: [0; SIGNALS.len()],
                });
                self.0.last_mut().unwrap()
            }
        };
        let (counters, signatures) = if added {
            (&mut scope.plus, &mut scope.plus_signatures)
        } else {
            (&mut scope.minus, &mut scope.minus_signatures)
        };
        let mut bits = line.signals;
        while bits != 0 {
            let i = bits.trailing_zeros() as usize;
            counters[i] = counters[i].saturating_add(1);
            signatures[i] = signatures[i].wrapping_add(line.signature);
            bits &= bits - 1;
        }
    }

    pub(super) fn changes(&self) -> impl Iterator<Item = Behavior<'_>> {
        self.0.iter().flat_map(|scope| {
            SIGNALS
                .into_iter()
                .enumerate()
                .filter_map(move |(i, signal)| {
                    let (plus, minus) = (scope.plus[i], scope.minus[i]);
                    let updated = plus != 0
                        && plus == minus
                        && scope.plus_signatures[i] != scope.minus_signatures[i];
                    (plus != minus || updated).then_some(Behavior {
                        scope: &scope.name,
                        signal,
                        added: plus > minus,
                        updated,
                        strength: plus.abs_diff(minus).max(1),
                    })
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_changed_bodies_in_priority_languages() {
        for (path, source, scope, signal) in [
            (
                "auth.ts",
                "function selectAccount() {\n  if (account == null) return;\n}\n",
                "selectAccount",
                Signal::NullGuard,
            ),
            (
                "view.js",
                "const initView = () => {\n  el.addEventListener('click', refresh);\n};\n",
                "initView",
                Signal::EventHandling,
            ),
            (
                "client.py",
                "async def load_account():\n    await asyncio.wait_for(fetch(), 10)\n",
                "load_account",
                Signal::Timeout,
            ),
            (
                "Storage.cs",
                "public Task LoadCache()\n{\n    return Cache.GetOrCreate(key, loader);\n}\n",
                "LoadCache",
                Signal::Caching,
            ),
            (
                "buffer.c",
                "int read_entry(int index) {\n    if (index >= count) return -1;\n}\n",
                "read_entry",
                Signal::BoundsGuard,
            ),
            (
                "buffer.cpp",
                "void resize_buffer() {\n    buffer = std::make_unique<char[]>(size);\n}\n",
                "resize_buffer",
                Signal::MemoryHandling,
            ),
            (
                "Client.java",
                "public void sendBatch() {\n    statement.executeBatch();\n}\n",
                "sendBatch",
                Signal::Batching,
            ),
        ] {
            let number = if path.ends_with(".cs") { 3 } else { 2 };
            let changed = Cursor::new(path, source.as_bytes()).line_at(number);
            assert_eq!(changed.scope, scope, "{path}");
            assert_ne!(changed.signals & (1 << signal as u32), 0, "{path}");
        }
    }

    #[test]
    fn ignores_comments_and_string_contents_and_cancels_balanced_signals() {
        let code = b"function check() {\n /*\n cache.get(value);\n */\n const sample = `\n cache.set(value);\n `;\n if (value == null) return;\n}";
        let mut cursor = Cursor::new("file.ts", code);
        assert_eq!(cursor.line_at(3).signals, 0);
        assert_eq!(cursor.line_at(6).signals, 0);
        let line = cursor.line_at(8);
        let mut summary = Summary::default();
        summary.observe(line, false);
        summary.observe(line, true);
        assert_eq!(summary.changes().count(), 0);
    }
}

#[derive(Clone, Copy)]
pub(super) struct Behavior<'a> {
    pub(super) scope: &'a str,
    pub(super) signal: Signal,
    pub(super) added: bool,
    pub(super) updated: bool,
    pub(super) strength: u8,
}
