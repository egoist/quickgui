// Package editor provides native editor, code-block, and diff components.
package editor

import (
	"encoding/json"
	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func init() { host.RequireExtension("editor", "0.1.6") }

// HighlightedCodeBlock renders Markdown fences with this extension's CodeBlock component.
var HighlightedCodeBlock = ui.ComponentReference{
	Package: "editor",
	Name:    "code-block",
	Props:   map[string]any{},
}

// ContentPadding sets document insets that scroll with code.
type ContentPadding struct {
	Left     any `json:"left,omitempty"`
	Right    any `json:"right,omitempty"`
	Vertical any `json:"vertical,omitempty"`
}
type GutterPadding struct {
	Left  any `json:"left,omitempty"`
	Right any `json:"right,omitempty"`
}
type DiffPresentation struct {
	HeaderHeight  any `json:"headerHeight,omitempty"`
	HeaderPadding any `json:"headerPadding,omitempty"`
	GutterPadding any `json:"gutterPadding,omitempty"`
}

// EditorProps configure a controlled native code editor.
type EditorProps struct {
	ui.Props
	Value                 any
	Language              any
	LineNumbers           any
	TabSize               any
	InsertSpaces          any
	AutoIndent            any
	ReadOnly              any
	ContentPadding        ContentPadding
	GutterPadding         GutterPadding
	GutterBackground      any
	GutterColor           any
	ActiveLineBackground  any
	ActiveLineNumberColor any
	SyntaxTheme           SyntaxTheme
	OnChange              func(string)
}

type DiffLayout string

const (
	DiffSplit   DiffLayout = "split"
	DiffUnified DiffLayout = "unified"
)

type DiffIndicators string

const (
	DiffBars    DiffIndicators = "bars"
	DiffClassic DiffIndicators = "classic"
	DiffNone    DiffIndicators = "none"
)

// DiffOptions select the Pierre-style presentation without changing diff data.
type DiffOptions struct {
	Layout      DiffLayout
	Indicators  DiffIndicators
	Backgrounds any
	LineNumbers any
	Wrap        any
	FileHeader  any
	Language    any
}

// DiffTheme overrides individual native diff colors.
type DiffTheme struct {
	MutedColor              any
	AddedBackground         any
	RemovedBackground       any
	AddedGutterBackground   any
	RemovedGutterBackground any
	AddedColor              any
	RemovedColor            any
	LineNumberColor         any
	HunkBackground          any
	HunkColor               any
	HeaderBackground        any
	InlineAddedBackground   any
	InlineRemovedBackground any
}

// SyntaxTheme overrides Tree-sitter foreground colors shared by editors, diffs, and code blocks.
type SyntaxTheme struct {
	Keyword  any `json:"keyword,omitempty"`
	Literal  any `json:"literal,omitempty"`
	String   any `json:"string,omitempty"`
	Comment  any `json:"comment,omitempty"`
	Number   any `json:"number,omitempty"`
	Type     any `json:"type,omitempty"`
	Function any `json:"function,omitempty"`
	Metadata any `json:"metadata,omitempty"`
}

// CodeBlockProps configure a selectable, caret-free source-code surface.
type CodeBlockProps struct {
	ui.Props
	ContentPadding   ContentPadding
	GutterPadding    GutterPadding
	Value            any
	Language         any
	LineNumbers      any
	Wrap             any
	GutterBackground any
	GutterColor      any
	SyntaxTheme      SyntaxTheme
}

// DiffViewProps accept either Patch or a complete OldText/NewText pair. Patch wins when both are
// supplied. Values may be accessors and update the retained native model without replacing it.
type DiffViewProps struct {
	ui.Props
	Patch        any
	OldText      any
	NewText      any
	OldPath      any
	NewPath      any
	Options      DiffOptions
	Presentation DiffPresentation
	Theme        DiffTheme
	SyntaxTheme  SyntaxTheme
}

func syntaxProperties(theme SyntaxTheme) map[string]any {
	return map[string]any{"keyword": theme.Keyword, "literal": theme.Literal, "string": theme.String, "comment": theme.Comment, "number": theme.Number, "type": theme.Type, "function": theme.Function, "metadata": theme.Metadata}
}

func Editor(props EditorProps) *ui.Element {
	previous := props.Props.OnComponentChange
	props.Props.OnComponentChange = func(event *native.Event) {
		var payload struct {
			Kind  string
			Value string
		}
		if json.Unmarshal([]byte(event.Value), &payload) == nil && payload.Kind == "input" && props.OnChange != nil {
			props.OnChange(payload.Value)
		}
		if previous != nil {
			previous(event)
		}
	}
	return ui.ExtensionComponent(
		"editor",
		"editor",
		map[string]any{
			"value": props.Value, "language": props.Language, "lineNumbers": props.LineNumbers, "tabSize": props.TabSize,
			"insertSpaces": props.InsertSpaces, "autoIndent": props.AutoIndent, "readOnly": props.ReadOnly,
			"contentPadding":   map[string]any{"left": props.ContentPadding.Left, "right": props.ContentPadding.Right, "vertical": props.ContentPadding.Vertical},
			"gutterPadding":    map[string]any{"left": props.GutterPadding.Left, "right": props.GutterPadding.Right},
			"gutterBackground": props.GutterBackground, "gutterColor": props.GutterColor,
			"activeLineBackground": props.ActiveLineBackground, "activeLineNumberColor": props.ActiveLineNumberColor,
			"syntaxTheme": syntaxProperties(props.SyntaxTheme),
		},
		props.Props,
	)
}

func CodeBlock(props CodeBlockProps) *ui.Element {
	return ui.ExtensionComponent(
		"editor",
		"code-block",
		map[string]any{
			"value": props.Value, "language": props.Language, "lineNumbers": props.LineNumbers, "wrap": props.Wrap,
			"contentPadding":   map[string]any{"left": props.ContentPadding.Left, "right": props.ContentPadding.Right, "vertical": props.ContentPadding.Vertical},
			"gutterPadding":    map[string]any{"left": props.GutterPadding.Left, "right": props.GutterPadding.Right},
			"gutterBackground": props.GutterBackground, "gutterColor": props.GutterColor, "syntaxTheme": syntaxProperties(props.SyntaxTheme),
		},
		props.Props,
	)
}

func DiffView(props DiffViewProps) *ui.Element {
	return ui.ExtensionComponent(
		"editor",
		"diff-view",
		map[string]any{
			"patch": props.Patch, "oldText": props.OldText, "newText": props.NewText, "oldPath": props.OldPath, "newPath": props.NewPath,
			"options":      map[string]any{"layout": props.Options.Layout, "indicators": props.Options.Indicators, "backgrounds": props.Options.Backgrounds, "lineNumbers": props.Options.LineNumbers, "wrap": props.Options.Wrap, "fileHeader": props.Options.FileHeader, "language": props.Options.Language},
			"presentation": map[string]any{"headerHeight": props.Presentation.HeaderHeight, "headerPadding": props.Presentation.HeaderPadding, "gutterPadding": props.Presentation.GutterPadding},
			"theme":        map[string]any{"mutedColor": props.Theme.MutedColor, "addedBackground": props.Theme.AddedBackground, "removedBackground": props.Theme.RemovedBackground, "addedGutterBackground": props.Theme.AddedGutterBackground, "removedGutterBackground": props.Theme.RemovedGutterBackground, "addedColor": props.Theme.AddedColor, "removedColor": props.Theme.RemovedColor, "lineNumberColor": props.Theme.LineNumberColor, "hunkBackground": props.Theme.HunkBackground, "hunkColor": props.Theme.HunkColor, "headerBackground": props.Theme.HeaderBackground, "inlineAddedBackground": props.Theme.InlineAddedBackground, "inlineRemovedBackground": props.Theme.InlineRemovedBackground},
			"syntaxTheme":  syntaxProperties(props.SyntaxTheme),
		},
		props.Props,
	)
}
