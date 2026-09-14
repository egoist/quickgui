package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/terminal"
	"github.com/egoist/quickgui/go/ui"
)

func appView(m *model) *ui.Element {
	return ui.View().
		Children(

			sidebar(m),
			workspace(m),
			agentSheet(m),
		).
		Position("relative").
		Display("flex").
		FlexDirection("row").
		Width("100%").
		Height("100%").
		MinWidth(0).
		MinHeight(0).
		BackgroundColor(m.color(func(t theme) string { return t.App })).
		TextColor(m.color(func(t theme) string { return t.Text })).
		FontWeight(500)
}
func workspace(m *model) *ui.Element {
	return ui.View().
		Child(
			func() *native.Node {
				var children []*native.Node
				children = append(children, tabBar(m).Node)
				children = append(children, ui.View().
					Height(1).
					FlexShrink(0).
					BackgroundColor(m.color(func(t theme) string { return t.Border })).Node)
				children = append(

					// Hide inactive surfaces without unmounting them. Each native PTY, scrollback,
					// selection and working directory survives tab and space navigation.
					children, ui.View().
						Children(

							ui.KeyedFor(
								m.Tabs.Read,
								func(t workspaceTab) any { return t.ID },
								func(tab func() workspaceTab, _ func() int) *ui.Element {
									return tabSurface(m, tab)
								},
								nil,
							),
							ui.Show(
								m.activeTab() == nil,
								func() *ui.Element {
									return ui.View().
										Children(

											icon("terminal", 24, m.color(func(t theme) string { return t.TextGhost })),
											ui.Text(
												"Open a tab to start working",
											).
												TextColor(m.color(func(t theme) string { return t.TextTertiary })).
												FontSize(13.5),
											ui.Button().
												Style(m.buttonStyle(false)).
												Child("New Tab").
												OnClick(func() { m.newTerminal(m.activeSpace()) }),
										).
										Display("flex").
										Flex(1).
										MinWidth(0).
										MinHeight(0).
										FlexDirection("column").
										AlignItems("center").
										JustifyContent("center").
										Gap(9).
										BackgroundColor(m.color(func(t theme) string { return t.Terminal }))
								},
							),
						).
						Position("relative").
						Display("flex").
						Flex(1).
						MinWidth(0).
						MinHeight(0).Node)
				errorMessage := func() string {
					if err := m.Error.Read(); err != "" {
						return err
					}
					if p := m.activePane(); p != nil && p.Status.Read().Status == "failed" {
						return p.Status.Read().Message
					}
					return ""
				}
				children = append(children, ui.Show(
					errorMessage() != "",
					func() *ui.Element {
						return ui.View().
							Children(

								ui.Text(
									errorMessage,
								).
									TextColor(m.color(func(t theme) string { return t.Danger })).
									FontSize(11.5),
								ui.View().Flex(1),
								ui.Show(

									m.Error.Read() == "" && m.activePane() != nil,

									func() *ui.Element {
										return ui.Button().
											Style(m.buttonStyle(false)).
											Child("Restart").
											Height(22).
											FontSize(10.5).
											OnClick(func() {
												if p := m.activePane(); p != nil {
													m.restartPane(p)
												}
											})

									},
									func() *ui.Element {
										return iconButton(m, "Dismiss error", "close", 22, func() { m.Error.Write("") })
									},
								),
							).
							Display("flex").
							MinHeight(30).
							FlexShrink(0).
							AlignItems("center").
							Gap(8).
							PaddingLeft(12).
							PaddingRight(8).
							BackgroundColor(m.color(func(t theme) string { return t.ErrorBackground }))
					},
				))
				return ui.Fragment(children)
			},
		).
		Display("flex").
		Flex(1).
		MinWidth(0).
		MinHeight(0).
		FlexDirection("column").
		BackgroundColor(m.color(func(t theme) string { return t.App }))
}
func tabBar(m *model) *ui.Element {
	return ui.View().
		Children(

			ui.View().
				Children(

					ui.KeyedFor(
						m.spaceTabs,
						func(t workspaceTab) any { return t.ID },
						func(tab func() workspaceTab, _ func() int) *ui.Element {
							active := func() bool { return tab().ID == m.ActiveTabID.Read() }
							return ui.View().
								Children(

									ui.Button().
										Child(ui.Text(

											tabTitle(tab(), m.Panes.Read()),
										).
											FontSize(12.5).
											FontWeight(560).
											LineClamp(1).
											TextOverflow("ellipsis")).
										AriaLabel("Select terminal tab").
										Display("flex").
										Width("100%").
										MinWidth(0).
										Height(28).
										AlignItems("center").
										PaddingLeft(10).
										PaddingRight(choose(active(), 30, 10)).
										BackgroundColor("transparent").
										TextColor(choose(active(), m.theme().Text, m.theme().TextTertiary)).
										Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
											return s.
												BackgroundColor(func() string {
													return choose(active(), m.theme().SelectedStrong, m.theme().Hover)
												})
										}).
										Active(func(s ui.StyleBuilder) ui.StyleBuilder {
											return s.BackgroundColor(m.color(func(t theme) string { return t.Active }))
										}).
										BorderRadius(6).
										Cursor("default").
										AppRegion("no-drag").
										OnClick(func() { m.selectTab(tab()) }),

									ui.Show(
										active,
										func() *ui.Element {
											return iconButton(m, "Close tab", "close", 20, func() { m.closeTab(tab().ID) }).
												Style(ui.Style().
													Position("absolute").
													Top(4).
													Right(3))
										},
									),
								).
								Position("relative").
								Display("flex").
								MinWidth(74).
								MaxWidth(170).
								Height(28).
								FlexShrink(1).
								AlignItems("center").
								BackgroundColor(choose(active(), m.theme().Selected, "transparent")).
								BorderRadius(6).
								Group("tab")

						},
						nil,
					),
					iconButton(m, "New tab", "plus", 24, func() { m.newTerminal(m.activeSpace()) }),
				).
				Display("flex").
				MinWidth(0).
				AlignItems("center").
				Gap(3).
				Overflow("hidden").
				AppRegion("no-drag"),
			ui.View().Flex(1).AppRegion("drag"),
			ui.Button().
				Children(
					icon("plus", 13, m.color(func(t theme) string { return t.TextSecondary })),
					ui.Text("Agent"),
				).
				AriaLabel("New agent").
				FocusOnPointer(ptr(false)).
				Display("flex").
				Height(24).
				FlexShrink(0).
				AlignItems("center").
				JustifyContent("center").
				Gap(5).
				PaddingLeft(9).
				PaddingRight(9).
				TextColor(m.color(func(t theme) string { return t.TextSecondary })).
				BackgroundColor("transparent").
				Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
					return s.BackgroundColor(m.color(func(t theme) string { return t.Hover }))
				}).
				Active(func(s ui.StyleBuilder) ui.StyleBuilder {
					return s.BackgroundColor(m.color(func(t theme) string { return t.Active }))
				}).
				Transition(colorTransition).
				BorderRadius(5).
				FontSize(11.5).
				FontWeight(600).
				Cursor("default").
				AppRegion("no-drag").
				OnClick(m.openAgentSheet),

			ui.View().
				Width(1).
				Height(16).
				FlexShrink(0).
				MarginLeft(2).
				MarginRight(2).
				BackgroundColor(m.color(func(t theme) string { return t.BorderStrong })),
			iconButton(m, "Split right", "columns", 24, func() { m.splitTerminal("horizontal") }),
			iconButton(m, "Split down", "rows", 24, func() { m.splitTerminal("vertical") }),
		).
		Display("flex").
		Height(40).
		FlexShrink(0).
		AlignItems("center").
		Gap(4).
		PaddingLeft(8).
		PaddingRight(8).
		AppRegion("drag")
}
func tabSurface(m *model, tab func() workspaceTab) *ui.Element {
	return ui.View().
		Child(

			// Restart replaces the pane object; other model updates preserve its identity.
			ui.For(
				func() []*pane { return m.tabPanes(tab().ID) },
				func(p *pane, _ func() int) *ui.Element { return terminalPane(m, p) },
				func(p *pane) any { return p },
				nil,
			),
		).
		Position("absolute").
		Display("flex").
		Top(0).
		Right(0).
		Bottom(0).
		Left(0).
		MinWidth(0).
		MinHeight(0).
		Gap(5).
		FlexDirection("row").
		When(
			tab().Direction == "vertical",
			ui.Style().FlexDirection("column"),
		).
		When(
			tab().ID != m.ActiveTabID.Read(),
			ui.Style().Visibility("hidden"),
		)

}
func terminalPane(m *model, p *pane) *ui.Element {
	directory := ""
	for _, s := range m.Spaces.Peek() {
		if s.ID == p.SpaceID {
			directory = s.Path
		}
	}
	return ui.View().
		Child(

			ui.View().
				Child(

					terminal.View(terminal.Props{
						Program:          p.Program,
						Args:             p.Arguments,
						WorkingDirectory: directory,
						Environment:      p.Environment,
						Scrollback:       50000,
						Palette:          func() terminal.Palette { return m.theme().TerminalPalette },
						CursorColor:      m.color(func(t theme) string { return t.TerminalCursor }),
						PaddingColor:     "extend",
						FontThicken:      true,
						OnStatus: func(event *native.Event) {
							if status := terminal.StatusFromEvent(event); status != nil {
								p.Status.Write(*status)
							}
						},
						Props: ui.Props{
							Ref:       func(node *native.Node) { m.registerTerminal(p, node) },
							AriaLabel: "Terminal pane",
							OnClick:   func(*native.Event) { m.selectPane(p) },
							Style: ui.Style().
								Position("absolute").
								Top(0).
								Right(0).
								Bottom(0).
								Left(0).
								Padding(8).
								BorderWidth(1).
								BorderColor(m.color(func(t theme) string { return t.Terminal })).
								BackgroundColor(m.color(func(t theme) string { return t.Terminal })).
								TextColor(m.color(func(t theme) string { return t.TerminalText })).
								FontFamily("JetBrainsMono Nerd Font Mono").
								FontSize(14).
								FontWeight(400).
								LineHeight(20.5),
						},
					}),
				).
				Position("relative").
				Display("flex").
				Flex(1).
				MinWidth(0).
				MinHeight(0),
		).
		Display("flex").
		FlexGrow(1).
		FlexBasis(0).
		MinWidth(0).
		MinHeight(0).
		FlexDirection("column").
		BackgroundColor(m.color(func(t theme) string { return t.Terminal }))
}
