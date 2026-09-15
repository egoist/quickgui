//! Bounded Tree-sitter syntax coloring shared by the editor, diff view, and code block.
//!
//! Grammar queries produce paint-only foreground spans: highlighting never changes text metrics.
//! Configurations are initialized once, parsers are reused per thread, and callers only invoke
//! this module when their source, language, or syntax theme changes.

#[cfg(feature = "bundled-languages")]
use std::sync::OnceLock;
use std::{cell::RefCell, ops::Range, path::Path, sync::Arc};

use tree_sitter_highlight::{
    HighlightConfiguration, HighlightEvent, Highlighter as TreeSitterHighlighter,
};

use crate::{Color, HighlightStyle, MAX_TEXT_HIGHLIGHTS, StyledText};

#[path = "syntax/registry.rs"]
mod registry;
pub use registry::{
    MAX_LANGUAGE_QUERY_BYTES, MAX_REGISTERED_SYNTAX_LANGUAGES, RegisteredSyntaxLanguage,
    SyntaxLanguageDefinition, SyntaxLanguageError, register_syntax_language,
    syntax_language_generation,
};
#[cfg(all(feature = "language-packs", not(target_arch = "wasm32")))]
#[path = "syntax/language_pack.rs"]
mod language_pack;
#[cfg(all(feature = "language-packs", not(target_arch = "wasm32")))]
pub use language_pack::{
    MAX_LANGUAGE_PACK_BYTES, load_syntax_language_pack, load_syntax_language_pack_bytes,
};

/// Maximum source bytes parsed by the built-in Tree-sitter highlighter.
pub const MAX_SYNTAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
/// A pathological generated line is left plain after this many bytes.
pub const MAX_SYNTAX_LINE_BYTES: usize = 32 * 1024;
/// Maximum paint spans emitted by the built-in syntax highlighter.
pub const MAX_SYNTAX_HIGHLIGHTS: usize = MAX_TEXT_HIGHLIGHTS;

/// Plain text, registered languages, and well-known language identifiers.
/// No grammars are included by the default `editor` feature. Load or register a grammar first;
/// the optional `bundled-languages` feature explicitly opts into the built-in grammar set.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum SyntaxLanguage {
    #[default]
    PlainText,
    Rust,
    JavaScript,
    TypeScript,
    Tsx,
    Python,
    Go,
    Cpp,
    Java,
    Ruby,
    Swift,
    Json,
    Yaml,
    Toml,
    Shell,
    Css,
    Html,
    Sql,
    Markdown,
    Registered(RegisteredSyntaxLanguage),
}

impl SyntaxLanguage {
    /// Resolve a language name, common fence tag, or filename extension.
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim().trim_start_matches('.').to_ascii_lowercase();
        registry::snapshot()
            .by_name(&name)
            .or_else(|| builtin_from_name(&name))
    }

    /// Canonical name, including independently registered languages.
    pub fn name(self) -> String {
        match self {
            Self::Registered(id) => registry::snapshot()
                .language_name(id)
                .unwrap_or("text")
                .to_owned(),
            _ => format!("{self:?}")
                .to_ascii_lowercase()
                .replace("plaintext", "text"),
        }
    }

    /// Infer a language from a path without accessing the filesystem.
    pub fn from_path(path: impl AsRef<Path>) -> Option<Self> {
        let path = path.as_ref();
        if let Some(name) = path.file_name().and_then(|name| name.to_str())
            && let Some(language) = registry::snapshot().by_filename(&name.to_ascii_lowercase())
        {
            return Some(language);
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            match name.to_ascii_lowercase().as_str() {
                "gemfile" | "rakefile" => {
                    return Self::from_name(name);
                }
                _ => {}
            }
        }
        path.extension()
            .and_then(|extension| extension.to_str())
            .and_then(Self::from_name)
    }
}

