package main

import (
	"fmt"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/ui"
)

func main() {
	native.Run(func() {
		openMainWindow()
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				openMainWindow()
			}
		})
	})
}

func openMainWindow() {
	native.NewWindow(native.WindowOptions{
		Title:                "QuickGUI Counter",
		Width:                760,
		Height:               520,
		MinimumWidth:         520,
		MinimumHeight:        360,
		Background:           "#090d16",
		TitleBarStyle:        "hiddenInset",
		TrafficLightPosition: &native.Point{X: 16, Y: 13},
		Renderer:             ui.CreateRenderer(Counter),
	})
}

func openDetailsWindow() {
	native.NewWindow(native.WindowOptions{
		Title:         "Dynamic QuickGUI window",
		Width:         420,
		Height:        260,
		MinimumWidth:  320,
		MinimumHeight: 200,
		Background:    "#111827",
		Renderer: ui.CreateRenderer(func() *native.Node {
			window := native.CurrentWindow()
			return ui.View(ui.Props{
				Style: ui.Style{
					Display: "flex", FlexDirection: "column", Width: "100%", Height: "100%",
					JustifyContent: "center", Gap: 16, Padding: 28,
					BackgroundColor: "#111827", Color: "#e2e8f0",
				},
				Children: []any{
					ui.Text(ui.Props{Style: ui.Style{FontSize: 22, FontWeight: 700}, Children: "Created while the app is running"}),
					ui.Text(ui.Props{Style: ui.Style{Color: "#94a3b8", LineHeight: 21}, Children: "This window has its own retained tree and native lifecycle."}),
					ui.Button(ui.Props{
						OnClick: func(*native.Event) { window.Close() },
						Style: ui.Style{
							Display: "flex", Height: 40, AlignItems: "center", JustifyContent: "center",
							BackgroundColor: "#334155", BorderRadius: 9, Cursor: "default",
						},
						Children: "Close window",
					}),
				},
			})
		}),
	})
}

func Counter() *native.Node {
	count, setCount := ui.CreateSignal(0)
	return ui.View(ui.Props{
		Style: ui.Style{
			Display: "flex", FlexDirection: "column", Width: "100%", Height: "100%",
			BackgroundColor: "#090d16", Color: "#e2e8f0",
		},
		Children: []any{
			ui.View(ui.Props{
				Style: ui.Style{
					Display: "flex", Height: 52, FlexShrink: 0, AlignItems: "center", JustifyContent: "center",
					AppRegion: "drag", BorderColor: "#1e293b", BorderWidth: 1,
				},
				Children: ui.Text(ui.Props{Style: ui.Style{FontWeight: 600}, Children: "QuickGUI · Go"}),
			}),
			ui.View(ui.Props{
				Style: ui.Style{
					Display: "flex", Flex: 1, MinHeight: 0, AlignItems: "center", JustifyContent: "center", Padding: 32,
				},
				Children: ui.View(ui.Props{
					Style: ui.Style{
						Display: "flex", FlexDirection: "column", Width: 420, Gap: 18, Padding: 28,
						BackgroundColor: "#111827", BorderColor: "#334155", BorderWidth: 1, BorderRadius: 16,
					},
					Children: []any{
						ui.Text(ui.Props{Style: ui.Style{FontSize: 28, LineHeight: 36, FontWeight: 700}, Children: "Fine-grained native UI"}),
						ui.Text(ui.Props{
							Style:    ui.Style{Color: "#94a3b8", FontSize: 14, LineHeight: 21},
							Children: "Signals update only the changed text node. The application is ordinary Go, and QuickGUI retains layout, sleeps while clean, and redraws once per mutation batch.",
						}),
						ui.Text(ui.Props{
							Style:    ui.Style{Color: "#bfdbfe", FontSize: 20, FontWeight: 600},
							Children: func() string { return fmt.Sprintf("Count: %d", count()) },
						}),
						ui.Show(func() bool { return count() >= 5 }, func() *native.Node {
							return ui.Text(ui.Props{
								Style:    ui.Style{Color: "#fbbf24", FontSize: 14},
								Children: "Five or more clicks: the row above was created on demand.",
							})
						}),
						ui.Button(ui.Props{
							OnClick: func(*native.Event) { setCount(count() + 1) },
							Style: ui.Style{
								Display: "flex", Height: 44, AlignItems: "center", JustifyContent: "center",
								BackgroundColor: "#2563eb", Color: "white", BorderRadius: 9, Cursor: "default",
								AppRegion: "no-drag", UserSelect: "none",
								Hover: &ui.Style{BackgroundColor: "#3b82f6"},
							},
							Children: "Increment",
						}),
						ui.Button(ui.Props{
							OnClick: func(*native.Event) { openDetailsWindow() },
							Style: ui.Style{
								Display: "flex", Height: 44, AlignItems: "center", JustifyContent: "center",
								BackgroundColor: "#334155", Color: "white", BorderRadius: 9, Cursor: "default",
								AppRegion: "no-drag", UserSelect: "none",
							},
							Children: "Open window",
						}),
					},
				}),
			}),
		},
	})
}
