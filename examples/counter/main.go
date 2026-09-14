package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		openMainWindow()
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				openMainWindow()
			}
		})
	}); err != nil {
		log.Fatal(err)
	}
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
		Component:            Counter,
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
		Component: func() *ui.Element {
			window := native.CurrentWindow()
			return ui.View().
				Children(

					ui.Text(
						"Created while the app is running",
					).
						FontSize(22).
						FontWeight(700),

					ui.Text(
						"This window has its own retained tree and native lifecycle.",
					).
						TextColor("#94a3b8").
						LineHeight(21),

					ui.Button().
						Child("Close window").
						OnClick(func() { window.Close() }).
						Display("flex").
						Height(40).
						AlignItems("center").
						JustifyContent("center").
						BackgroundColor("#334155").
						BorderRadius(9).
						Cursor("default"),
				).
				Display("flex").
				FlexDirection("column").
				Width("100%").
				Height("100%").
				JustifyContent("center").
				Gap(16).
				Padding(28).
				BackgroundColor("#111827").
				TextColor("#e2e8f0")

		},
	})
}

func Counter() *ui.Element {
	count, setCount := ui.CreateSignal(0)
	return ui.View().
		Children(
			ui.View().
				Child(
					ui.Text("QuickGUI · Go").FontWeight(600),
				).
				Display("flex").
				Height(52).
				FlexShrink(0).
				AlignItems("center").
				JustifyContent("center").
				AppRegion("drag").
				BorderColor("#1e293b").
				BorderWidth(1),

			ui.View().
				Child(

					ui.View().
						Children(

							ui.Text(
								"Fine-grained native UI",
							).
								FontSize(28).
								LineHeight(36).
								FontWeight(700),

							ui.Text(
								"Signals update only the changed text node. The application is ordinary Go, "+
									"and QuickGUI retains layout, sleeps while clean, and redraws once per mutation batch.",
							).
								TextColor("#94a3b8").
								FontSize(14).
								LineHeight(21),

							CountLabel(count()),

							ui.Show(
								count() >= 5,
								func() *ui.Element {
									return ui.Text(
										"Five or more clicks: the row above was created on demand.",
									).
										TextColor("#fbbf24").
										FontSize(14)

								},
							),
							ui.Button().
								Child("Increment").
								OnClick(func() { setCount(count() + 1) }).
								Display("flex").
								Height(44).
								AlignItems("center").
								JustifyContent("center").
								BackgroundColor("#2563eb").
								TextColor("white").
								BorderRadius(9).
								Cursor("default").
								AppRegion("no-drag").
								UserSelect("none").
								Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
									return s.BackgroundColor("#3b82f6")
								}),

							ui.Button().
								Child("Open window").
								OnClick(func() { openDetailsWindow() }).
								Display("flex").
								Height(44).
								AlignItems("center").
								JustifyContent("center").
								BackgroundColor("#334155").
								TextColor("white").
								BorderRadius(9).
								Cursor("default").
								AppRegion("no-drag").
								UserSelect("none"),
						).
						Display("flex").
						FlexDirection("column").
						Width(420).
						Gap(18).
						Padding(28).
						BackgroundColor("#111827").
						BorderColor("#334155").
						BorderWidth(1).
						BorderRadius(16),
				).
				Display("flex").
				Flex(1).
				MinHeight(0).
				AlignItems("center").
				JustifyContent("center").
				Padding(32),
		).
		Display("flex").
		FlexDirection("column").
		Width("100%").
		Height("100%").
		BackgroundColor("#090d16").
		TextColor("#e2e8f0")

}

func CountLabel(count int) *ui.Element {
	return ui.Text("Count: ", count).
		TextColor("#bfdbfe").
		When(count >= 5, ui.Style().TextColor("#fbbf24")).
		FontSize(20).
		FontWeight(600)
}
