package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func run(options native.WindowOptions, component ui.Component) {
	options.Background = "#0b0e14"
	options.TitleBarStyle = "hiddenInset"
	options.TrafficLightPosition = &native.Point{X: 16, Y: 14}
	options.Component = func() *ui.Element {
		return ui.View().
			Children(

				ui.View().
					Child(

						ui.Text(
							options.Title,
						).
							FontSize(14).
							FontWeight(600),
					).
					Display("flex").
					Height(52).
					FlexShrink(0).
					AlignItems("center").
					JustifyContent("center").
					AppRegion("drag").
					BorderColor("#1f2530").
					BorderBottomWidth(1),
				ui.View().
					Child(
						component,
					).
					Display("flex").
					Flex(1).
					MinHeight(0).
					Padding(36).
					AlignItems("center").
					JustifyContent("center").
					OverflowY("auto"),
			).
			Display("flex").
			FlexDirection("column").
			Width("100%").
			Height("100%").
			BackgroundColor("#0b0e14").
			TextColor("#f4f7fb")
	}
	if err := native.Run(func() {
		open := func() { native.NewWindow(options) }
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				open()
			}
		})
		open()
	}); err != nil {
		log.Fatal(err)
	}
}

var panelStyle = ui.Style().
	Display("flex").
	FlexDirection("column").
	Width("100%").
	MaxWidth(480).
	FlexShrink(0).
	Gap(20).
	Padding(28).
	BackgroundColor("#151922").
	BorderColor("#2c3442").
	BorderWidth(1).
	BorderRadius(16)

var buttonStyle = ui.Style().
	Display("flex").
	Height(42).
	AlignItems("center").
	JustifyContent("center").
	PaddingLeft(16).
	PaddingRight(16).
	BackgroundColor("#262c38").
	TextColor("#f4f7fb").
	BorderColor("#3a4353").
	BorderWidth(1).
	BorderRadius(9).
	Cursor("default").
	AppRegion("no-drag").
	UserSelect("none").
	Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor("#30394a") })

func button(label string, disabled func() bool, click func()) *ui.Element {
	return ui.Button().Child(label).Style(buttonStyle).Disabled(disabled()).OnClick(click)
}

func dialogStatus(status func() string, pending func() bool) *ui.Element {
	return ui.View().
		Child(

			ui.Text(
				status(),
			).
				FontSize(13).
				LineHeight(19).
				TextAlign("center").
				UserSelect("text").
				TextColor(func() string {
					if pending() {
						return "#c7d2fe"
					}
					return "#aeb9c9"
				}),
		).
		Display("flex").
		MinHeight(64).
		AlignItems("center").
		JustifyContent("center").
		Padding(14).
		BackgroundColor("#0f131a").
		BorderColor("#252c38").
		BorderWidth(1).
		BorderRadius(9)
}
