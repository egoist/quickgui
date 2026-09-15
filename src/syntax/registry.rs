use super::{CAPTURE_NAMES, SyntaxLanguage};
use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, OnceLock, RwLock},
};
use tree_sitter_highlight::HighlightConfiguration;

pub const MAX_REGISTERED_SYNTAX_LANGUAGES: usize = 128;
pub const MAX_LANGUAGE_QUERY_BYTES: usize = 1024 * 1024;
pub const MAX_LANGUAGE_REGISTRY_BYTES: usize = 256 * 1024 * 1024;
const MAX_REGISTRY_QUERY_BYTES: usize = 16 * 1024 * 1024;
const MAX_NAMES: usize = 32;

/// Opaque process-local identity. Registrations are immutable and live until process exit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RegisteredSyntaxLanguage(pub(super) u16);

/// A linked Tree-sitter grammar and the queries and names that belong to it.
/// Unspecified capture colors still inherit from the application's syntax theme.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxLanguageDefinition {
    pub name: String,
    pub grammar: tree_sitter::Language,
    pub highlights: String,
    pub injections: String,
    pub locals: String,
    pub aliases: Vec<String>,
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
}

impl SyntaxLanguageDefinition {
    pub fn new(
        name: impl Into<String>,
        grammar: tree_sitter::Language,
        highlights: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            grammar,
            highlights: highlights.into(),
            injections: String::new(),
            locals: String::new(),
            aliases: Vec::new(),
            extensions: Vec::new(),
            filenames: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxLanguageError(pub(super) String);
impl fmt::Display for SyntaxLanguageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SyntaxLanguageError {}
pub(super) fn error(message: impl Into<String>) -> SyntaxLanguageError {
    SyntaxLanguageError(message.into())
}

pub(super) struct Registration {
    pub configuration: HighlightConfiguration,
    definition: SyntaxLanguageDefinition,
    // Reuse compiled Wasm by grammar export name and content hash.
    #[cfg_attr(not(feature = "language-packs"), allow(dead_code))]
    wasm_identity: Option<(String, [u8; 32])>,
}

#[derive(Clone, Default)]
pub(super) struct Registry {
    pub generation: u64,
    pub entries: Vec<Arc<Registration>>,
    names: BTreeMap<String, SyntaxLanguage>,
    extensions: BTreeMap<String, SyntaxLanguage>,
    filenames: BTreeMap<String, SyntaxLanguage>,
    bytes: usize,
    query_bytes: usize,
}

static REGISTRY: OnceLock<RwLock<Arc<Registry>>> = OnceLock::new();
fn registry() -> &'static RwLock<Arc<Registry>> {
    REGISTRY.get_or_init(Default::default)
}
pub(super) fn snapshot() -> Arc<Registry> {
    registry().read().unwrap_or_else(|e| e.into_inner()).clone()
}
pub fn syntax_language_generation() -> u64 {
    snapshot().generation
}

impl Registry {
    fn has_capacity(&self, bytes: usize, query_bytes: usize) -> bool {
        self.entries.len() < MAX_REGISTERED_SYNTAX_LANGUAGES
            && self.bytes.saturating_add(bytes) <= MAX_LANGUAGE_REGISTRY_BYTES
            && self.query_bytes.saturating_add(query_bytes) <= MAX_REGISTRY_QUERY_BYTES
    }
    pub fn from_name(&self, name: &str) -> Option<SyntaxLanguage> {
        self.names
            .get(name)
            .or_else(|| self.extensions.get(name))
            .copied()
    }
    pub fn from_filename(&self, name: &str) -> Option<SyntaxLanguage> {
        self.filenames.get(name).copied()
    }
    pub fn configuration(&self, id: RegisteredSyntaxLanguage) -> Option<&HighlightConfiguration> {
        self.entries.get(id.0 as usize).map(|e| &e.configuration)
    }
    pub fn language_name(&self, id: RegisteredSyntaxLanguage) -> Option<&str> {
        self.entries
            .get(id.0 as usize)
            .map(|e| e.definition.name.as_str())
    }
}

fn normalized_name(value: &str) -> Result<String, SyntaxLanguageError> {
    let value = value.trim().trim_start_matches('.').to_ascii_lowercase();
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.+#".contains(&c))
    {
        return Err(error(
            "language names must contain 1–64 ASCII letters, digits, '-', '_', '.', '+', or '#'",
        ));
    }
    Ok(value)
}

fn normalize(def: &mut SyntaxLanguageDefinition) -> Result<usize, SyntaxLanguageError> {
    def.name = normalized_name(&def.name)?;
    for names in [&mut def.aliases, &mut def.extensions] {
        if names.len() > MAX_NAMES {
            return Err(error("too many language aliases, extensions, or filenames"));
        }
        for name in names.iter_mut() {
            *name = normalized_name(name)?;
        }
        names.sort();
        names.dedup();
    }
    if def.filenames.len() > MAX_NAMES {
        return Err(error("too many language filenames"));
    }
    for name in &mut def.filenames {
        *name = name.trim().to_ascii_lowercase();
        if name.is_empty()
            || name.len() > 255
            || matches!(name.as_str(), "." | "..")
            || name.contains(['/', '\\', '\0'])
        {
            return Err(error("invalid language filename"));
        }
    }
    def.filenames.sort();
    def.filenames.dedup();
    let bytes = def
        .highlights
        .len()
        .saturating_add(def.injections.len())
        .saturating_add(def.locals.len());
    if bytes > MAX_LANGUAGE_QUERY_BYTES {
        return Err(error("language queries exceed 1 MiB"));
    }
    let abi = def.grammar.abi_version();
    if !(tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION..=tree_sitter::LANGUAGE_VERSION)
        .contains(&abi)
    {
        return Err(error(format!("unsupported Tree-sitter grammar ABI {abi}")));
    }
    Ok(bytes)
}

/// Register an independently supplied, statically linked grammar. Identical registration is a
/// no-op. Conflicting names/queries fail atomically instead of replacing another language.
pub fn register_syntax_language(
    definition: SyntaxLanguageDefinition,
) -> Result<SyntaxLanguage, SyntaxLanguageError> {
    register_many(vec![PendingLanguage {
        definition,
        wasm_identity: None,
        grammar_bytes: 0,
    }])
    .map(|mut ids| ids.remove(0))
}

pub(super) struct PendingLanguage {
    pub definition: SyntaxLanguageDefinition,
    pub wasm_identity: Option<(String, [u8; 32])>,
    pub grammar_bytes: usize,
}

impl Registry {
    #[cfg(feature = "language-packs")]
    pub fn wasm_grammar(&self, identity: &(String, [u8; 32])) -> Option<tree_sitter::Language> {
        self.entries
            .iter()
            .find(|entry| entry.wasm_identity.as_ref() == Some(identity))
            .map(|entry| entry.configuration.language.clone())
    }

