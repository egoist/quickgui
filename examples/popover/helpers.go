package main

import "github.com/egoist/quickgui/go/ui"

var buttonStyle = ui.Style().
	Display("flex").
	Width("100%").
	Height(42).
	AlignItems("center").
	JustifyContent("center").
	PaddingLeft(16).
	PaddingRight(16).
	BackgroundColor("#2563eb").
	TextColor("#ffffff").
	BorderRadius(9).
	Cursor("default").
	AppRegion("no-drag").
	UserSelect("none").
	Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor("#3b82f6") })

func card(title, description string, children ui.Component) *ui.Element {
	return ui.View().
		Children(

			ui.Text(title).FontSize(17).FontWeight(700),
			ui.Text(
				description,
			).
				TextColor("#9ba8bc").
				FontSize(13).
				LineHeight(19),
			children,
		).
		Display("flex").
		FlexDirection("column").
		Flex(1).
		MinWidth(0).
		Gap(14).
		Padding(20).
		BackgroundColor("#151a23").
		BorderColor("#30394a").
		BorderWidth(1).
		BorderRadius(12)
}

func content(kind, description string, close func()) *ui.Element {
	count, setCount := ui.CreateSignal(0)
	actionStyle := ui.Style().
		BackgroundColor("#30394a").
		BorderColor("#465166").
		BorderWidth(1).
		Flex(1).
		Width(0).
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor("#465166") })
	return ui.View().
		Children(

			ui.View().
				Children(

					ui.Text(
						kind,
					).
						TextColor("#93c5fd").
						FontSize(12).
						FontWeight(700),
					ui.Text(
						"Interactive popover content",
					).
						FontSize(19).
						LineHeight(24).
						FontWeight(700),
					ui.Text(
						description,
					).
						TextColor("#aeb8c9").
						FontSize(13).
						LineHeight(19),
				).
				Display("flex").
				FlexDirection("column").
				Gap(6),
			ui.View().
				Children(

					ui.Button().
						Style(actionStyle).
						Child(ui.Text("Count: ", count())).
						Style(buttonStyle).
						OnClick(func() { setCount(count() + 1) }),

					ui.Button().Style(actionStyle).Child("Close").Style(buttonStyle).OnClick(close),
				).
				Display("flex").
				Gap(10),
		).
		Display("flex").
		FlexDirection("column").
		Width("100%").
		Height("100%").
		Gap(14).
		Padding(20).
		BackgroundColor("#151a23").
		TextColor("#f5f7fb").
		BorderColor("#3b4558").
		BorderWidth(1).
		BorderRadius(12)
}
