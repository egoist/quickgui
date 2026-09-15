package main

import (
	"fmt"
	"github.com/egoist/quickgui/extensions/editor"
	"github.com/egoist/quickgui/extensions/markdown"
	"github.com/egoist/quickgui/extensions/terminal"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
	"log"
	"path/filepath"
	"strings"
)

func main() {
	native.App.OnReady(func() {
		native.App.GetPaths(func(paths *native.AppPaths, err error) {
			if err != nil {
				log.Print(err)
				return
			}
			editor.LoadLanguagePack(filepath.Join(paths.ResourceDir, "languages.qglang"), func(_ []string, err error) {
				if err != nil {
					log.Printf("Demo Rust grammar: %v (run bun run pack:languages)", err)
				}
			})
		})
	})
	if err := native.Run(func() {
		native.NewWindow(native.WindowOptions{
			Title:                "QuickGUI Extensions",
			Width:                1200,
			Height:               860,
			MinimumWidth:         800,
			MinimumHeight:        600,
			TitleBarStyle:        "hiddenInset",
			TrafficLightPosition: &native.Point{X: 16, Y: 16},
			Background:           "#101114",
			Component:            Demo,
		})
	}); err != nil {
		log.Fatal(err)
	}
}

const initial = "pub fn greeting(name: &str) -> String {\n    format!(\"Hello from QuickGUI, {name}!\")\n}\n\nfn main() {\n    let audience = \"editor\";\n    println!(\"{}\", greeting(audience));\n}\n"

func Demo() *ui.Element {
	source, setSource := ui.CreateSignal(initial)
	long := func() {
		var text strings.Builder
		for i := 0; i < 150; i++ {
			fmt.Fprintf(&text, "let value_%d = \"A long source line that scrolls horizontally while the line gutter stays fixed at the left edge of the editor and the split diff pane\";\n", i)
		}
		setSource(text.String())
	}
	common := ui.Style().Width("100%").MinWidth(0).MinHeight(0)
	frame := ui.Style().
		BackgroundColor("#1e1f24").
		TextColor("#dcdfe6").
		BorderWidth(1).
		BorderColor("#393c44").
		BorderRadius(6).
		FontSize(13).
		LineHeight(20)
	syntax := editor.SyntaxTheme{
		Keyword: "#c678dd", Literal: "#d19a66", String: "#98c379", Comment: "#5c6370",
		Number: "#d19a66", Type: "#e5c07b", Function: "#61afef", Metadata: "#56b6c2",
	}
	contentPadding := editor.ContentPadding{Vertical: 8}
	gutterPadding := editor.GutterPadding{Left: 8, Right: 10}
	codeFence := editor.HighlightedCodeBlock
	codeFence.Props = map[string]any{
		"style":          map[string]any{"backgroundColor": "#1e1f24", "color": "#dcdfe6", "borderWidth": 1, "borderColor": "#393c44", "borderRadius": 6},
		"contentPadding": editor.ContentPadding{Left: 12, Right: 12, Vertical: 8},
		"syntaxTheme":    syntax,
	}
	edit := editor.Editor(editor.EditorProps{Value: source, Language: "rust", LineNumbers: true, OnChange: setSource, SyntaxTheme: syntax, ContentPadding: contentPadding, GutterPadding: gutterPadding, GutterColor: "#686d78", ActiveLineNumberColor: "#d6d9e0", ActiveLineBackground: "#ffffff08"}).Style(common.Merge(frame)).Flex(1)
	code := editor.CodeBlock(editor.CodeBlockProps{Value: source, Language: "rust", LineNumbers: true, SyntaxTheme: syntax, ContentPadding: contentPadding, GutterPadding: gutterPadding, GutterColor: "#686d78"}).Style(common.Merge(frame)).Height(240)
	diff := editor.DiffView(editor.DiffViewProps{OldPath: "before.rs", NewPath: "after.rs", OldText: initial, NewText: source, Options: editor.DiffOptions{Layout: editor.DiffSplit, Indicators: editor.DiffBars, LineNumbers: true}, SyntaxTheme: syntax,
		Presentation: editor.DiffPresentation{HeaderHeight: 38, HeaderPadding: 12, GutterPadding: 8},
		Theme: editor.DiffTheme{HeaderBackground: "#141518", LineNumberColor: "#717680", HunkBackground: "#2d2f34", HunkColor: "#a8adb8", MutedColor: "#7b808b",
			AddedBackground: "#00c28110", RemovedBackground: "#ff353f10", AddedColor: "#00c281", RemovedColor: "#ff353f",
			InlineAddedBackground: "#00c28137", InlineRemovedBackground: "#ff353f37"},
	}).Style(common.Merge(frame).BackgroundColor("#18191d").BorderColor("#30333a").BorderRadius(7)).Flex(1)
	md := markdown.View(markdown.Props{Value: "### Markdown\n\nThe fenced block uses the editor extension.\n\n```rust\nfn answer() -> u32 { 42 }\n```", CodeBlockComponent: codeFence}).Style(common)
	console := terminal.View(terminal.Props{Program: "/bin/sh", Args: []string{"-c", "printf 'Terminal extension ready\\n'; exec /bin/sh"}, Props: ui.Props{Style: common.Height(180).BorderRadius(6).Overflow("hidden")}})
	return ui.View().
		Width("100%").
		Height("100%").
		FlexCol().
		TextColor("#dcdfe6").
		Children(
			ui.View().
				Height(52).
				FlexShrink(0).
				FlexRow().
				AlignItems("center").
				PaddingLeft(92).
				PaddingRight(16).
				Gap(12).
				AppRegion("drag").
				Children(
					ui.Text("Editor + Diff View").FontWeight(600),
					ui.Button().Child("Long document").OnClick(long).Padding(7).AppRegion("no-drag"),
					ui.Button().
						Child("Reset").
						OnClick(func() { setSource(initial) }).
						Padding(7).
						AppRegion("no-drag"),
				),
			ui.View().
				Flex(1).
				MinHeight(0).
				Padding(12).
				Gap(12).
				FlexRow().
				Children(
					ui.View().
						Width("50%").
						MinWidth(0).
						MinHeight(0).
						Gap(12).
						FlexCol().
						Children(
							edit,
							code,
						),
					ui.View().
						Width("50%").
						MinWidth(0).
						MinHeight(0).
						Gap(12).
						FlexCol().
						Children(
							diff,
							md,
							console,
						),
				),
		)
}
