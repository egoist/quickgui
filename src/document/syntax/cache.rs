//! Bounded cache for neutral syntax documents.
//!
//! Ported from Comet (https://github.com/zeronsh/comet), MIT.
//! Original: `crates/ui/src/syntax_cache.rs`.
//!
//! Colours and gpui runs deliberately stay OUTSIDE this cache. A theme change
//! then recolours existing spans without reparsing, and one cached document
//! serves both appearances.
//!
//! GPUIX is immediate-mode: `<code>` re-renders on every frame. Without this
//! cache a 200-line snippet is reparsed 60 times a second.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use super::{HighlightRequest, HighlightedDocument, LanguageId, highlight};

/// Bump when the scope map or Syntect syntax dump changes, so stale entries
/// from a previous build of the same process cannot be served.
pub const QUERY_GENERATION: u32 = 2;
const MAX_DOCUMENTS: usize = 96;
const MAX_RETAINED_BYTES: usize = 24 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentKey {
    language: LanguageId,
    content_hash: u64,
    content_len: usize,
    query_generation: u32,
}

impl DocumentKey {
    fn new(language: LanguageId, source: &str) -> Self {
        Self {
            language,
            // A 64-bit SipHash from the standard library. Comet uses SHA-256;
            // a collision here paints the wrong colours for one snippet, never
            // corrupts memory, and the length is mixed in as a second check.
            content_hash: hash64(source),
            content_len: source.len(),
            query_generation: QUERY_GENERATION,
        }
    }
}

fn hash64(source: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

struct CachedDocument {
    retained_bytes: usize,
    document: Arc<HighlightedDocument>,
}

#[derive(Default)]
pub struct SyntaxCache {
    documents: HashMap<DocumentKey, CachedDocument>,
    recency: VecDeque<DocumentKey>,
    retained_bytes: usize,
    hits: u64,
    misses: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub documents: usize,
    pub retained_bytes: usize,
}

impl SyntaxCache {
    fn get(&mut self, key: &DocumentKey) -> Option<Arc<HighlightedDocument>> {
        let Some(document) = self.documents.get(key).map(|entry| entry.document.clone()) else {
            self.misses += 1;
            return None;
        };
        self.hits += 1;
        self.touch(*key);
        Some(document)
    }

    fn insert(&mut self, key: DocumentKey, document: Arc<HighlightedDocument>) {
        if let Some(previous) = self.documents.remove(&key) {
            self.retained_bytes = self.retained_bytes.saturating_sub(previous.retained_bytes);
        }
        let retained_bytes = estimated_bytes(&document);
        if retained_bytes > MAX_RETAINED_BYTES {
            self.recency.retain(|candidate| *candidate != key);
            return;
        }
        self.retained_bytes = self.retained_bytes.saturating_add(retained_bytes);
        self.documents.insert(
            key,
            CachedDocument {
                retained_bytes,
                document,
            },
        );
        self.touch(key);
        while self.documents.len() > MAX_DOCUMENTS || self.retained_bytes > MAX_RETAINED_BYTES {
            let Some(oldest) = self.recency.pop_front() else {
                break;
            };
            if let Some(removed) = self.documents.remove(&oldest) {
                self.retained_bytes = self.retained_bytes.saturating_sub(removed.retained_bytes);
            }
        }
    }

    fn touch(&mut self, key: DocumentKey) {
        self.recency.retain(|candidate| *candidate != key);
        self.recency.push_back(key);
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits,
            misses: self.misses,
            documents: self.documents.len(),
            retained_bytes: self.retained_bytes,
        }
    }
}

fn estimated_bytes(document: &HighlightedDocument) -> usize {
    let spans: usize = document.lines.iter().map(|line| line.len()).sum();
    std::mem::size_of::<HighlightedDocument>()
        + document.lines.len() * std::mem::size_of::<Vec<super::HighlightSpan>>()
        + spans * std::mem::size_of::<super::HighlightSpan>()
}

fn global() -> &'static Mutex<SyntaxCache> {
    static CACHE: OnceLock<Mutex<SyntaxCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(SyntaxCache::default()))
}

/// Highlight through the process-wide cache.
///
/// Returns `None` when the language is unknown or the source is too large,
/// which callers render as plain text. Negative results are NOT cached: they
/// are cheap to recompute and caching them would keep unparseable megabytes
/// keyed forever.
pub fn highlight_cached(
    source: &str,
    path: Option<&str>,
    fence_tag: Option<&str>,
) -> Option<Arc<HighlightedDocument>> {
    let language = super::detect_language(path, fence_tag, source.lines().next())?;
    let key = DocumentKey::new(language, source);
    if let Some(cached) = global().lock().ok()?.get(&key) {
        return Some(cached);
    }
    let document = highlight(HighlightRequest {
        source,
        path,
        fence_tag,
    })
    .ok()?;
    let document = Arc::new(document);
    if let Ok(mut cache) = global().lock() {
        cache.insert(key, document.clone());
    }
    Some(document)
}

pub fn stats() -> CacheStats {
    global()
        .lock()
        .map(|cache| cache.stats())
        .unwrap_or(CacheStats {
            hits: 0,
            misses: 0,
            documents: 0,
            retained_bytes: 0,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_lookup_hits_the_cache() {
        let before = stats().hits;
        let source = "fn cache_probe() -> u32 { 7 }";
        let a = highlight_cached(source, Some("probe.rs"), None).unwrap();
        let b = highlight_cached(source, Some("probe.rs"), None).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "the same Arc must be served twice");
        assert!(stats().hits > before);
    }

    #[test]
    fn different_sources_do_not_share_an_entry() {
        let a = highlight_cached("let a = 1;", Some("a.rs"), None).unwrap();
        let b = highlight_cached("let bb = 2;", Some("a.rs"), None).unwrap();
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn unknown_language_returns_none() {
        assert!(highlight_cached("x", Some("notes.txt"), None).is_none());
    }

    #[test]
    fn eviction_keeps_the_cache_bounded() {
        let mut cache = SyntaxCache::default();
        for i in 0..(MAX_DOCUMENTS + 20) {
            let source = format!("let x{i} = {i};");
            let document = highlight(HighlightRequest {
                source: &source,
                path: Some("a.rs"),
                fence_tag: None,
            })
            .unwrap();
            cache.insert(
                DocumentKey::new(LanguageId::Rust, &source),
                Arc::new(document),
            );
        }
        assert!(cache.stats().documents <= MAX_DOCUMENTS);
    }
}