#[cfg(feature = "bundled-languages")]
fn builtin_from_name(name: &str) -> Option<SyntaxLanguage> {
    Some(match name {
        "text" | "txt" | "plaintext" => SyntaxLanguage::PlainText,
        "rust" | "rs" => SyntaxLanguage::Rust,
        "javascript" | "js" | "jsx" | "mjs" | "cjs" => SyntaxLanguage::JavaScript,
        "typescript" | "ts" | "mts" | "cts" => SyntaxLanguage::TypeScript,
        "tsx" => SyntaxLanguage::Tsx,
        "python" | "py" | "py3" => SyntaxLanguage::Python,
        "go" | "golang" => SyntaxLanguage::Go,
        "cc" | "cpp" | "c++" | "cxx" | "hpp" => SyntaxLanguage::Cpp,
        "java" => SyntaxLanguage::Java,
        "ruby" | "rb" | "rake" | "gemfile" | "rakefile" => SyntaxLanguage::Ruby,
        "swift" => SyntaxLanguage::Swift,
        "json" => SyntaxLanguage::Json,
        "yaml" | "yml" => SyntaxLanguage::Yaml,
        "toml" => SyntaxLanguage::Toml,
        "shell" | "shellscript" | "sh" | "bash" => SyntaxLanguage::Shell,
        "css" => SyntaxLanguage::Css,
        "html" | "htm" => SyntaxLanguage::Html,
        "sql" | "postgres" | "postgresql" | "mysql" | "sqlite" => SyntaxLanguage::Sql,
        "markdown" | "md" | "mdown" => SyntaxLanguage::Markdown,
        _ => return None,
    })
}

#[cfg(not(feature = "bundled-languages"))]
fn builtin_from_name(name: &str) -> Option<SyntaxLanguage> {
    matches!(name, "text" | "txt" | "plaintext").then_some(SyntaxLanguage::PlainText)
}

