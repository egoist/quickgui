package ui

import "github.com/egoist/quickgui/packages/go/ui"

type Theme struct {
	Appearance                                                                    string
	Content, SidebarWash, ContentAlt, Raised                                      string
	Text, TextSecondary, TextTertiary, TextOnAccent                               string
	Border, BorderStrong, Hover, Active, Selection, SelectionText, SelectionMuted string
	Accent, AccentHover, AccentWash                                               string
	Success, SuccessWash, Danger, DangerWash, Warning, WarningWash                string
	DiffAdded, DiffAddedGutter, DiffAddedText                                     string
	DiffRemoved, DiffRemovedGutter, DiffRemovedText                               string
	DiffHunk, DiffHunkText, DiffLineNumber                                        string
	Graph                                                                         []string
	Scrim, Input, InputBorder, FocusRing                                          string
}

const (
	UIFontSize     = 13
	MonoFontSize   = 12
	TitlebarHeight = 52
)

func ThemeFor(appearance string) Theme {
	if appearance == "light" {
		return Theme{
			Appearance: appearance, Content: "#ffffff", SidebarWash: "#00000000", ContentAlt: "#f6f6f8", Raised: "#ffffff",
			Text: "#1d1d1f", TextSecondary: "#5f5f66", TextTertiary: "#9a9aa1", TextOnAccent: "#ffffff",
			Border: "#e4e4e8", BorderStrong: "#cfcfd6", Hover: "#00000009", Active: "#00000014",
			Selection: "#0000001c", SelectionText: "#1d1d1f", SelectionMuted: "#dbe6ff",
			Accent: "#007aff", AccentHover: "#0a6ff0", AccentWash: "#e5f0ff",
			Success: "#1a7f37", SuccessWash: "#e6f6ea", Danger: "#cf222e", DangerWash: "#ffebe9",
			Warning: "#9a6700", WarningWash: "#fff8c5",
			DiffAdded: "#e8ffee", DiffAddedGutter: "#c9f2d4", DiffAddedText: "#1a7f37",
			DiffRemoved: "#ffebe9", DiffRemovedGutter: "#ffd0cc", DiffRemovedText: "#cf222e",
			DiffHunk: "#eef4ff", DiffHunkText: "#3b6fd6", DiffLineNumber: "#a1a1a8",
			Graph: []string{"#2f6bff", "#1a9e63", "#d0432f", "#b56cd8", "#e08a1e", "#1b9fb7", "#c74f9d", "#6d7fd6"},
			Scrim: "#1a1a2266", Input: "#ffffff", InputBorder: "#00000026", FocusRing: "#007aff66",
		}
	}
	return Theme{
		Appearance: appearance, Content: "#1d1d1f", SidebarWash: "#00000000", ContentAlt: "#232326", Raised: "#2a2a2e",
		Text: "#f2f2f5", TextSecondary: "#a9a9b2", TextTertiary: "#6f6f78", TextOnAccent: "#ffffff",
		Border: "#333338", BorderStrong: "#45454c", Hover: "#ffffff0d", Active: "#ffffff1a",
		Selection: "#ffffff29", SelectionText: "#f2f2f5", SelectionMuted: "#243657",
		Accent: "#0a84ff", AccentHover: "#2b93ff", AccentWash: "#1c2d4a",
		Success: "#4cc38a", SuccessWash: "#16301f", Danger: "#ff6b62", DangerWash: "#3a1d20",
		Warning: "#e3b341", WarningWash: "#3a2e12",
		DiffAdded: "#15291c", DiffAddedGutter: "#1f4a2d", DiffAddedText: "#5fd48f",
		DiffRemoved: "#33191c", DiffRemovedGutter: "#5a2a2e", DiffRemovedText: "#ff8a80",
		DiffHunk: "#1a2231", DiffHunkText: "#7aa7ff", DiffLineNumber: "#5f5f68",
		Graph: []string{"#4a86ff", "#3fbf7f", "#ff6b62", "#c792ea", "#f0a044", "#3ec1d3", "#e57bbd", "#8da2ff"},
		Scrim: "#00000099", Input: "#1a1a1c", InputBorder: "#ffffff26", FocusRing: "#0a84ff66",
	}
}

func StatusColor(theme Theme, code string) string {
	switch code {
	case "A", "?":
		return theme.Success
	case "D":
		return theme.Danger
	case "R", "C":
		return theme.Accent
	default:
		return theme.Warning
	}
}

func (t Theme) Button(kind string) ui.Style {
	style := ui.Style{
		Display: "flex", Height: 22, FlexShrink: 0, AlignItems: "center", JustifyContent: "center",
		Gap: 5, PaddingLeft: 11, PaddingRight: 11, BorderRadius: 6, FontSize: UIFontSize,
		Cursor: "default", UserSelect: "none", AppRegion: "no-drag",
		Disabled: &ui.Style{Opacity: 0.4},
	}
	if kind == "primary" {
		style.BackgroundColor = t.Accent
		style.Color = t.TextOnAccent
		style.Hover = &ui.Style{BackgroundColor: t.AccentHover}
	} else {
		style.BackgroundColor = t.Raised
		if kind == "danger" {
			style.Color = t.Danger
		} else {
			style.Color = t.Text
		}
		style.BorderWidth = 1
		style.BorderColor = t.BorderStrong
		style.Hover = &ui.Style{BackgroundColor: t.Hover}
	}
	return style
}

func (t Theme) IconButton() ui.Style {
	return ui.Style{
		Display: "flex", Width: 22, Height: 22, FlexShrink: 0, AlignItems: "center", JustifyContent: "center",
		Color: t.TextSecondary, BackgroundColor: "transparent", BorderRadius: 5, Cursor: "default", AppRegion: "no-drag",
		Hover: &ui.Style{BackgroundColor: t.Hover, Color: t.Text}, Active: &ui.Style{BackgroundColor: t.Active},
		Disabled: &ui.Style{Opacity: 0.4},
	}
}

func (t Theme) InputStyle() ui.Style {
	return ui.Style{
		Display: "flex", Height: 22, PaddingLeft: 7, PaddingRight: 7, BackgroundColor: t.Input, Color: t.Text,
		BorderWidth: 1, BorderColor: t.InputBorder, BorderRadius: 6, FontSize: UIFontSize, AppRegion: "no-drag",
		Focus: &ui.Style{BorderColor: t.Accent},
	}
}
