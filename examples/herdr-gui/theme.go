package main

import (
	"github.com/egoist/quickgui/extensions/terminal"
	"github.com/egoist/quickgui/go/ui"
)

type theme struct {
	App             string
	Sidebar         string
	Surface         string
	Terminal        string
	TerminalText    string
	TerminalCursor  string
	Hover           string
	Active          string
	Selected        string
	SelectedStrong  string
	Border          string
	BorderStrong    string
	Text            string
	TextSecondary   string
	TextTertiary    string
	TextGhost       string
	Accent          string
	AccentHover     string
	AccentWash      string
	AccentText      string
	Working         string
	WorkingWash     string
	Success         string
	SuccessWash     string
	Warning         string
	WarningWash     string
	Danger          string
	Scrim           string
	ErrorBackground string
	TerminalPalette terminal.Palette
}

var lightTheme = theme{
	App:             "#f7f7f6",
	Sidebar:         "#f1f1ef",
	Surface:         "#ffffff",
	Hover:           "#e9e9e6",
	Active:          "#dfdfdc",
	Selected:        "#e5e5e1",
	SelectedStrong:  "#dcdcd7",
	Border:          "#ddddda",
	BorderStrong:    "#cacac6",
	Text:            "#252524",
	TextSecondary:   "#62625f",
	TextTertiary:    "#858581",
	TextGhost:       "#a7a7a2",
	Accent:          "#cf684c",
	AccentHover:     "#bc5a41",
	AccentWash:      "#f5e7e2",
	AccentText:      "#ffffff",
	Working:         "#b27b20",
	WorkingWash:     "#f4ebd9",
	Success:         "#2d9860",
	SuccessWash:     "#e1f1e8",
	Warning:         "#b27b20",
	WarningWash:     "#f4ebd9",
	Danger:          "#c94e43",
	Scrim:           "#18181655",
	ErrorBackground: "#f7e6e2",
	Terminal:        "#ffffff",
	TerminalText:    "#1f2328",
	TerminalCursor:  "#0969da",
	TerminalPalette: terminal.Palette{
		"#24292f",
		"#cf222e",
		"#116329",
		"#4d2d00",
		"#0969da",
		"#8250df",
		"#1b7c83",
		"#6e7781",
		"#57606a",
		"#a40e26",
		"#1a7f37",
		"#633c01",
		"#218bff",
		"#a475f9",
		"#3192aa",
		"#8c959f",
	},
}
var darkTheme = theme{
	App:             "#191918",
	Sidebar:         "#161615",
	Surface:         "#20201f",
	Hover:           "#242423",
	Active:          "#2b2b29",
	Selected:        "#282826",
	SelectedStrong:  "#30302d",
	Border:          "#2c2c2a",
	BorderStrong:    "#3c3c39",
	Text:            "#e4e4e1",
	TextSecondary:   "#aaa9a4",
	TextTertiary:    "#7e7d78",
	TextGhost:       "#595955",
	Accent:          "#e17b5d",
	AccentHover:     "#ec8a6d",
	AccentWash:      "#352520",
	AccentText:      "#1d110d",
	Working:         "#dfb25f",
	WorkingWash:     "#332a1c",
	Success:         "#67c68a",
	SuccessWash:     "#1d3125",
	Warning:         "#dfb25f",
	WarningWash:     "#332a1c",
	Danger:          "#e06e66",
	Scrim:           "#050505bb",
	ErrorBackground: "#321d1b",
	Terminal:        "#0d1117",
	TerminalText:    "#e6edf3",
	TerminalCursor:  "#2f81f7",
	TerminalPalette: terminal.Palette{
		"#484f58",
		"#ff7b72",
		"#3fb950",
		"#d29922",
		"#58a6ff",
		"#bc8cff",
		"#39c5cf",
		"#b1bac4",
		"#6e7681",
		"#ffa198",
		"#56d364",
		"#e3b341",
		"#79c0ff",
		"#d2a8ff",
		"#56d4dd",
		"#ffffff",
	},
}

func themeFor(appearance string) theme {
	if appearance == "light" {
		return lightTheme
	}
	return darkTheme
}
func (m *model) theme() theme { return themeFor(m.Appearance.Read()) }
func (m *model) color(get func(theme) string) func() string {
	return func() string { return get(m.theme()) }
}

const colorTransition = "background-color 70ms, border-color 70ms, color 70ms"

func (m *model) iconStyle(size float64) ui.StyleBuilder {
	return ui.Style().
		Display("flex").
		Width(size).
		Height(size).
		FlexShrink(0).
		AlignItems("center").
		JustifyContent("center").
		TextColor(m.color(func(t theme) string { return t.TextTertiary })).
		BackgroundColor("transparent").
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.
				BackgroundColor(m.color(func(t theme) string { return t.Hover })).
				TextColor(m.color(func(t theme) string { return t.Text }))
		}).
		Active(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.BackgroundColor(m.color(func(t theme) string { return t.Active }))
		}).
		Transition(colorTransition).
		BorderRadius(5).
		Cursor("default").
		AppRegion("no-drag").
		UserSelect("none")
}
func (m *model) rowStyle(selected func() bool) ui.StyleBuilder {
	return ui.Style().
		Display("flex").
		Width("100%").
		MinWidth(0).
		Height(48).
		FlexShrink(0).
		AlignItems("center").
		Gap(8).
		PaddingLeft(8).
		PaddingRight(8).
		BackgroundColor(func() string { return choose(selected(), m.theme().Selected, "transparent") }).
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.
				BackgroundColor(func() string { return choose(selected(), m.theme().SelectedStrong, m.theme().Hover) })
		}).
		Active(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.BackgroundColor(m.color(func(t theme) string { return t.Active }))
		}).
		Transition(colorTransition).
		BorderRadius(6).
		Cursor("default").
		UserSelect("none").
		AppRegion("no-drag")
}
func (m *model) buttonStyle(primary bool) ui.StyleBuilder {
	return ui.Style().
		Display("flex").
		Height(30).
		FlexShrink(0).
		AlignItems("center").
		JustifyContent("center").
		PaddingLeft(11).
		PaddingRight(11).
		BackgroundColor(func() string { return choose(primary, m.theme().Accent, "transparent") }).
		TextColor(func() string { return choose(primary, m.theme().AccentText, m.theme().TextSecondary) }).
		BorderWidth(choose(primary, 0, 1)).
		BorderColor(m.color(func(t theme) string { return t.Border })).
		BorderRadius(5).
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.BackgroundColor(func() string { return choose(primary, m.theme().AccentHover, m.theme().Hover) })
		}).
		Active(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.BackgroundColor(func() string { return choose(primary, m.theme().AccentHover, m.theme().Active) })
		}).
		FontSize(11).
		FontWeight(choose(primary, 680, 500)).
		Cursor("default").
		AppRegion("no-drag").
		UserSelect("none").
		Transition(colorTransition)
}
