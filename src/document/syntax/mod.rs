//! Syntect syntax highlighting, reduced to a neutral contract.
//!
//! Ported from Comet (https://github.com/zeronsh/comet), MIT.
//! Original: `crates/syntax/src/lib.rs`.
//!
//! The contract is deliberately **colour-free**: a [`HighlightSpan`] carries a
//! [`HighlightKind`], never an `Hsla`. Colour is applied later from the theme,
//! so switching appearance recolours existing spans instead of reparsing, and a
//! JS-supplied palette can override every token without touching this module.
//!
//! Syntect scope stacks are mapped to [`HighlightKind`] here. Themes stay out
//! of this module so a palette change never reparses.
//!
//! Ranges are byte offsets relative to one UTF-8 source **line**, which is what
//! per-line rendering needs and what makes a code block's height exact before
//! any highlighting has run.

pub mod cache;

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;
use std::sync::OnceLock;

use syntect::easy::ScopeRangeIterator;
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

/// Sources larger than this are rendered plain. Highlighting a megabyte of
/// minified JS blocks the frame for longer than anyone will tolerate.
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 512 * 1024;
pub const DEFAULT_MAX_SPANS: usize = 200_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightLimits {
    pub max_source_bytes: usize,
    pub max_spans: usize,
}

impl Default for HighlightLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_spans: DEFAULT_MAX_SPANS,
        }
    }
}

/// Languages resolved against Syntect's default syntax set. Detection still
/// uses this closed list so unknown fences stay plain text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    JavaScript,
    Jsx,
    TypeScript,
    Tsx,
    Python,
    Go,
    Json,
    Jsonc,
    Bash,
    Toml,
    Markdown,
    Html,
    Css,
    Yaml,
    C,
}

/// A capture class. One per theme colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HighlightKind {
    Comment,
    Keyword,
    String,
    StringSpecial,
    Escape,
    Number,
    Boolean,
    Type,
    TypeBuiltin,
    Constructor,
    Function,
    FunctionBuiltin,
    Macro,
    Property,
    Constant,
    Variable,
    VariableSpecial,
    Parameter,
    Operator,
    Punctuation,
    Tag,
    Attribute,
    Label,
    Embedded,
    Invalid,
}

impl HighlightKind {
    /// Stable precedence used to resolve overlapping parser captures.
    /// Without it, nested captures resolve by iteration order and a string
    /// inside a macro flickers between two colours across parses.
    pub const fn precedence(self) -> u8 {
        match self {
            Self::Invalid => 100,
            Self::Escape => 95,
            Self::Macro => 90,
            Self::Property | Self::Attribute => 85,
            Self::FunctionBuiltin | Self::TypeBuiltin | Self::VariableSpecial => 80,
            Self::StringSpecial | Self::Constructor | Self::Parameter => 75,
            Self::Function | Self::Type | Self::Constant | Self::Tag | Self::Label => 70,
            Self::Comment | Self::Keyword | Self::String | Self::Number | Self::Boolean => 60,
            Self::Variable | Self::Operator => 50,
            Self::Punctuation | Self::Embedded => 40,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub range: Range<usize>,
    pub kind: HighlightKind,
}

/// Highlight spans grouped per source line, sorted and non-overlapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightedDocument {
    pub language: LanguageId,
    pub lines: Vec<Vec<HighlightSpan>>,
}

#[derive(Debug, Clone, Copy)]
pub struct HighlightRequest<'a> {
    pub source: &'a str,
    /// File path, used for extension-based detection.
    pub path: Option<&'a str>,
    /// Markdown fence tag such as `ts`, which beats the path.
    pub fence_tag: Option<&'a str>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HighlightError {
    #[error("the source language is not registered")]
    UnknownLanguage,
    #[error("highlight range {start}..{end} is invalid for a {len}-byte source")]
    InvalidRange {
        start: usize,
        end: usize,
        len: usize,
    },
    #[error("highlight range {start}..{end} is not on UTF-8 boundaries")]
    InvalidUtf8Boundary { start: usize, end: usize },
    #[error("source exceeds the configured highlighting limit")]
    SourceTooLarge,
    #[error("highlight output exceeds the configured span limit")]
    TooManySpans,
    #[error("parser failed: {0}")]
    Parser(String),
}

