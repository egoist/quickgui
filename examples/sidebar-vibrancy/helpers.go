package main

import "github.com/egoist/quickgui/go/ui"

type choice struct{ value, label string }

var materials = []choice{
	{"appearance-based", "Appearance based"},
	{"titlebar", "Titlebar"},
	{"selection", "Selection"},
	{"menu", "Menu"},
	{"popover", "Popover"},
	{"sidebar", "Sidebar"},
	{"header", "Header"},
	{"sheet", "Sheet"},
	{"window", "Window"},
	{"hud", "HUD"},
	{"fullscreen-ui", "Fullscreen UI"},
	{"tooltip", "Tooltip"},
	{"content", "Content"},
	{"under-window", "Under window"},
	{"under-page", "Under page"},
}

var effectStates = []choice{
	{"followWindow", "Auto"},
	{"active", "Active"},
	{"inactive", "Inactive"},
}

func checkmark() *ui.Element {
	return ui.SVG().
		Value(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#2563eb" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12 4 4L19 6"/></svg>`).
		Width(14).
		Height(14).
		FlexShrink(0).
		TextColor("#2563eb")
}

func readout(label string, value any) *ui.Element {
	return ui.View().
		Children(

			ui.Text(label).TextColor("#8a94a6").FontSize(11),
			ui.Text(
				value,
			).
				TextColor("#354056").
				FontSize(11).
				FontWeight(700),
		).
		Display("flex").
		FlexDirection("column").
		Flex(1).
		MinWidth(0).
		Gap(3).
		Padding(12).
		BackgroundColor("#f8fafc").
		BorderColor("#e2e8f0").
		BorderWidth(1).
		BorderRadius(9)
}