    fn existing(
        &self,
        definition: &SyntaxLanguageDefinition,
    ) -> Result<Option<SyntaxLanguage>, SyntaxLanguageError> {
        if let Some(SyntaxLanguage::Registered(id)) = self.names.get(&definition.name) {
            if self.entries[id.0 as usize].definition == *definition {
                return Ok(Some(SyntaxLanguage::Registered(*id)));
            }
            return Err(error(format!(
                "language '{}' is already registered with a different definition",
                definition.name
            )));
        }
        for name in std::iter::once(&definition.name)
            .chain(&definition.aliases)
            .chain(&definition.extensions)
        {
            if super::builtin_from_name(name).is_some() || self.from_name(name).is_some() {
                return Err(error(format!(
                    "language name or extension '{name}' is already registered"
                )));
            }
        }
        for name in &definition.filenames {
            if self.filenames.contains_key(name)
                || (cfg!(feature = "bundled-languages")
                    && matches!(name.as_str(), "gemfile" | "rakefile"))
            {
                return Err(error(format!("filename '{name}' is already registered")));
            }
        }
        Ok(None)
    }
}

/// Compile outside the lock, then publish every language in one immutable registry snapshot.
/// Failed packs leave no names, configurations or generation changes behind.
pub(super) fn register_many(
    pending: Vec<PendingLanguage>,
) -> Result<Vec<SyntaxLanguage>, SyntaxLanguageError> {
    if pending.is_empty() || pending.len() > MAX_REGISTERED_SYNTAX_LANGUAGES {
        return Err(error("invalid language count"));
    }
    let current = snapshot();
    let mut prepared = Vec::with_capacity(pending.len());
    for mut item in pending {
        let query_bytes = normalize(&mut item.definition)?;
        let bytes = query_bytes.saturating_add(item.grammar_bytes);
        let configuration = if current.existing(&item.definition)?.is_some() {
            None
        } else {
            if !current.has_capacity(bytes, query_bytes) {
                return Err(error("language registry capacity exceeded"));
            }
            let mut config = HighlightConfiguration::new(
                item.definition.grammar.clone(),
                item.definition.name.clone(),
                &item.definition.highlights,
                &item.definition.injections,
                &item.definition.locals,
            )
            .map_err(|e| error(format!("invalid queries for {}: {e}", item.definition.name)))?;
            config.configure(CAPTURE_NAMES);
            Some(config)
        };
        prepared.push((item, configuration, bytes, query_bytes));
    }
    let mut guard = registry().write().unwrap_or_else(|e| e.into_inner());
    let mut next = (**guard).clone();
    let mut ids = Vec::with_capacity(prepared.len());
    for (item, configuration, bytes, query_bytes) in prepared {
        if let Some(id) = next.existing(&item.definition)? {
            ids.push(id);
            continue;
        }
        if !next.has_capacity(bytes, query_bytes) {
            return Err(error("language registry capacity exceeded"));
        }
        let configuration =
            configuration.ok_or_else(|| error("registered language unexpectedly disappeared"))?;
        let id = SyntaxLanguage::Registered(RegisteredSyntaxLanguage(next.entries.len() as u16));
        next.names.insert(item.definition.name.clone(), id);
        for name in &item.definition.aliases {
            next.names.insert(name.clone(), id);
        }
        for name in &item.definition.extensions {
            next.extensions.insert(name.clone(), id);
        }
        for name in &item.definition.filenames {
            next.filenames.insert(name.clone(), id);
        }
        next.entries.push(Arc::new(Registration {
            configuration,
            definition: item.definition,
            wasm_identity: item.wasm_identity,
        }));
        next.bytes += bytes;
        next.query_bytes += query_bytes;
        ids.push(id);
    }
    if next.entries.len() != guard.entries.len() {
        next.generation += 1;
        *guard = Arc::new(next);
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lua(name: &str) -> SyntaxLanguageDefinition {
        SyntaxLanguageDefinition::new(
            name,
            tree_sitter_lua::LANGUAGE.into(),
            "\"local\" @keyword\n(number) @number\n(string) @string",
        )
    }

    #[test]
    fn registers_new_grammars_aliases_extensions_and_exact_filenames() {
        let mut definition = lua("test-registered-lua");
        definition.aliases = vec!["test-lua-alias".into()];
        definition.extensions = vec![".testlua".into()];
        definition.filenames = vec![".testrc".into()];
        let id = register_syntax_language(definition.clone()).unwrap();
        assert_eq!(SyntaxLanguage::from_name("TEST-LUA-ALIAS"), Some(id));
        assert_eq!(SyntaxLanguage::from_path("/project/main.testlua"), Some(id));
        assert_eq!(SyntaxLanguage::from_path("/project/.testrc"), Some(id));
        let config = snapshot();
        let SyntaxLanguage::Registered(key) = id else {
            panic!()
        };
        let original = Arc::as_ptr(&config.entries[key.0 as usize]);
        assert_eq!(register_syntax_language(definition.clone()).unwrap(), id);
        assert_eq!(Arc::as_ptr(&snapshot().entries[key.0 as usize]), original);
        definition.highlights = "(identifier) @function".into();
        assert!(register_syntax_language(definition).is_err());
        let spans = super::super::syntax_spans("local answer = 42", id);
        assert!(
            spans
                .iter()
                .any(|span| span.kind == super::super::SyntaxTokenKind::Keyword)
        );
        assert!(
            spans
                .iter()
                .any(|span| span.kind == super::super::SyntaxTokenKind::Number)
        );
    }

    #[test]
    fn rejects_invalid_queries_and_conflicts_without_publishing_partial_names() {
        let mut definition = lua("test-invalid-query");
        definition.highlights = "(not_a_lua_node) @keyword".into();
        assert!(register_syntax_language(definition).is_err());
        assert_eq!(SyntaxLanguage::from_name("test-invalid-query"), None);
        let mut definition = lua("test-reserved-alias");
        definition.aliases = vec!["text".into()];
        assert!(register_syntax_language(definition).is_err());
        assert_eq!(SyntaxLanguage::from_name("test-reserved-alias"), None);
        let mut definition = lua("test-oversized-query");
        definition.highlights = " ".repeat(MAX_LANGUAGE_QUERY_BYTES + 1);
        assert!(register_syntax_language(definition).is_err());
        let mut definition = lua("../bad");
        definition.aliases = vec!["unused".into()];
        assert!(register_syntax_language(definition).is_err());
    }

    #[test]
    fn registered_injection_grammars_work_without_any_bundled_languages() {
        let mut nested = lua("test-nested-lua");
        nested.aliases = vec!["test-nested-fence".into()];
        register_syntax_language(nested).unwrap();
        let mut outer = lua("test-host-lua");
        outer.highlights = String::new();
        outer.injections="((string_content) @injection.content (#set! injection.language \"test-nested-fence\"))".into();
        let id = register_syntax_language(outer).unwrap();
        let source = "local code = \"local answer = 42\"";
        let spans = super::super::syntax_spans(source, id);
        let number = source.find("42").unwrap();
        assert!(spans.iter().any(|span| span.range.contains(&number)
            && span.kind == super::super::SyntaxTokenKind::Number));
    }

    #[test]
    #[cfg(feature = "bundled-languages")]
    fn registered_aliases_resolve_inside_markdown_injections() {
        let mut definition = lua("test-injected-lua");
        definition.aliases = vec!["test-lua-fence".into()];
        register_syntax_language(definition).unwrap();
        let source = "```test-lua-fence\nlocal answer = 42\n```\n";
        let spans = super::super::syntax_spans(source, SyntaxLanguage::Markdown);
        let number = source.find("42").unwrap();
        assert!(spans.iter().any(|span| span.range.contains(&number)
            && span.kind == super::super::SyntaxTokenKind::Number));
    }
}