impl HighlightedDocument {
    /// Validate, split and normalize absolute source spans into line-relative
    /// spans. A span that crosses a newline becomes one span per line.
    pub fn from_absolute_spans(
        language: LanguageId,
        source: &str,
        spans: impl IntoIterator<Item = HighlightSpan>,
    ) -> Result<Self, HighlightError> {
        let starts = line_starts(source);
        let mut lines = vec![Vec::new(); starts.len()];
        for span in spans {
            validate_span(source, &span.range)?;
            if span.range.is_empty() {
                continue;
            }
            let first_line = starts.partition_point(|&start| start <= span.range.start) - 1;
            for (line_ix, &start) in starts.iter().enumerate().skip(first_line) {
                let raw_end = starts.get(line_ix + 1).copied().unwrap_or(source.len());
                let mut end = raw_end;
                if source.as_bytes().get(end.wrapping_sub(1)) == Some(&b'\n') {
                    end -= 1;
                    if source.as_bytes().get(end.wrapping_sub(1)) == Some(&b'\r') {
                        end -= 1;
                    }
                }
                let segment_start = span.range.start.max(start);
                let segment_end = span.range.end.min(end);
                if segment_start < segment_end {
                    lines[line_ix].push(HighlightSpan {
                        range: segment_start - start..segment_end - start,
                        kind: span.kind,
                    });
                }
                if raw_end >= span.range.end {
                    break;
                }
            }
        }
        for line in &mut lines {
            *line = normalize_line(std::mem::take(line));
        }
        Ok(Self { language, lines })
    }
}

fn validate_span(source: &str, range: &Range<usize>) -> Result<(), HighlightError> {
    if range.start > range.end || range.end > source.len() {
        return Err(HighlightError::InvalidRange {
            start: range.start,
            end: range.end,
            len: source.len(),
        });
    }
    if !source.is_char_boundary(range.start) || !source.is_char_boundary(range.end) {
        return Err(HighlightError::InvalidUtf8Boundary {
            start: range.start,
            end: range.end,
        });
    }
    Ok(())
}

/// Flatten overlapping spans on one line into a sorted, non-overlapping run,
/// resolving each byte to the highest-precedence kind covering it.
fn normalize_line(spans: Vec<HighlightSpan>) -> Vec<HighlightSpan> {
    #[derive(Clone, Copy)]
    enum Edge {
        Start(usize),
        End(usize),
    }

    let mut edges = spans
        .iter()
        .enumerate()
        .flat_map(|(index, span)| {
            [
                (span.range.start, Edge::Start(index)),
                (span.range.end, Edge::End(index)),
            ]
        })
        .collect::<Vec<_>>();
    edges.sort_unstable_by_key(|(offset, _)| *offset);

    // The span index is the tie-breaker, so equal-precedence overlaps let the
    // later span win — matching the parser's own nesting order.
    let mut active = BTreeSet::new();
    let mut normalized: Vec<HighlightSpan> = Vec::new();
    let mut cursor = 0;
    while cursor < edges.len() {
        let offset = edges[cursor].0;
        let group_start = cursor;
        while cursor < edges.len() && edges[cursor].0 == offset {
            if let Edge::End(index) = edges[cursor].1 {
                active.remove(&(spans[index].kind.precedence(), index));
            }
            cursor += 1;
        }
        for (_, edge) in &edges[group_start..cursor] {
            if let Edge::Start(index) = *edge {
                active.insert((spans[index].kind.precedence(), index));
            }
        }

        let Some(next_offset) = edges.get(cursor).map(|(next, _)| *next) else {
            break;
        };
        if offset == next_offset {
            continue;
        }
        if let Some((_, index)) = active.last().copied() {
            let kind = spans[index].kind;
            let merged = match normalized.last_mut() {
                Some(previous) if previous.kind == kind && previous.range.end == offset => {
                    previous.range.end = next_offset;
                    true
                }
                _ => false,
            };
            if !merged {
                normalized.push(HighlightSpan {
                    range: offset..next_offset,
                    kind,
                });
            }
        }
    }
    normalized
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        source
            .match_indices('\n')
            .map(|(index, _)| index + 1)
            .filter(|start| *start < source.len()),
    );
    starts
}

/// Highlight a document with the default limits.
pub fn highlight(request: HighlightRequest<'_>) -> Result<HighlightedDocument, HighlightError> {
    highlight_with_limits(request, HighlightLimits::default())
}

