# Editor, CodeBlock, and diff view

QuickGUI's editor feature adds a compact native code editor, a selectable caret-free CodeBlock,
and a virtualized file diff. All three stay inside the existing renderer and event loop: there is
no webview, second renderer, polling loop, CGO bridge, or IPC process.

All three components are unstyled by default. Backgrounds, borders, rounding, padding, active-line
fills, syntax colors, and diff colors belong to the application. Unspecified text and gutter colors
inherit; classic `+`/`−` markers distinguish changes without a palette. The styled examples opt into
their dark appearance explicitly.

Language grammars are not bundled by default. Register a linked grammar or load a portable
[language pack](language-packs.md) containing your selected Wasm grammars and queries.

## Rust

Enable the opt-in crate feature:

```toml
[dependencies]
quickgui = { version = "0.1.6", features = ["editor"] }
```

Keep editor, code-block, and diff models on the owning view. The editor is controlled, like
`text_area`; its listener commits the native value back into the retained model. CodeBlock is
read-only and selectable without constructing text-input state.

```rust
use quickgui::{
    CodeBlock, CodeBlockStyle, DiffLayout, DiffView, DiffViewStyle, Editor, SyntaxLanguage, View,
    ViewContext, div,
};

struct SourceView {
    editor: Editor,
    code: CodeBlock,
    diff: DiffView,
}

impl View for SourceView {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let changed = cx.input_listener("source-editor", |view, value, cx| {
            if view.editor.set_text(value).changed {
                cx.invalidate();
            }
        });
        self.diff
            .set_style(DiffViewStyle::default().layout(DiffLayout::Split));

        div()
            .size_full()
            .flex_row()
            .child(self.editor.element("source-editor").on_input(changed).w_fraction(0.5))
            .child(self.code.element("source-code").w_fraction(0.25))
            .child(self.diff.element("source-diff").w_fraction(0.5))
    }
}
```

Register a grammar, then call `Editor::set_language(language)` for bounded Tree-sitter captures and supply
`SyntaxTheme` colors to paint them. The
editor reuses the core textarea's selection, IME composition, clipboard, undo/redo, accessibility,
caret, and both-axis scroll state. Tab/Shift-Tab and newline auto-indent are enabled by
`TextInputIndentation`. Line numbers are painted only for visible logical lines; they do not add one
layout element per source line. Focus keeps the native caret; active-line fills and borders are opt-in.

Rust callers can supply `EditorStyle::focus_border`.

### Syntax highlighting

The highlighter uses registered Tree-sitter grammars and their `highlights.scm` queries.
Query captures are flattened into eight stable public classes—keyword, literal, string, comment,
number, type, function, and metadata—and `SyntaxTheme` maps those classes to foreground-only
`HighlightStyle` values. Highlighting therefore never changes text metrics or layout. Markdown
and HTML use Tree-sitter injection queries for supported embedded languages. This is structural
syntax highlighting; QuickGUI does not start a language server or request semantic tokens.

`Editor` reparses when its source or selected language changes. A palette change only remaps the
retained capture table, and ordinary renders and scrolling reuse the same `StyledText`. CodeBlock
also parses on source/language changes, splits the resulting capture table across logical lines,
and lazily materializes styled text only for mounted rows. DiffView parses the visible old and new
file lanes when its document or syntax settings change, then lazily builds each mounted row and
merges inline addition/deletion backgrounds at shared byte boundaries. Scrolling never invokes
Tree-sitter.

Work is explicitly bounded: Tree-sitter examines at most 2 MiB of one syntax source, leaves any
individual line over 32 KiB plain, and emits no more than `MAX_SYNTAX_HIGHLIGHTS` paint ranges.
CodeBlock retains at most 4 MiB and 200,000 logical lines; bytes beyond the syntax limit remain
selectable with the default foreground. Grammar configurations are initialized once and parser
instances are reused per UI thread.

`DiffDocument::from_texts` runs the same histogram line-diff algorithm used by Zed's buffer diff,
then applies Git-like hunk post-processing. `DiffDocument::from_patch` parses captured Git or
ordinary unified patches without invoking Git or touching the filesystem. For large inputs, build
or parse the document on a background task and install the completed immutable model on the UI
thread.

`DiffViewStyle` supports:

