# Retained Markdown

QuickGUI's opt-in `markdown` feature includes a native `Markdown` document. It parses CommonMark with tables,
strikethrough, and task-list extensions, then composes ordinary QuickGUI text and layout elements.
It does not use a webview or HTML renderer.

Keep one `Markdown` value for each logical document. Updating the source is controlled state:

```sh
cargo add quickgui --features markdown
```

```rust
use quickgui::{Color, Markdown, MarkdownStyle, View, ViewContext};

struct Article {
    body: String,
    markdown: Markdown,
    streaming: bool,
}

impl View for Article {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        self.markdown.set_streaming(self.streaming);
        self.markdown.set_style(
            MarkdownStyle::default()
                .link_color(Color::rgb8(96, 165, 250))
                .code_background(Color::rgb8(17, 24, 39)),
        );
        self.markdown.set_text(&self.body);
        self.markdown.element("article-markdown")
    }
}
```

`set_text` reports whether the source changed, whether the change was append-only, its byte reparse
boundary, and whether the input hit the source limit. Append-only updates retain settled parsed
blocks and flattened `StyledText`; only the unstable tail is reparsed and reshaped. While
`streaming` is true, incomplete emphasis, strike, code, and link markers are closed in a temporary
display tail without mutating the canonical source. Turning streaming off renders the exact final
parse.

## Supported presentation

The core renderer supports paragraphs, six heading levels, emphasis, strong text, strike,
inline/fenced code, links, block quotes, ordered/unordered/task lists, tables, rules, and image
fallback labels. `MarkdownStyle` controls semantic metrics and optional text, muted, link, code,
background, and border colors; surrounding layout and paint remain application-owned.

## Custom code blocks

`element_with_code_blocks` hands each parsed fenced or indented block to a Rust component factory.
The request contains the optional fence language, code, and a stable `ElementId`; parser ownership
and streaming reconciliation remain in the Markdown model. Keep custom component state in the
owning view keyed by that ID.

When both `markdown` and `editor` are enabled,
`element_with_highlighted_code_blocks(id, style, max_height)` is the retained convenience for the
new CodeBlock component. Each fence gets its own Tree-sitter language, virtual-scroll state, and
height cap.

Links are currently styled and selectable but do not own an open callback. Images currently render
their alt text and URL instead of fetching remote content. Those behaviors stay explicit so the
core never performs network access or opens URLs without application policy.

One document accepts at most 4 MiB of UTF-8 source, 32,768 top-level parsed blocks, 64 levels of
container nesting, 64 table columns, and the existing bounded rich-text highlight count. Input is
truncated only at a valid UTF-8 boundary. Settled documents add no timer, task, redraw loop, or idle
work.

Component properties have a separate 12 MiB JSON transport limit, so the native document's
4 MiB source limit also applies to Go and TypeScript. Ordinary string properties remain capped
at 1 MiB.

## Go component

The optional `extensions/markdown` package retains the core document and parser across streamed appends:

```sh
go get github.com/egoist/quickgui/extensions/markdown github.com/egoist/quickgui/extensions/editor
```

```go
import (
	"github.com/egoist/quickgui/extensions/editor"
	"github.com/egoist/quickgui/extensions/markdown"
	"github.com/egoist/quickgui/go/ui"
)

func Answer(answer string, streaming bool) *ui.Element {
	return markdown.View(markdown.Props{
		Value: answer,
		Streaming: streaming,
		CodeBlockComponent: editor.HighlightedCodeBlock,
		Theme: markdown.Theme{
			LinkColor: "#60a5fa",
			CodeBackground: "#090b10",
			CodeBorderColor: "#343843",
		},
	}).
		TextColor("#e4e4e7").
		FontSize(15).
		LineHeight(23)
}
```

Markdown children are not treated as source. Numeric/string children belong to `ui.Text`;
`markdown.View` consumes the complete document string in `Value`.
The complete streaming example is [AI Chat](../examples/ai-chat).

## TypeScript component

```sh
bun add @quickgui/extension-markdown @quickgui/extension-editor
```

```tsx
import { Markdown } from "@quickgui/extension-markdown";
import { HighlightedCodeBlock } from "@quickgui/extension-editor";

<Markdown content={answer()} streaming={streaming()} codeBlockComponent={HighlightedCodeBlock} />;
```

Add both installed packages to `extensions` in `quickgui.config.ts`:

```ts
extensions: ["@quickgui/extension-markdown", "@quickgui/extension-editor"]
```

To use another native code renderer, supply its `{ package, name, props }` component reference.
Markdown adds the parsed `value` and `language` properties; the renderer keeps its own state.
Without a custom renderer, only the Markdown extension is needed.

Return to the [documentation index](README.md).