/// Foreground palette used by Tree-sitter capture classes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SyntaxTheme {
    pub keyword: Option<Color>,
    pub literal: Option<Color>,
    pub string: Option<Color>,
    pub comment: Option<Color>,
    pub number: Option<Color>,
    pub r#type: Option<Color>,
    pub function: Option<Color>,
    pub metadata: Option<Color>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SyntaxTokenKind {
    Keyword,
    Literal,
    String,
    Comment,
    Number,
    Type,
    Function,
    Metadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SyntaxSpan {
    pub(crate) range: Range<usize>,
    pub(crate) kind: SyntaxTokenKind,
}

impl SyntaxTheme {
    pub(crate) fn style(self, kind: SyntaxTokenKind) -> HighlightStyle {
        let color = match kind {
            SyntaxTokenKind::Keyword => self.keyword,
            SyntaxTokenKind::Literal => self.literal,
            SyntaxTokenKind::String => self.string,
            SyntaxTokenKind::Comment => self.comment,
            SyntaxTokenKind::Number => self.number,
            SyntaxTokenKind::Type => self.r#type,
            SyntaxTokenKind::Function => self.function,
            SyntaxTokenKind::Metadata => self.metadata,
        };
        match color {
            Some(color) => HighlightStyle::default().color(color),
            None => HighlightStyle::default(),
        }
    }
}

/// Apply bounded, paint-only Tree-sitter syntax coloring to one UTF-8 document.
pub fn highlight_syntax(
    source: impl Into<Arc<str>>,
    language: SyntaxLanguage,
    theme: SyntaxTheme,
) -> StyledText {
    let source = source.into();
    let spans = syntax_spans(&source, language);
    StyledText::new(source).with_highlights(
        spans
            .into_iter()
            .map(|span| (span.range, theme.style(span.kind))),
    )
}

// Tree-sitter capture names can be more specific than QuickGUI's intentionally compact palette.
// HighlightConfiguration chooses the most specific matching entry; the parallel array maps that
// entry back to a stable public theme slot.
const CAPTURE_NAMES: &[&str] = &[
    "keyword",
    "operator",
    "boolean",
    "constant.builtin",
    "variable.builtin",
    "constant",
    "string.escape",
    "string.special",
    "string",
    "character",
    "comment",
    "number",
    "float",
    "type.builtin",
    "type",
    "constructor",
    "module",
    "namespace",
    "function.builtin",
    "function",
    "method",
    "attribute",
    "property",
    "tag",
    "label",
    "punctuation.special",
    "text.title",
    "text.literal",
    "text.uri",
    "text.reference",
    "text.emphasis",
    "text.strong",
];

const CAPTURE_KINDS: &[SyntaxTokenKind] = &[
    SyntaxTokenKind::Keyword,
    SyntaxTokenKind::Keyword,
    SyntaxTokenKind::Literal,
    SyntaxTokenKind::Literal,
    SyntaxTokenKind::Literal,
    SyntaxTokenKind::Literal,
    SyntaxTokenKind::String,
    SyntaxTokenKind::String,
    SyntaxTokenKind::String,
    SyntaxTokenKind::String,
    SyntaxTokenKind::Comment,
    SyntaxTokenKind::Number,
    SyntaxTokenKind::Number,
    SyntaxTokenKind::Type,
    SyntaxTokenKind::Type,
    SyntaxTokenKind::Type,
    SyntaxTokenKind::Type,
    SyntaxTokenKind::Type,
    SyntaxTokenKind::Function,
    SyntaxTokenKind::Function,
    SyntaxTokenKind::Function,
    SyntaxTokenKind::Metadata,
    SyntaxTokenKind::Metadata,
    SyntaxTokenKind::Metadata,
    SyntaxTokenKind::Metadata,
    SyntaxTokenKind::Metadata,
    SyntaxTokenKind::Metadata,
    SyntaxTokenKind::String,
    SyntaxTokenKind::Metadata,
    SyntaxTokenKind::Metadata,
    SyntaxTokenKind::Metadata,
    SyntaxTokenKind::Metadata,
];

struct SyntaxHighlighter {
    inner: TreeSitterHighlighter,
    #[cfg(all(feature = "language-packs", not(target_arch = "wasm32")))]
    wasm_ready: bool,
}
thread_local! {
    static TREE_SITTER_HIGHLIGHTER: RefCell<SyntaxHighlighter> = RefCell::new(SyntaxHighlighter {
        inner: TreeSitterHighlighter::new(),
        #[cfg(all(feature = "language-packs", not(target_arch = "wasm32")))]
        wasm_ready: false,
    });
}

#[cfg(feature = "bundled-languages")]
macro_rules! configuration_slot {
    ($slot:ident, $language:expr, $name:literal, $highlights:expr, $injections:expr, $locals:expr) => {{
        static $slot: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
        $slot
            .get_or_init(|| {
                let mut configuration = HighlightConfiguration::new(
                    $language.into(),
                    $name,
                    $highlights,
                    $injections,
                    $locals,
                )
                .ok()?;
                configuration.configure(CAPTURE_NAMES);
                Some(configuration)
            })
            .as_ref()
    }};
}

#[cfg(feature = "bundled-languages")]
fn configuration(
    language: SyntaxLanguage,
    registry: &registry::Registry,
) -> Option<&HighlightConfiguration> {
    match language {
        SyntaxLanguage::PlainText => None,
        SyntaxLanguage::Registered(id) => registry.configuration(id),
        SyntaxLanguage::Rust => configuration_slot!(
            RUST,
            tree_sitter_rust::LANGUAGE,
            "rust",
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            tree_sitter_rust::INJECTIONS_QUERY,
            ""
        ),
        SyntaxLanguage::JavaScript => {
            static JAVASCRIPT: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            JAVASCRIPT
                .get_or_init(|| {
                    let highlights = format!(
                        "{}\n{}",
                        tree_sitter_javascript::HIGHLIGHT_QUERY,
                        tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
                    );
                    let mut configuration = HighlightConfiguration::new(
                        tree_sitter_javascript::LANGUAGE.into(),
                        "javascript",
                        &highlights,
                        tree_sitter_javascript::INJECTIONS_QUERY,
                        tree_sitter_javascript::LOCALS_QUERY,
                    )
                    .ok()?;
                    configuration.configure(CAPTURE_NAMES);
                    Some(configuration)
                })
                .as_ref()
        }
        SyntaxLanguage::TypeScript | SyntaxLanguage::Tsx => {
            static TYPESCRIPT: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            static TSX: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            let slot = if language == SyntaxLanguage::Tsx {
                &TSX
            } else {
                &TYPESCRIPT
            };
            slot.get_or_init(|| {
                let highlights = if language == SyntaxLanguage::Tsx {
                    format!(
                        "{}\n{}\n{}",
                        tree_sitter_javascript::HIGHLIGHT_QUERY,
                        tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
                        tree_sitter_typescript::HIGHLIGHTS_QUERY
                    )
                } else {
                    format!(
                        "{}\n{}",
                        tree_sitter_javascript::HIGHLIGHT_QUERY,
                        tree_sitter_typescript::HIGHLIGHTS_QUERY
                    )
                };
                let locals = format!(
                    "{}\n{}",
                    tree_sitter_javascript::LOCALS_QUERY,
                    tree_sitter_typescript::LOCALS_QUERY
                );
                let mut configuration = HighlightConfiguration::new(
                    if language == SyntaxLanguage::Tsx {
                        tree_sitter_typescript::LANGUAGE_TSX.into()
                    } else {
                        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
                    },
                    if language == SyntaxLanguage::Tsx {
                        "tsx"
                    } else {
                        "typescript"
                    },
                    &highlights,
                    tree_sitter_javascript::INJECTIONS_QUERY,
                    &locals,
                )
                .ok()?;
                configuration.configure(CAPTURE_NAMES);
                Some(configuration)
            })
            .as_ref()
        }
        SyntaxLanguage::Python => configuration_slot!(
            PYTHON,
            tree_sitter_python::LANGUAGE,
            "python",
            tree_sitter_python::HIGHLIGHTS_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Go => configuration_slot!(
            GO,
            tree_sitter_go::LANGUAGE,
            "go",
            tree_sitter_go::HIGHLIGHTS_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Cpp => configuration_slot!(
            CPP,
            tree_sitter_cpp::LANGUAGE,
            "cpp",
            tree_sitter_cpp::HIGHLIGHT_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Java => configuration_slot!(
            JAVA,
            tree_sitter_java::LANGUAGE,
            "java",
            tree_sitter_java::HIGHLIGHTS_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Ruby => configuration_slot!(
            RUBY,
            tree_sitter_ruby::LANGUAGE,
            "ruby",
            tree_sitter_ruby::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_ruby::LOCALS_QUERY
        ),
        SyntaxLanguage::Swift => configuration_slot!(
            SWIFT,
            tree_sitter_swift::LANGUAGE,
            "swift",
            tree_sitter_swift::HIGHLIGHTS_QUERY,
            tree_sitter_swift::INJECTIONS_QUERY,
            tree_sitter_swift::LOCALS_QUERY
        ),
        SyntaxLanguage::Json => configuration_slot!(
            JSON,
            tree_sitter_json::LANGUAGE,
            "json",
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Yaml => configuration_slot!(
            YAML,
            tree_sitter_yaml::LANGUAGE,
            "yaml",
            tree_sitter_yaml::HIGHLIGHTS_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Toml => configuration_slot!(
            TOML,
            tree_sitter_toml_ng::LANGUAGE,
            "toml",
            tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Shell => configuration_slot!(
            SHELL,
            tree_sitter_bash::LANGUAGE,
            "bash",
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Css => configuration_slot!(
            CSS,
            tree_sitter_css::LANGUAGE,
            "css",
            tree_sitter_css::HIGHLIGHTS_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Html => configuration_slot!(
            HTML,
            tree_sitter_html::LANGUAGE,
            "html",
            tree_sitter_html::HIGHLIGHTS_QUERY,
            tree_sitter_html::INJECTIONS_QUERY,
            ""
        ),
        SyntaxLanguage::Sql => configuration_slot!(
            SQL,
            tree_sitter_sequel::LANGUAGE,
            "sql",
            tree_sitter_sequel::HIGHLIGHTS_QUERY,
            "",
            ""
        ),
        SyntaxLanguage::Markdown => configuration_slot!(
            MARKDOWN,
            tree_sitter_markdown::LANGUAGE,
            "markdown",
            tree_sitter_markdown::HIGHLIGHT_QUERY_BLOCK,
            tree_sitter_markdown::INJECTION_QUERY_BLOCK,
            ""
        ),
    }
}

#[cfg(feature = "bundled-languages")]
fn markdown_inline_configuration() -> Option<&'static HighlightConfiguration> {
    configuration_slot!(
        MARKDOWN_INLINE,
        tree_sitter_markdown::INLINE_LANGUAGE,
        "markdown_inline",
        tree_sitter_markdown::HIGHLIGHT_QUERY_INLINE,
        tree_sitter_markdown::INJECTION_QUERY_INLINE,
        ""
    )
}

#[cfg(not(feature = "bundled-languages"))]
fn configuration(
    language: SyntaxLanguage,
    registry: &registry::Registry,
) -> Option<&HighlightConfiguration> {
    match language {
        SyntaxLanguage::PlainText => None,
        SyntaxLanguage::Registered(id) => registry.configuration(id),
        _ => registry.by_name(&language.name()).and_then(|language| {
            if let SyntaxLanguage::Registered(id) = language {
                registry.configuration(id)
            } else {
                None
            }
        }),
    }
}

fn injected_configuration<'a>(
    name: &str,
    registry: &'a registry::Registry,
) -> Option<&'a HighlightConfiguration> {
    #[cfg(feature = "bundled-languages")]
    if name == "markdown_inline" {
        return markdown_inline_configuration();
    }
    {
        builtin_from_name(&name.to_ascii_lowercase())
            .or_else(|| registry.by_name(&name.to_ascii_lowercase()))
            .and_then(|language| configuration(language, registry))
    }
}

pub(crate) fn syntax_spans(source: &str, language: SyntaxLanguage) -> Vec<SyntaxSpan> {
    let registry = registry::snapshot();
    let Some(configuration) = configuration(language, &registry) else {
        return Vec::new();
    };
    if source.is_empty() {
        return Vec::new();
    }
    debug_assert_eq!(CAPTURE_NAMES.len(), CAPTURE_KINDS.len());
    let limit = floor_char_boundary(source, source.len().min(MAX_SYNTAX_SOURCE_BYTES));
    let source = &source[..limit];
    let excluded = overlong_line_ranges(source);

    TREE_SITTER_HIGHLIGHTER.with(|highlighter| {
        let mut state = highlighter.borrow_mut();
        #[cfg(all(feature = "language-packs", not(target_arch = "wasm32")))]
        if !state.wasm_ready
            && let Some(engine) = language_pack::engine()
        {
            let Ok(store) = tree_sitter::WasmStore::new(engine) else {
                return Vec::new();
            };
            if state.inner.parser.set_wasm_store(store).is_err() {
                return Vec::new();
            }
            state.wasm_ready = true;
        }
        let highlighter = &mut state.inner;
        let Ok(events) = highlighter.highlight(configuration, source.as_bytes(), None, |name| {
            injected_configuration(name, &registry)
        }) else {
            return Vec::new();
        };
        let mut stack = Vec::new();
        let mut spans = Vec::new();
        for event in events {
            let Ok(event) = event else {
                break;
            };
            match event {
                HighlightEvent::HighlightStart(highlight) => {
                    stack.push(CAPTURE_KINDS.get(highlight.0).copied());
                }
                HighlightEvent::HighlightEnd => {
                    let _ = stack.pop();
                }
                HighlightEvent::Source { start, end } => {
                    let Some(kind) = stack.iter().rev().flatten().next().copied() else {
                        continue;
                    };
                    if start >= end
                        || end > source.len()
                        || !source.is_char_boundary(start)
                        || !source.is_char_boundary(end)
                        || excluded
                            .iter()
                            .any(|line| line.start < end && line.end > start)
                    {
                        continue;
                    }
                    push_span(
                        &mut spans,
                        SyntaxSpan {
                            range: start..end,
                            kind,
                        },
                    );
                    if spans.len() == MAX_SYNTAX_HIGHLIGHTS {
                        break;
                    }
                }
            }
        }
        spans
    })
}

fn overlong_line_ranges(source: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for line in source.split_inclusive('\n') {
        let end = start + line.len();
        if line.strip_suffix('\n').unwrap_or(line).len() > MAX_SYNTAX_LINE_BYTES {
            ranges.push(start..end);
        }
        start = end;
    }
    ranges
}

fn push_span(spans: &mut Vec<SyntaxSpan>, span: SyntaxSpan) {
    if span.range.is_empty() {
        return;
    }
    if let Some(previous) = spans.last_mut()
        && previous.kind == span.kind
        && previous.range.end == span.range.start
    {
        previous.range.end = span.range.end;
    } else if spans.len() < MAX_SYNTAX_HIGHLIGHTS {
        spans.push(span);
    }
}

fn floor_char_boundary(value: &str, mut index: usize) -> usize {
    while !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(all(test, feature = "bundled-languages"))]
mod tests {
    use super::*;

    #[test]
    fn resolves_common_names_and_paths() {
        assert_eq!(SyntaxLanguage::from_name("tsx"), Some(SyntaxLanguage::Tsx));
        assert_eq!(
            SyntaxLanguage::from_name("javascript"),
            Some(SyntaxLanguage::JavaScript)
        );
        assert_eq!(
            SyntaxLanguage::from_path("src/main.rs"),
            Some(SyntaxLanguage::Rust)
        );
        assert_eq!(SyntaxLanguage::from_path("Dockerfile"), None);
        assert_eq!(SyntaxLanguage::from_name("kotlin"), None);
        assert_eq!(SyntaxLanguage::from_name("unknown"), None);
    }

    #[test]
    fn every_declared_language_has_a_tree_sitter_configuration() {
        for language in [
            SyntaxLanguage::Rust,
            SyntaxLanguage::JavaScript,
            SyntaxLanguage::TypeScript,
            SyntaxLanguage::Tsx,
            SyntaxLanguage::Python,
            SyntaxLanguage::Go,
            SyntaxLanguage::Cpp,
            SyntaxLanguage::Java,
            SyntaxLanguage::Ruby,
            SyntaxLanguage::Swift,
            SyntaxLanguage::Json,
            SyntaxLanguage::Yaml,
            SyntaxLanguage::Toml,
            SyntaxLanguage::Shell,
            SyntaxLanguage::Css,
            SyntaxLanguage::Html,
            SyntaxLanguage::Sql,
            SyntaxLanguage::Markdown,
        ] {
            assert!(
                configuration(language, &registry::snapshot()).is_some(),
                "missing {language:?}"
            );
        }
    }

    #[test]
    fn colors_incomplete_code_without_overlapping_ranges() {
        let source = "pub fn main() {\n  /* open\n  comment */ let value = \"ok\";\n}";
        let spans = syntax_spans(source, SyntaxLanguage::Rust);
        assert!(!spans.is_empty());
        assert!(
            spans
                .windows(2)
                .all(|pair| pair[0].range.end <= pair[1].range.start)
        );
        assert!(
            spans
                .iter()
                .any(|span| span.kind == SyntaxTokenKind::Comment)
        );
        assert!(
            spans
                .iter()
                .any(|span| span.kind == SyntaxTokenKind::String)
        );
        assert!(
            spans
                .iter()
                .all(|span| source.is_char_boundary(span.range.start)
                    && source.is_char_boundary(span.range.end))
        );
    }

    #[test]
    fn source_and_line_work_are_bounded() {
        let long = format!("{}\nlet value = 1;", "x".repeat(MAX_SYNTAX_LINE_BYTES + 1));
        let spans = syntax_spans(&long, SyntaxLanguage::Rust);
        assert!(spans.len() <= MAX_SYNTAX_HIGHLIGHTS);
        assert!(
            spans
                .iter()
                .all(|span| span.range.start > MAX_SYNTAX_LINE_BYTES)
        );
    }

    #[test]
    fn markup_captures_are_flattened_to_non_overlapping_paint_spans() {
        let source = "<div class=\"card\">value</div>";
        let spans = syntax_spans(source, SyntaxLanguage::Html);
        assert!(
            spans
                .windows(2)
                .all(|pair| pair[0].range.end <= pair[1].range.start)
        );
        let _ = highlight_syntax(source, SyntaxLanguage::Html, SyntaxTheme::default());
    }

    #[test]
    fn markdown_inline_configuration_and_document_highlights_are_available() {
        let source = "# Heading with `code`\n";
        let spans = syntax_spans(source, SyntaxLanguage::Markdown);
        assert!(!spans.is_empty());
        assert!(markdown_inline_configuration().is_some());
    }
}
