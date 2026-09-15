# Editor language packs

Editor, CodeBlock and DiffView include **no language grammars by default**. Selecting an unknown
language produces plain text and never downloads anything. Syntax colors remain application-owned.

The runtime uses Tree-sitter's Wasm grammar loader, as
[Zed does](https://zed.dev/blog/language-extensions-part-1). Grammar files are compiled `.wasm`
modules; `highlights.scm`, injection queries and local queries are text. There is no native
`.dylib`/`.so`/`.dll` grammar loader.

## Build one portable pack

The CLI can fetch and compile pinned grammar repositories itself; no application build script is
needed. For example, `examples/extensions/languages.json` contains:

```json
{
  "grammars": {
    "rust": {
      "repository": "https://github.com/tree-sitter/tree-sitter-rust",
      "rev": "77a3747266f4d621d0757825e6b11edcbf991ca5"
    }
  },
  "languages": [{ "name": "rust", "grammar": "rust", "extensions": ["rs"] }]
}
```

```sh
quickgui pack-languages languages.json --out resources/languages.qglang
```

Repository revisions must be full commit hashes. The CLI caches checkouts and compiled Wasm under
`.quickgui/language-packs/`; unchanged output is not rewritten. A grammar's optional `path` selects
a subdirectory in the repository. By default, queries come from `queries/highlights.scm`,
`queries/injections.scm`, and `queries/locals.scm` when present, and the grammar license is included.
A repository grammar may specify `queries` paths relative to its checkout to override these defaults.
Language-level query paths override those defaults and remain relative to the pack configuration.

Alternatively, supply precompiled Wasm files. Pack authors can compile them with the standard [Tree-sitter CLI](https://tree-sitter.github.io/tree-sitter/cli/build.html):

```sh
tree-sitter build --wasm --output grammars/lua.wasm path/to/tree-sitter-lua
tree-sitter build --wasm --output grammars/rust.wasm path/to/tree-sitter-rust
```

For repository sources the QuickGUI CLI invokes a pinned Tree-sitter CLI and its WASI SDK;
compilation happens only on cache misses. Applications consuming prebuilt packs do not
need a compiler. Commit/publish the generated pack as an application resource.

Create a text configuration; all paths are relative to this file:

```json
{
  "grammars": {
    "lua": "grammars/lua.wasm",
    "rust": "grammars/rust.wasm"
  },
  "languages": [
    {
      "name": "lua",
      "grammar": "lua",
      "aliases": ["lua-fence"],
      "extensions": ["lua"],
      "filenames": [".luarc"],
      "queries": { "highlights": "queries/lua/highlights.scm" }
    },
    {
      "name": "rust",
      "grammar": "rust",
      "extensions": ["rs"],
      "queries": { "highlights": "queries/rust/highlights.scm" }
    }
  ],
  "licenses": {
    "lua": "licenses/lua.txt",
    "rust": "licenses/rust.txt"
  }
}
```

```sh
quickgui pack-languages languages.json --languages lua,rust --out resources/languages.qglang
```

Omit `--languages` to include every configured language. A language may declare
`"dependencies": ["javascript", "css"]` to include embedded languages automatically. The builder
includes only selected languages and their declared dependencies, and stores shared grammars once.
Query values may be a path or an array of paths; arrays concatenate queries in order, useful for
TypeScript/JavaScript query inheritance. Include the grammar authors' licenses.

The result is **one portable binary ustar file**, containing a UTF-8 `manifest.json` with metadata,
inline query text and licenses, plus raw `.wasm` members. There is no base64 or decompression step.
The loader validates archive headers and passes borrowed module slices directly to Tree-sitter;
it never extracts files or copies/decodes module payloads. The exact same pack works on all
supported desktop targets. The text configuration is only for building the pack, not for parsing
`grammar.js` at runtime.

## TypeScript

After application readiness, load the pack once and select language names in components:

```ts
import { join } from "node:path";
import { app } from "@quickgui/native";
import { loadLanguagePack } from "@quickgui/extension-editor";

await app.whenReady();
const paths = await app.getPaths();
if (!paths) throw new Error("Application paths are unavailable");
const languages = await loadLanguagePack(join(paths.resourceDir, "languages.qglang"));
// languages is ["lua", "rust"]. Use language="lua" on Editor or CodeBlock.
```

## Go

```go
import (
    "path/filepath"
    "github.com/egoist/quickgui/extensions/editor"
    "github.com/egoist/quickgui/go/native"
)

// Run after application readiness; callbacks return on the UI goroutine.
native.App.GetPaths(func(paths *native.AppPaths, err error) {
    if err != nil { reportError(err); return }
    editor.LoadLanguagePack(filepath.Join(paths.ResourceDir, "languages.qglang"),
        func(languages []string, err error) {
            if err != nil { reportError(err); return }
            setLanguage("lua") // EditorProps.Language or CodeBlockProps.Language
        })
})
```

The normal CLI `resources/` convention bundles the single pack file. Libraries, aliases, filename
inference, DiffView and Markdown's custom CodeBlock renderer share one registry. Loading a pack
refreshes already-mounted extension components, including late-loaded injections, without changing
source text, selections or scroll anchors.

## Rust

Enable `language-packs` for the Wasm loader; it includes `editor` but no grammars:

```toml
quickgui = { version = "0.1.4-next.4", features = ["language-packs"] }
```

```rust
let languages = quickgui::load_syntax_language_pack_bytes(
    include_bytes!("../resources/languages.qglang"),
)?;
let lua = quickgui::SyntaxLanguage::from_name("lua").unwrap();
let code = quickgui::CodeBlock::with_text("local answer = 42").with_language(lua);
```

`load_syntax_language_pack(path)` loads the same format from a file. Rust models created before a
registration can call `refresh_syntax_languages()` and invalidate their view; unchanged generations
do no parsing and retain their scroll state.

Rust apps that prefer statically linked grammars need only `features = ["editor"]` and a selected
grammar crate, without Wasmtime:

```rust
let mut definition = quickgui::SyntaxLanguageDefinition::new(
    "lua", tree_sitter_lua::LANGUAGE.into(), tree_sitter_lua::HIGHLIGHTS_QUERY,
);
definition.extensions = vec!["lua".into()];
let lua = quickgui::register_syntax_language(definition)?;
```

The optional `bundled-languages` feature explicitly opts into the built-in grammar set. Neither
`editor` nor the published editor extension enables it. Building Wasm-pack support from Rust
source requires CMake; prebuilt Go/TypeScript users do not need CMake. Tree-sitter 0.26.7 is pinned
because its Wasm dependencies are published and compatible with the project's Rust baseline.

## Lifetime and validation

Packs register atomically. A bad grammar, invalid query, duplicate/conflicting name or exceeded
limit leaves no partial registration behind. Identical loads reuse compiled grammars and do not
invalidate components. Registrations are immutable and process-wide; restart to replace a pack.

Limits: 64 MiB pack, 16 MiB per Wasm module, 128 languages, 32 aliases/extensions/filenames each,
1 MiB query text per language and 16 MiB total query text. The registry bounds accounted module
and query bytes to 256 MiB. The extension uses a sleeping worker and a 16-request queue. File
reads and compilation never happen while scrolling. Each parsing thread reuses its parser and
Wasm store; all stores share one Wasmtime engine.

Use packs from trusted sources: Wasm avoids loading arbitrary native libraries, but grammar and
query execution can still consume CPU and memory. Signed, hardened macOS applications using
Wasmtime need the `com.apple.security.cs.allow-jit` entitlement in their app entitlements; Bun
applications already require JIT support. Native grammar-library signing is not involved.
