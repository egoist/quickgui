// Package markdown provides a retained native Markdown component.
package markdown

import (
	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/ui"
)

// Props configure a retained Markdown document.
type Props struct {
	ui.Props
	Value     any
	Streaming any
	// A component reference receives each parsed fence's value and language.
	CodeBlockComponent any
	CodeBlockMaxHeight any
	Theme              Theme
}

// Theme configures Markdown-specific colors and metrics.
type Theme struct {
	LinkColor       any
	MutedColor      any
	CodeColor       any
	CodeBackground  any
	CodeBorderColor any
	BlockGap        any
	CodeFontSize    any
}

func View(props Props) *ui.Element {
	return ui.ExtensionComponent(
		"markdown",
		"markdown",
		map[string]any{
			"value": props.Value, "streaming": props.Streaming, "codeBlockComponent": props.CodeBlockComponent, "codeBlockMaxHeight": props.CodeBlockMaxHeight,
			"theme": map[string]any{"linkColor": props.Theme.LinkColor, "mutedColor": props.Theme.MutedColor, "codeColor": props.Theme.CodeColor, "codeBackground": props.Theme.CodeBackground, "codeBorderColor": props.Theme.CodeBorderColor, "blockGap": props.Theme.BlockGap, "codeFontSize": props.Theme.CodeFontSize},
		},
		props.Props,
	)
}
func init() { host.RequireExtension("markdown", "0.1.6") }