pub fn highlight_with_limits(
    request: HighlightRequest<'_>,
    limits: HighlightLimits,
) -> Result<HighlightedDocument, HighlightError> {
    if request.source.len() > limits.max_source_bytes {
        return Err(HighlightError::SourceTooLarge);
    }
    let language = detect_language(
        request.path,
        request.fence_tag,
        request.source.lines().next(),
    )
    .ok_or(HighlightError::UnknownLanguage)?;

    let syntax = syntax_for_language(language)?;
    let set = syntax_set();
    let mut parse_state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut spans = Vec::new();
    let mut offset = 0;

    if !request.source.is_empty() {
        for line in request.source.split_inclusive('\n') {
            let ops = parse_state
                .parse_line(line, set)
                .map_err(|error| HighlightError::Parser(error.to_string()))?;
            for (range, op) in ScopeRangeIterator::new(&ops, line) {
                stack
                    .apply(op)
                    .map_err(|error| HighlightError::Parser(error.to_string()))?;
                if range.is_empty() {
                    continue;
                }
                let Some(kind) = kind_for_region(language, &line[range.clone()], &stack) else {
                    continue;
                };
                spans.push(HighlightSpan {
                    range: offset + range.start..offset + range.end,
                    kind,
                });
                if spans.len() > limits.max_spans {
                    return Err(HighlightError::TooManySpans);
                }
            }
            offset += line.len();
        }
    }

    HighlightedDocument::from_absolute_spans(language, request.source, spans)
}

fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    // Default syntect dump has no TypeScript, TSX, or TOML. two-face is bat's
    // extra syntax pack.
    //
    // Loading this dump costs ~3ms. The expensive part is that syntect compiles
    // a grammar's TextMate regexes lazily, on the FIRST highlight of that
    // language, on the frame thread, inside a paint. With fancy-regex that was
    // ~133ms for TypeScript and ~17ms for Rust, which read as a slow mount and
    // as one dropped frame when a scroll first revealed a code block. Cargo.toml
    // therefore picks regex-onig off wasm (~12ms and ~1.6ms) and keeps
    // regex-fancy for wasm, where onig_sys cannot compile its C sources because
    // wasm32-unknown-unknown has no libc. Do not "simplify" that split back into
    // one dependency, and do not retry onig on wasm.
    SET.get_or_init(two_face::syntax::extra_newlines)
}

fn syntax_for_language(language: LanguageId) -> Result<&'static SyntaxReference, HighlightError> {
    let set = syntax_set();
    let (names, extensions): (&[&str], &[&str]) = match language {
        LanguageId::Rust => (&["Rust"], &["rs"]),
        LanguageId::JavaScript => (&["JavaScript"], &["js"]),
        // two-face fancy excludes JavaScript (Babel), so JSX uses the TSX grammar.
        LanguageId::Jsx => (&["TypeScriptReact", "JavaScript"], &["jsx", "tsx"]),
        LanguageId::TypeScript => (&["TypeScript"], &["ts"]),
        LanguageId::Tsx => (&["TypeScriptReact"], &["tsx"]),
        LanguageId::Python => (&["Python"], &["py"]),
        LanguageId::Go => (&["Go"], &["go"]),
        LanguageId::Json => (&["JSON"], &["json"]),
        LanguageId::Jsonc => (&["JSON with Comments", "JSONC", "JSON"], &["jsonc", "json"]),
        LanguageId::Bash => (&["Bourne Again Shell (bash)", "Bash"], &["bash", "sh"]),
        LanguageId::Toml => (&["TOML"], &["toml"]),
        LanguageId::Markdown => (&["Markdown"], &["md"]),
        LanguageId::Html => (&["HTML"], &["html"]),
        LanguageId::Css => (&["CSS"], &["css"]),
        LanguageId::Yaml => (&["YAML"], &["yaml", "yml"]),
        LanguageId::C => (&["C"], &["c"]),
    };
    names
        .iter()
        .find_map(|name| set.find_syntax_by_name(name))
        .or_else(|| {
            extensions
                .iter()
                .find_map(|ext| set.find_syntax_by_extension(ext))
        })
        .ok_or_else(|| HighlightError::Parser(format!("no syntect grammar for {language:?}")))
}

fn kind_for_stack(stack: &ScopeStack) -> Option<HighlightKind> {
    let mut best = None;
    for scope in stack.as_slice() {
        let Some(kind) = kind_for_scope_name(&scope.build_string()) else {
            continue;
        };
        if best.is_none_or(|prev: HighlightKind| kind.precedence() >= prev.precedence()) {
            best = Some(kind);
        }
    }
    best
}

fn stack_has_prefix(stack: &ScopeStack, prefix: &str) -> bool {
    stack
        .as_slice()
        .iter()
        .any(|scope| scope_matches(&scope.build_string(), prefix))
}

fn is_boolean_literal(text: &str) -> bool {
    matches!(
        text.trim(),
        "true" | "false" | "True" | "False" | "TRUE" | "FALSE" | "yes" | "no" | "on" | "off"
    )
}