- split or unified layout;
- bars, classic `+`/`−`, or no indicators;
- optional line backgrounds, line numbers, wrapping, and file header;
- collapsed-context separators and inline changed-range emphasis;
- caller-supplied diff and syntax palettes.

The diff body uses `ListState`, so mounted rows stay proportional to the viewport plus bounded
overscan. In split layout the panes remain fixed at equal widths, retain independent horizontal
offsets and overlay thumbs, and share one synchronized vertical offset with a single trailing
thumb. Source, line, and syntax metadata have explicit byte/count limits.

## Go

Import the optional binding package; it is absent from applications that do not import it.

```sh
go get github.com/egoist/quickgui/extensions/editor
```

```go
import (
    "github.com/egoist/quickgui/extensions/editor"
    "github.com/egoist/quickgui/go/ui"
)

source := editor.Editor(editor.EditorProps{
    Value:    code,
    Language: "go",
    OnChange: setCode,
    Props: ui.Props{Style: ui.Style().Width("100%").Height(360)},
})

codeBlock := editor.CodeBlock(editor.CodeBlockProps{
    Value: code,
    Language: "go",
    LineNumbers: true,
    Props: ui.Props{Style: ui.Style().Width("100%").Height(280)},
})

diff := editor.DiffView(editor.DiffViewProps{
    OldText: oldCode,
    NewText: newCode,
    OldPath: "before.go",
    NewPath: "after.go",
    Options: editor.DiffOptions{Layout: editor.DiffSplit},
    Props: ui.Props{Style: ui.Style().Width("100%").Height(420)},
})
```

Fields typed as `any` accept either their scalar value or a zero-argument reactive accessor.

## TypeScript

```sh
bun add @quickgui/extension-editor
```

Add `"@quickgui/extension-editor"` to `extensions` in `quickgui.config.ts`.

```tsx
import { CodeBlock, DiffView, Editor } from "@quickgui/extension-editor";

<Editor
  value={source()}
  language="typescript"
  onInput={(event) => setSource(event.value ?? "")}
  style={{ width: "100%", height: 360 }}
/>

<CodeBlock
  value={source()}
  language="typescript"
  lineNumbers
  style={{ width: "100%", height: 280 }}
/>

<DiffView
  oldText={before()}
  newText={after()}
  oldPath="before.ts"
  newPath="after.ts"
  options={{ layout: "split", indicators: "bars", lineNumbers: true }}
  style={{ width: "100%", height: 420 }}
/>
```

The API accepts a `patch` instead of the old/new pair, and exposes individual theme colors without
requiring CSS or HTML.

## Styling

Use the ordinary root style for the frame. Set `contentPadding` (`ContentPadding` in Go) for
document insets and `gutterPadding` (`GutterPadding`) for the fixed line-number lane. These default
to zero. Document insets scroll with the text and do not create a fixed clipping band.
`syntaxTheme`/`SyntaxTheme` supplies optional capture colors; omitted classes inherit text color.
`theme`/`Theme` supplies optional diff colors, and `presentation`/`Presentation` sets header height,
header padding, and gutter padding. None of these chooses a bundled appearance.

```tsx
<CodeBlock
  value="fn answer() -> u32 { 42 }"
  language="rust"
  contentPadding={{ left: 12, right: 12, vertical: 8 }}
  syntaxTheme={{ keyword: "#c678dd", function: "#61afef", number: "#d19a66" }}
  style={{
    width: "100%", height: 60, color: "#dcdfe6", backgroundColor: "#1e1f24",
    borderWidth: 1, borderColor: "#393c44", borderRadius: 6,
  }}
/>
```

The Rust equivalents are the caller-owned `EditorStyle`, `CodeBlockStyle`, and `DiffViewStyle`
values. Their default paint is empty; foreground and syntax colors use `Option<Color>` to
distinguish inheritance from an explicitly transparent color.

## Reference boundaries

The implementation follows Zed's separation of buffer/diff state from its visible-row editor
element and its Tree-sitter query model, and Waku's smaller proof that a shared native text engine
can add a paint-only gutter.
Pierre Diffs informed the split/unified options, bars/classic/none indicators, collapsed-context
presentation, header statistics, wrapping/line-number switches, and inline changed ranges. The
QuickGUI code uses its own retained elements, text engine, virtual list, and accessibility tree.
