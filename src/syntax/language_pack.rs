//! Portable single-file language packs. Grammar modules are Wasm, never native libraries.
use super::{
    SyntaxLanguage, SyntaxLanguageDefinition, SyntaxLanguageError,
    registry::{self, PendingLanguage, error},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Cursor, Read},
    path::Path,
    sync::OnceLock,
};
use tree_sitter::{WasmStore, wasmtime};

pub const MAX_LANGUAGE_PACK_BYTES: usize = 64 * 1024 * 1024;
const MAX_WASM_BYTES: usize = 16 * 1024 * 1024;
static ENGINE: OnceLock<Result<wasmtime::Engine, String>> = OnceLock::new();
pub(super) fn engine() -> Option<&'static wasmtime::Engine> {
    ENGINE.get().and_then(|engine| engine.as_ref().ok())
}
fn initialize_engine() -> Result<&'static wasmtime::Engine, SyntaxLanguageError> {
    ENGINE
        .get_or_init(|| wasmtime::Engine::new(&wasmtime::Config::new()).map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| error(e.clone()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Pack {
    format_version: u32,
    grammars: BTreeMap<String, String>,
    languages: Vec<Language>,
    #[serde(default)]
    licenses: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Language {
    name: String,
    grammar: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    filenames: Vec<String>,
    queries: Queries,
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
#[derive(Default)]
struct Queries {
    highlights: String,
    injections: String,
    locals: String,
}

/// Load a binary ustar pack containing every selected raw Wasm module and a JSON manifest with
/// language metadata and queries. No files are extracted or native libraries loaded.
pub fn load_syntax_language_pack(
    path: impl AsRef<Path>,
) -> Result<Vec<SyntaxLanguage>, SyntaxLanguageError> {
    let file =
        File::open(path.as_ref()).map_err(|e| error(format!("cannot open language pack: {e}")))?;
    let mut bytes = Vec::new();
    file.take(MAX_LANGUAGE_PACK_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| error(e.to_string()))?;
    load_syntax_language_pack_bytes(&bytes)
}

/// Load a pack directly from embedded application bytes. Registrations are atomic and immutable.
pub fn load_syntax_language_pack_bytes(
    bytes: &[u8],
) -> Result<Vec<SyntaxLanguage>, SyntaxLanguageError> {
    if bytes.len() > MAX_LANGUAGE_PACK_BYTES {
        return Err(error("language pack exceeds 64 MiB"));
    }
    let members = pack_members(bytes)?;
    let manifest = members
        .get("manifest.json")
        .ok_or_else(|| error("language pack is missing manifest.json"))?;
    if manifest.len() > 16 * 1024 * 1024 {
        return Err(error("language pack manifest exceeds 16 MiB"));
    }
    let pack: Pack = serde_json::from_slice(manifest)
        .map_err(|e| error(format!("invalid language pack manifest: {e}")))?;
    if pack.format_version != 1 {
        return Err(error("unsupported language pack formatVersion"));
    }
    if pack.languages.is_empty()
        || pack.languages.len() > registry::MAX_REGISTERED_SYNTAX_LANGUAGES
        || pack.grammars.len() > registry::MAX_REGISTERED_SYNTAX_LANGUAGES
    {
        return Err(error("invalid language pack language count"));
    }
    drop(pack.licenses);
    let mut names = BTreeSet::new();
    let selected: BTreeSet<_> = pack
        .languages
        .iter()
        .map(|language| language.grammar.clone())
        .collect();
    if selected.len() != pack.grammars.len()
        || selected
            .iter()
            .any(|name| !pack.grammars.contains_key(name))
    {
        return Err(error("pack grammars must match the selected languages"));
    }
    for language in &pack.languages {
        if !names.insert(language.name.trim().to_ascii_lowercase()) {
            return Err(error("duplicate language name in pack"));
        }
        if language
            .queries
            .highlights
            .len()
            .saturating_add(language.queries.injections.len())
            .saturating_add(language.queries.locals.len())
            > registry::MAX_LANGUAGE_QUERY_BYTES
        {
            return Err(error("language queries exceed 1 MiB"));
        }
    }
    let files: BTreeSet<_> = pack.grammars.values().map(String::as_str).collect();
    if files.len() + 1 != members.len()
        || files
            .iter()
            .any(|path| *path == "manifest.json" || !members.contains_key(*path))
    {
        return Err(error("pack members must match the selected grammars"));
    }
    let current = registry::snapshot();
    let mut decoded = BTreeMap::new();
    for (name, path) in pack.grammars {
        if name.is_empty()
            || name.len() > 64
            || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err(error("invalid Wasm grammar name"));
        }
        let module = members[path.as_str()];
        if module.len() > MAX_WASM_BYTES || !module.starts_with(b"\0asm\x01\0\0\0") {
            return Err(error("grammar must be a compiled Wasm module"));
        }
        let identity = (name.clone(), Sha256::digest(module).into());
        decoded.insert(name, (module, identity));
    }
    let mut store = None;
    let mut grammars = BTreeMap::new();
    for (name, (bytes, identity)) in decoded {
        let grammar = match current.wasm_grammar(&identity) {
            Some(grammar) => grammar,
            None => {
                if store.is_none() {
                    store = Some(
                        WasmStore::new(initialize_engine()?).map_err(|e| error(e.to_string()))?,
                    );
                }
                store
                    .as_mut()
                    .unwrap()
                    .load_language(&name, bytes)
                    .map_err(|e| error(format!("cannot load grammar {name}: {e}")))?
            }
        };
        grammars.insert(name, (grammar, identity, bytes.len()));
    }
    let pending = pack
        .languages
        .into_iter()
        .map(|language| {
            let (grammar, identity, bytes) = &grammars[&language.grammar];
            PendingLanguage {
                definition: SyntaxLanguageDefinition {
                    name: language.name,
                    grammar: grammar.clone(),
                    highlights: language.queries.highlights,
                    injections: language.queries.injections,
                    locals: language.queries.locals,
                    aliases: language.aliases,
                    extensions: language.extensions,
                    filenames: language.filenames,
                },
                wasm_identity: Some(identity.clone()),
                grammar_bytes: *bytes,
            }
        })
        .collect();
    registry::register_many(pending)
}

/// Validate headers and expose borrowed module slices; never decode, copy or extract their data.
fn pack_members(bytes: &[u8]) -> Result<BTreeMap<String, &[u8]>, SyntaxLanguageError> {
    let mut archive = tar::Archive::new(Cursor::new(bytes));
    let mut members = BTreeMap::new();
    let entries = archive
        .entries()
        .map_err(|e| error(format!("invalid pack archive: {e}")))?;
    for entry in entries.raw(true) {
        let entry = entry.map_err(|e| error(format!("invalid pack archive: {e}")))?;
        if !entry.header().entry_type().is_file() || entry.header().as_ustar().is_none() {
            return Err(error("pack members must be regular ustar files"));
        }
        let path = entry.path().map_err(|e| error(e.to_string()))?;
        let name = path
            .to_str()
            .ok_or_else(|| error("pack paths must be UTF-8"))?;
        if name.is_empty()
            || name.contains('\\')
            || name.starts_with('/')
            || name
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(error("invalid pack member path"));
        }
        if members.len() >= registry::MAX_REGISTERED_SYNTAX_LANGUAGES + 1 {
            return Err(error("too many pack members"));
        }
        let start =
            usize::try_from(entry.raw_file_position()).map_err(|_| error("invalid pack offset"))?;
        let size = usize::try_from(entry.size()).map_err(|_| error("invalid pack size"))?;
        let end = start
            .checked_add(size)
            .ok_or_else(|| error("invalid pack size"))?;
        let contents = bytes
            .get(start..end)
            .ok_or_else(|| error("truncated pack member"))?;
        if members.insert(name.to_owned(), contents).is_some() {
            return Err(error("duplicate pack member"));
        }
    }
    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_packs_do_not_register_languages() {
        for bytes in [
            b"{}".as_slice(),
            br#"{"formatVersion":1,"grammars":{},"languages":[]}"#,
        ] {
            assert!(load_syntax_language_pack_bytes(bytes).is_err());
        }
        let bytes = vec![b' '; MAX_LANGUAGE_PACK_BYTES + 1];
        assert!(load_syntax_language_pack_bytes(&bytes).is_err());
    }
    #[test]
    fn raw_modules_are_borrowed_and_links_or_duplicates_are_rejected() {
        fn archive(duplicate: bool, link: bool) -> Vec<u8> {
            let mut builder = tar::Builder::new(Vec::new());
            for _ in 0..if duplicate { 2 } else { 1 } {
                let mut header = tar::Header::new_ustar();
                header.set_size(8);
                header.set_mode(0o644);
                if link {
                    header.set_entry_type(tar::EntryType::Symlink);
                }
                header.set_cksum();
                builder
                    .append_data(
                        &mut header,
                        "grammars/test.wasm",
                        b"\0asm\x01\0\0\0".as_slice(),
                    )
                    .unwrap();
            }
            builder.into_inner().unwrap()
        }
        let bytes = archive(false, false);
        let members = pack_members(&bytes).unwrap();
        assert_eq!(
            members["grammars/test.wasm"].as_ptr(),
            bytes[512..].as_ptr()
        );
        assert!(pack_members(&archive(true, false)).is_err());
        assert!(pack_members(&archive(false, true)).is_err());
    }
}