fn is_builtin_type(language: LanguageId, text: &str) -> bool {
    let text = text.trim();
    match language {
        LanguageId::C => matches!(
            text,
            "int"
                | "void"
                | "char"
                | "unsigned"
                | "signed"
                | "long"
                | "short"
                | "float"
                | "double"
                | "_Bool"
                | "bool"
                | "size_t"
                | "ssize_t"
                | "int8_t"
                | "int16_t"
                | "int32_t"
                | "int64_t"
                | "uint8_t"
                | "uint16_t"
                | "uint32_t"
                | "uint64_t"
                | "uintptr_t"
                | "intptr_t"
                | "ptrdiff_t"
        ),
        LanguageId::Rust => matches!(
            text,
            "i8" | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
                | "bool"
                | "char"
                | "str"
                | "Self"
        ),
        LanguageId::TypeScript | LanguageId::Tsx | LanguageId::JavaScript | LanguageId::Jsx => {
            matches!(
                text,
                "string"
                    | "number"
                    | "boolean"
                    | "bigint"
                    | "symbol"
                    | "object"
                    | "undefined"
                    | "never"
                    | "unknown"
                    | "any"
                    | "void"
            )
        }
        _ => false,
    }
}

fn kind_for_region(language: LanguageId, text: &str, stack: &ScopeStack) -> Option<HighlightKind> {
    let kind = kind_for_stack(stack)?;
    if stack_has_prefix(stack, "constant.language") {
        return Some(if is_boolean_literal(text) {
            HighlightKind::Boolean
        } else {
            HighlightKind::Constant
        });
    }
    if stack_has_prefix(stack, "storage.type") {
        return Some(if is_builtin_type(language, text) {
            match language {
                LanguageId::C => HighlightKind::Type,
                _ => HighlightKind::TypeBuiltin,
            }
        } else {
            HighlightKind::Keyword
        });
    }
    Some(kind)
}

/// Longest prefix first. `string` must not steal `string.regexp`.
const SCOPE_PREFIXES: &[(&str, HighlightKind)] = &[
    ("entity.other.attribute-name", HighlightKind::Attribute),
    ("support.type.property-name", HighlightKind::Property),
    ("constant.language.undefined", HighlightKind::Constant),
    ("entity.name.function.macro", HighlightKind::Macro),
    ("constant.character.escape", HighlightKind::Escape),
    ("constant.language.boolean", HighlightKind::Boolean),
    ("entity.name.constructor", HighlightKind::Constructor),
    ("meta.object-literal.key", HighlightKind::Property),
    ("support.function.builtin", HighlightKind::FunctionBuiltin),
    ("support.function.macro", HighlightKind::Macro),
    ("variable.other.property", HighlightKind::Property),
    ("variable.other.constant", HighlightKind::Constant),
    ("constant.language.none", HighlightKind::Constant),
    ("constant.language.null", HighlightKind::Constant),
    ("support.class.builtin", HighlightKind::TypeBuiltin),
    ("entity.name.interface", HighlightKind::Type),
    ("entity.name.namespace", HighlightKind::Type),
    ("entity.name.function", HighlightKind::Function),
    ("variable.other.member", HighlightKind::Property),
    ("constant.language.nil", HighlightKind::Constant),
    ("string.interpolated", HighlightKind::StringSpecial),
    ("constant.character", HighlightKind::Escape),
    ("entity.name.section", HighlightKind::Label),
    ("variable.parameter", HighlightKind::Parameter),
    ("constant.language", HighlightKind::Boolean),
    ("entity.name.struct", HighlightKind::Type),
    ("variable.language", HighlightKind::VariableSpecial),
    ("constant.numeric", HighlightKind::Number),
    ("entity.name.class", HighlightKind::Type),
    ("entity.name.label", HighlightKind::Label),
    ("entity.name.macro", HighlightKind::Macro),
    ("entity.name.trait", HighlightKind::Type),
    ("keyword.operator", HighlightKind::Operator),
    ("meta.mapping.key", HighlightKind::Property),
    ("storage.modifier", HighlightKind::Keyword),
    ("string.quoted.other", HighlightKind::StringSpecial),
    ("support.constant", HighlightKind::Constant),
    ("support.function", HighlightKind::FunctionBuiltin),
    ("constant.other", HighlightKind::Constant),
    ("entity.name.enum", HighlightKind::Type),
    ("entity.name.tag", HighlightKind::Tag),
    ("entity.name.type", HighlightKind::Type),
    ("keyword.control", HighlightKind::Keyword),
    ("string.unquoted", HighlightKind::StringSpecial),
    ("meta.embedded", HighlightKind::Embedded),
    ("string.escape", HighlightKind::Escape),
    ("string.regexp", HighlightKind::StringSpecial),
    ("support.class", HighlightKind::TypeBuiltin),
    ("support.macro", HighlightKind::Macro),
    ("storage.type", HighlightKind::Keyword),
    ("support.type", HighlightKind::TypeBuiltin),
    ("punctuation", HighlightKind::Punctuation),
    ("constructor", HighlightKind::Constructor),
    ("variable", HighlightKind::Variable),
    ("function", HighlightKind::Function),
    ("operator", HighlightKind::Operator),
    ("property", HighlightKind::Property),
    ("constant", HighlightKind::Constant),
    ("keyword", HighlightKind::Keyword),
    ("storage", HighlightKind::Keyword),
    ("boolean", HighlightKind::Boolean),
    ("comment", HighlightKind::Comment),
    ("invalid", HighlightKind::Invalid),
    ("string", HighlightKind::String),
    ("markup", HighlightKind::Embedded),
    ("number", HighlightKind::Number),
    ("label", HighlightKind::Label),
    ("type", HighlightKind::Type),
];

