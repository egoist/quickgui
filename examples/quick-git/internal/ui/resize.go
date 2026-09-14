package ui

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	gui "github.com/egoist/quickgui/go/ui"
)

func resizablePanel(label string, width func() float64, setWidth func(float64), minimum, maximum float64, children gui.Component, style gui.StyleBuilder) *native.Node {
	index := 0
	splitter75 := gui.NewSplitter(gui.SplitterRootProps{
		Value: func() []float64 { // Only the fixed panel is mounted. The second size supplies its remaining
			// resize allowance; the enclosing flex layout owns the rest of the window.
			// Rust updates the pane directly during a drag, then reports sizes to Go
			// asynchronously for persistence. No pointer move waits for a Go width echo.

			size := width()
			return []float64{size, maximum - size}
		},
		Panes: []gui.SplitterPaneDeclaration{{Min: &minimum}, {}},
		OnSizesChange: func(sizes []float64, _ *native.Event) {
			if len(sizes) == 2 {
				setWidth(sizes[0])
			}
		},
		PartProps: gui.PartProps{Style: gui.Style().Display("flex").FlexShrink(0).MinWidth(0).MinHeight(0)},
	})
	return splitter75.Root().
		Children(func() *native.Node {
			return gui.Fragment([]*native.Node{
				splitter75.PaneWith(gui.SplitterPaneProps{
					Index:     &index,
					PartProps: gui.PartProps{Style: style},
				}).
					Children(children).
					NativeNode(),
				splitter75.HandleWith(gui.SplitterPaneProps{
					Index: &index,
					PartProps: gui.PartProps{
						AriaLabel: label,
						Ref: func(node *native.Node) {
							native.SetNumber(node, protocol.HitSlopLeft, 4)
							native.SetNumber(node, protocol.HitSlopRight, 4)
						},
						Style: gui.Style().
							Width(1).
							FlexShrink(0).
							Cursor("col-resize").
							AppRegion("no-drag").
							BackgroundColor(UseApp().Theme().Border),
					},
				}).
					NativeNode(),
			})
		}).
		NativeNode()
}