fn scope_matches(name: &str, prefix: &str) -> bool {
    name == prefix
        || (name.len() > prefix.len()
            && name.starts_with(prefix)
            && name.as_bytes()[prefix.len()] == b'.')
}

/// Map a TextMate scope name onto the nearest [`HighlightKind`].
/// Language prefixes such as `source.rust` are ignored.
pub fn kind_for_scope_name(name: &str) -> Option<HighlightKind> {
    SCOPE_PREFIXES
        .iter()
        .find(|(prefix, _)| scope_matches(name, prefix))
        .map(|(_, kind)| *kind)
}

/// Fence tag beats path, path beats shebang.
pub fn detect_language(
    path: Option<&str>,
    fence_tag: Option<&str>,
    first_line: Option<&str>,
) -> Option<LanguageId> {
    fence_tag
        .and_then(language_for_alias)
        .or_else(|| path.and_then(language_for_path))
        .or_else(|| first_line.and_then(language_for_shebang))
}

pub fn language_for_alias(alias: &str) -> Option<LanguageId> {
    let alias = alias
        .trim()
        .split_ascii_whitespace()
        .next()?
        .to_ascii_lowercase();
    Some(match alias.as_str() {
        "rust" | "rs" => LanguageId::Rust,
        "javascript" | "js" | "mjs" | "cjs" => LanguageId::JavaScript,
        "jsx" => LanguageId::Jsx,
        "typescript" | "ts" | "mts" | "cts" => LanguageId::TypeScript,
        "tsx" => LanguageId::Tsx,
        "python" | "py" | "python3" => LanguageId::Python,
        "go" | "golang" => LanguageId::Go,
        "json" => LanguageId::Json,
        "jsonc" => LanguageId::Jsonc,
        "bash" | "sh" | "shell" | "zsh" | "console" => LanguageId::Bash,
        "toml" => LanguageId::Toml,
        "markdown" | "md" => LanguageId::Markdown,
        "html" | "htm" => LanguageId::Html,
        "css" => LanguageId::Css,
        "yaml" | "yml" => LanguageId::Yaml,
        "c" | "h" => LanguageId::C,
        _ => return None,
    })
}

pub fn language_for_path(path: &str) -> Option<LanguageId> {
    let path = Path::new(path);
    let name = path.file_name()?.to_str()?;
    match name.to_ascii_lowercase().as_str() {
        "cargo.lock" | "cargo.toml" | "pyproject.toml" => return Some(LanguageId::Toml),
        _ => {}
    }
    language_for_alias(path.extension()?.to_str()?)
}

fn language_for_shebang(line: &str) -> Option<LanguageId> {
    let line = line.strip_prefix("#!")?.to_ascii_lowercase();
    if line.contains("python") {
        Some(LanguageId::Python)
    } else if line.contains("node") {
        Some(LanguageId::JavaScript)
    } else if ["bash", "zsh", "/sh", " sh"]
        .iter()
        .any(|name| line.contains(name))
    {
        Some(LanguageId::Bash)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_keep_language_variants_distinct() {
        let cases = [
            ("js", LanguageId::JavaScript),
            ("jsx", LanguageId::Jsx),
            ("ts", LanguageId::TypeScript),
            ("tsx", LanguageId::Tsx),
            ("RS", LanguageId::Rust),
            ("shell", LanguageId::Bash),
        ];
        for (alias, expected) in cases {
            assert_eq!(language_for_alias(alias), Some(expected), "{alias}");
        }
        assert_eq!(language_for_alias("unknown-lang"), None);
    }

    #[test]
    fn paths_and_exact_names_resolve() {
        let cases = [
            ("src/main.rs", LanguageId::Rust),
            ("web/app.tsx", LanguageId::Tsx),
            ("Cargo.toml", LanguageId::Toml),
            ("config.jsonc", LanguageId::Jsonc),
        ];
        for (path, expected) in cases {
            assert_eq!(language_for_path(path), Some(expected), "{path}");
        }
        assert_eq!(language_for_path("README"), None);
        assert_eq!(language_for_path("image.png"), None);
    }

    #[test]
    fn fence_tag_beats_path() {
        assert_eq!(
            detect_language(Some("a.rs"), Some("python"), None),
            Some(LanguageId::Python)
        );
    }

    #[test]
    fn shebang_is_the_last_resort() {
        assert_eq!(
            detect_language(None, None, Some("#!/usr/bin/env python3")),
            Some(LanguageId::Python)
        );
        assert_eq!(detect_language(None, None, Some("let x = 1")), None);
    }

    #[test]
    fn spans_are_line_relative_sorted_and_non_overlapping() {
        let source = "let café = \"x\";\nnext";
        let document = highlight(HighlightRequest {
            source,
            path: None,
            fence_tag: Some("rust"),
        })
        .unwrap();
        assert_eq!(document.lines.len(), 2);
        for line in &document.lines {
            let mut previous_end = 0;
            for span in line {
                assert!(span.range.start >= previous_end, "spans must not overlap");
                previous_end = span.range.end;
            }
        }
        // Line 0 is 16 bytes ("café" is 5), so nothing may reach past it and
        // in particular no span may swallow the newline.
        assert!(document.lines[0].iter().all(|s| s.range.end <= 16));
    }

    #[test]
    fn language_constants_split_booleans_from_other_literals() {
        assert_eq!(
            kind_of("true null", "json", "true"),
            Some(HighlightKind::Boolean)
        );
        assert_eq!(
            kind_of("true null", "json", "null"),
            Some(HighlightKind::Constant)
        );
        assert_eq!(
            kind_of("True None", "python", "True"),
            Some(HighlightKind::Boolean)
        );
        assert_eq!(
            kind_of("True None", "python", "None"),
            Some(HighlightKind::Constant)
        );
        assert_eq!(
            kind_of("true NaN Infinity", "js", "true"),
            Some(HighlightKind::Boolean)
        );
        assert_eq!(
            kind_of("true NaN Infinity", "js", "NaN"),
            Some(HighlightKind::Constant)
        );
        assert_eq!(
            kind_of("true NaN Infinity", "js", "Infinity"),
            Some(HighlightKind::Constant)
        );
    }

    #[test]
    fn storage_type_keeps_keywords_and_builtin_types_apart() {
        assert_eq!(
            kind_of("let x: u32 = 1;", "rust", "let"),
            Some(HighlightKind::Keyword)
        );
        assert_eq!(
            kind_of("let x: u32 = 1;", "rust", "u32"),
            Some(HighlightKind::TypeBuiltin)
        );
        assert_eq!(
            kind_of("int main() { return 0; }", "c", "int"),
            Some(HighlightKind::Type)
        );
    }

    #[test]
    fn rust_numbers_and_booleans_are_their_own_kinds() {
        let document = highlight(HighlightRequest {
            source: "let a = 42; let b = true;",
            path: Some("x.rs"),
            fence_tag: None,
        })
        .unwrap();
        let kinds: Vec<_> = document.lines[0].iter().map(|s| s.kind).collect();
        assert!(kinds.contains(&HighlightKind::Number), "{kinds:?}");
        assert!(kinds.contains(&HighlightKind::Boolean), "{kinds:?}");
        assert!(kinds.contains(&HighlightKind::Keyword), "{kinds:?}");
    }

    #[test]
    fn typescript_highlights_keywords_and_strings() {
        let document = highlight(HighlightRequest {
            source: "const greeting: string = \"hi\"",
            path: None,
            fence_tag: Some("ts"),
        })
        .unwrap();
        let kinds: Vec<_> = document.lines[0].iter().map(|s| s.kind).collect();
        assert!(kinds.contains(&HighlightKind::Keyword), "{kinds:?}");
        assert!(kinds.contains(&HighlightKind::String), "{kinds:?}");
    }

    #[test]
    fn unknown_language_is_an_error_not_a_panic() {
        let result = highlight(HighlightRequest {
            source: "hello",
            path: Some("notes.txt"),
            fence_tag: None,
        });
        assert_eq!(result, Err(HighlightError::UnknownLanguage));
    }

    #[test]
    fn oversized_sources_are_rejected() {
        let big = "a".repeat(64);
        let result = highlight_with_limits(
            HighlightRequest {
                source: &big,
                path: Some("a.rs"),
                fence_tag: None,
            },
            HighlightLimits {
                max_source_bytes: 8,
                max_spans: 10,
            },
        );
        assert_eq!(result, Err(HighlightError::SourceTooLarge));
    }

    #[test]
    fn crlf_line_endings_do_not_leak_into_spans() {
        let document = highlight(HighlightRequest {
            source: "// one\r\n// two\r\n",
            path: Some("a.rs"),
            fence_tag: None,
        })
        .unwrap();
        assert_eq!(document.lines[0][0].range, 0..6);
        assert_eq!(document.lines[1][0].range, 0..6);
    }

    #[test]
    fn empty_source_yields_one_empty_line() {
        let document = highlight(HighlightRequest {
            source: "",
            path: Some("a.rs"),
            fence_tag: None,
        })
        .unwrap();
        assert_eq!(document.lines.len(), 1);
        assert!(document.lines[0].is_empty());
    }

    #[test]
    fn scope_names_map_to_neutral_kinds() {
        let cases = [
            ("comment.line.double-slash.rust", HighlightKind::Comment),
            ("keyword.control.rust", HighlightKind::Keyword),
            ("storage.type.rust", HighlightKind::Keyword),
            ("storage.modifier.rust", HighlightKind::Keyword),
            ("string.quoted.double.rust", HighlightKind::String),
            ("string.regexp.js", HighlightKind::StringSpecial),
            ("constant.character.escape.rust", HighlightKind::Escape),
            (
                "constant.numeric.integer.decimal.rust",
                HighlightKind::Number,
            ),
            ("constant.language.boolean.rust", HighlightKind::Boolean),
            ("constant.language.rust", HighlightKind::Boolean),
            ("entity.name.type.rust", HighlightKind::Type),
            ("support.type.python", HighlightKind::TypeBuiltin),
            ("support.class.builtin.python", HighlightKind::TypeBuiltin),
            ("entity.name.function.rust", HighlightKind::Function),
            (
                "support.function.builtin.python",
                HighlightKind::FunctionBuiltin,
            ),
            ("entity.name.macro.rust", HighlightKind::Macro),
            ("meta.mapping.key.json", HighlightKind::Property),
            ("variable.other.member.rust", HighlightKind::Property),
            ("variable.other.constant.rust", HighlightKind::Constant),
            ("variable.language.rust", HighlightKind::VariableSpecial),
            ("variable.parameter.rust", HighlightKind::Parameter),
            ("variable.other.rust", HighlightKind::Variable),
            ("keyword.operator.rust", HighlightKind::Operator),
            (
                "punctuation.definition.string.rust",
                HighlightKind::Punctuation,
            ),
            ("entity.name.tag.html", HighlightKind::Tag),
            ("entity.other.attribute-name.html", HighlightKind::Attribute),
            ("entity.name.label.rust", HighlightKind::Label),
            ("meta.embedded.block.html", HighlightKind::Embedded),
            ("invalid.illegal.rust", HighlightKind::Invalid),
        ];
        for (scope, expected) in cases {
            assert_eq!(kind_for_scope_name(scope), Some(expected), "{scope}");
        }
        assert_eq!(kind_for_scope_name("source.rust"), None);
        assert_eq!(kind_for_scope_name("text.html.basic"), None);
    }

    #[test]
    fn scope_prefixes_list_more_specific_first() {
        for (index, (specific, _)) in SCOPE_PREFIXES.iter().enumerate() {
            for (general, _) in &SCOPE_PREFIXES[index + 1..] {
                assert!(
                    !scope_matches(general, specific),
                    "{specific} is a prefix of later {general}"
                );
            }
        }
    }

    #[test]
    fn more_specific_scopes_win_over_generic_prefixes() {
        assert_eq!(
            kind_for_scope_name("constant.numeric.float.rust"),
            Some(HighlightKind::Number)
        );
        assert_eq!(
            kind_for_scope_name("constant.other.rust"),
            Some(HighlightKind::Constant)
        );
        assert_eq!(
            kind_for_scope_name("string.unquoted.yaml"),
            Some(HighlightKind::StringSpecial)
        );
    }

    fn first_line_kinds(source: &str, fence_tag: &str) -> Vec<HighlightKind> {
        let document = highlight(HighlightRequest {
            source,
            path: None,
            fence_tag: Some(fence_tag),
        })
        .unwrap();
        document.lines[0].iter().map(|span| span.kind).collect()
    }

    fn kind_of(source: &str, fence_tag: &str, token: &str) -> Option<HighlightKind> {
        let document = highlight(HighlightRequest {
            source,
            path: None,
            fence_tag: Some(fence_tag),
        })
        .unwrap();
        source
            .split('\n')
            .zip(document.lines.iter())
            .find_map(|(line, spans)| {
                spans.iter().find_map(|span| {
                    (line.get(span.range.clone()) == Some(token)).then_some(span.kind)
                })
            })
    }

    #[test]
    fn rust_function_names_are_functions() {
        let kinds = first_line_kinds("fn greet() {}", "rust");
        assert!(kinds.contains(&HighlightKind::Function), "{kinds:?}");
        assert!(kinds.contains(&HighlightKind::Keyword), "{kinds:?}");
    }

    #[test]
    fn html_highlights_tags_and_attributes() {
        let kinds = first_line_kinds("<div class=\"box\">", "html");
        assert!(kinds.contains(&HighlightKind::Tag), "{kinds:?}");
        assert!(kinds.contains(&HighlightKind::Attribute), "{kinds:?}");
        assert!(kinds.contains(&HighlightKind::String), "{kinds:?}");
    }

    #[test]
    fn jsx_highlights_tags() {
        let kinds = first_line_kinds("const el = <Button label=\"ok\" />;", "jsx");
        assert!(kinds.contains(&HighlightKind::Keyword), "{kinds:?}");
        assert!(kinds.contains(&HighlightKind::Attribute), "{kinds:?}");
        assert!(kinds.contains(&HighlightKind::String), "{kinds:?}");
    }

    #[test]
    fn python_comments_and_strings() {
        let kinds = first_line_kinds("# hi\nx = \"ok\"", "python");
        assert!(kinds.contains(&HighlightKind::Comment), "{kinds:?}");
        let document = highlight(HighlightRequest {
            source: "# hi\nx = \"ok\"",
            path: None,
            fence_tag: Some("python"),
        })
        .unwrap();
        let line1: Vec<_> = document.lines[1].iter().map(|span| span.kind).collect();
        assert!(line1.contains(&HighlightKind::String), "{line1:?}");
    }

    #[test]
    fn json_keys_and_numbers_are_highlighted() {
        let kinds = first_line_kinds("{\"name\": 1}", "json");
        assert!(
            kinds.contains(&HighlightKind::Property) || kinds.contains(&HighlightKind::String),
            "{kinds:?}"
        );
        assert!(kinds.contains(&HighlightKind::Number), "{kinds:?}");
    }

    #[test]
    fn css_properties_are_highlighted() {
        let kinds = first_line_kinds("body { color: red; }", "css");
        assert!(kinds.contains(&HighlightKind::Property), "{kinds:?}");
    }

    #[test]
    fn markdown_fence_tag_highlights_the_fenced_language() {
        let document = highlight(HighlightRequest {
            source: "fn main() {}",
            path: None,
            fence_tag: Some("rust"),
        })
        .unwrap();
        assert_eq!(document.language, LanguageId::Rust);
        let kinds: Vec<_> = document.lines[0].iter().map(|span| span.kind).collect();
        assert!(kinds.contains(&HighlightKind::Keyword), "{kinds:?}");
    }

    #[test]
    fn every_registered_language_highlights() {
        let samples = [
            ("rust", "fn main() { let x = 1; }"),
            ("js", "const x = 'hi';"),
            ("jsx", "const el = <div />;"),
            ("ts", "const x: string = 'hi';"),
            ("tsx", "const el = <div />;"),
            ("python", "def f():\n    return True"),
            ("go", "func main() { x := 1 }"),
            ("json", "{\"a\": 1}"),
            ("jsonc", "{\"a\": 1}"),
            ("bash", "echo hi"),
            ("toml", "name = \"x\""),
            ("markdown", "# Title"),
            ("html", "<p>hi</p>"),
            ("css", "a { color: red; }"),
            ("yaml", "name: x"),
            ("c", "int main() { return 0; }"),
        ];
        for (tag, source) in samples {
            let document = highlight(HighlightRequest {
                source,
                path: None,
                fence_tag: Some(tag),
            })
            .expect(tag);
            assert!(
                !document.lines.is_empty(),
                "{tag} must produce at least one line"
            );
        }
    }

    #[test]
    fn too_many_spans_are_rejected() {
        let result = highlight_with_limits(
            HighlightRequest {
                source: "let a = 1; let b = 2; let c = 3;",
                path: Some("a.rs"),
                fence_tag: None,
            },
            HighlightLimits {
                max_source_bytes: 10_000,
                max_spans: 1,
            },
        );
        assert_eq!(result, Err(HighlightError::TooManySpans));
    }
}
