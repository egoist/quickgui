package main

import "github.com/egoist/quickgui/go/native"

import (
	"fmt"

	"github.com/egoist/quickgui/go/ui"
)

func sidebar(m *model) *native.Node {
	return ui.Fragment([]*native.Node{
		ui.View().
			Children(

				ui.View().
					Children(

						ui.View().Flex(1),
						ui.Button().
							Style(m.iconStyle(24)).
							Child(dynamicIcon(func() string { return choose(m.Appearance.Read() == "dark", "sun", "moon") }, 14, m.color(func(t theme) string { return t.TextTertiary }))).
							AriaLabel("Toggle light or dark appearance").
							FocusOnPointer(ptr(false)).
							OnClick(m.toggleTheme),
					).
					Display("flex").
					Height(40).
					FlexShrink(0).
					AlignItems("center").
					PaddingLeft(78).
					PaddingRight(8).
					PaddingTop(2).
					AppRegion("drag"),
				sidebarSection(m.SectionRatio.Read, func() *native.Node {
					return ui.Fragment([]*native.Node{
						sectionHeader(m, "Spaces", "", func() { m.addSpace() }).Node,
						sidebarList(func() *native.Node {
							return ui.For(
								m.Spaces.Read,
								func(s space, _ func() int) *ui.Element {
									return spaceRow(m, s)
								},
								func(s space) any { return s.ID },
								nil,
							)
						}).Node,
					})
				}),
				ui.View().
					Child(

						ui.View().
							Width("100%").
							Height(1).
							BackgroundColor(m.color(func(t theme) string { return t.Border })),
					).
					AriaLabel("Resize sidebar sections").
					Display("flex").
					Height(7).
					FlexShrink(0).
					AlignItems("center").
					PaddingLeft(8).
					PaddingRight(8).
					Cursor("ns-resize").
					AppRegion("no-drag").
					OnPointer(m.handleSectionPointer),

				sidebarSection(func() float64 { return 1 - m.SectionRatio.Read() }, func() *native.Node {
					return ui.Fragment([]*native.Node{
						sectionHeader(m, "Agents", "grouped", nil).Node,
						sidebarList(func() *native.Node {
							return ui.For(
								m.visibleAgents,
								func(p *pane, _ func() int) *ui.Element {
									return agentRow(m, p)
								},
								func(p *pane) any { return p },
								func() *ui.Element {
									return ui.View().
										Child(

											ui.Text(
												"Agents appear here when detected in a pane.",
											).
												TextColor(m.color(func(t theme) string { return t.TextGhost })).
												FontSize(12).
												LineHeight(16),
										).
										Display("flex").
										PaddingLeft(10).
										PaddingRight(10).
										PaddingTop(8)
								},
							)
						}).Node,
					})
				}),
			).
			Display("flex").
			FlexDirection("column").
			FlexShrink(0).
			MinWidth(0).
			MinHeight(0).
			Width(m.SidebarWidth.Read).
			BackgroundColor(m.color(func(t theme) string { return t.Sidebar })).Node,
		ui.View().
			Child(

				ui.View().
					Width(1).
					Height("100%").
					BackgroundColor(m.color(func(t theme) string { return t.Border })),
			).
			AriaLabel("Resize sidebar").
			Position("relative").
			Display("flex").
			Width(1).
			FlexShrink(0).
			HitSlopLeft(5).
			Cursor("ew-resize").
			AppRegion("no-drag").
			OnPointer(m.handleSidebarPointer).
			Node,
	})
}
func sidebarSection(grow func() float64, children ui.Component) *ui.Element {
	return ui.View().
		Child(
			children,
		).
		Display("flex").
		FlexBasis(0).
		FlexGrow(grow()).
		MinHeight(0).
		FlexDirection("column").
		PaddingLeft(8).
		PaddingRight(8)
}
func sidebarList(children ui.Component) *ui.Element {
	return ui.View().
		Child(
			children,
		).
		Display("flex").
		Flex(1).
		MinHeight(0).
		FlexDirection("column").
		Gap(2).
		PaddingBottom(8).
		OverflowY("auto")
}
func sectionHeader(m *model, label, trailing string, action func()) *ui.Element {
	return ui.View().
		Child(
			func() *native.Node {
				var children_ []*native.Node
				children_ = append(children_, ui.Text(
					label,
				).
					TextColor(m.color(func(t theme) string { return t.TextTertiary })).
					FontSize(12).
					FontWeight(650).Node)
				children_ = append(children_, ui.View().Flex(1).Node)
				if trailing != "" {
					children_ = append(children_, ui.Text(
						trailing,
					).
						TextColor(m.color(func(t theme) string { return t.TextGhost })).
						FontSize(10.5).Node)
				}
				if action != nil {
					children_ = append(children_, iconButton(m, "Add space", "plus", 20, action).Style(ui.Style().MarginLeft(5)).Node)
				}
				return ui.Fragment(children_)
			},
		).
		Display("flex").
		Height(30).
		FlexShrink(0).
		AlignItems("center").
		PaddingLeft(10).
		PaddingRight(6)
}
func twoLineRow(children ui.Component) *ui.Element {
	return ui.View().
		Child(
			children,
		).
		Display("flex").
		Flex(1).
		MinWidth(0).
		FlexDirection("column").
		JustifyContent("center").
		Gap(1)
}
func rowLine(children ui.Component) *ui.Element {
	return ui.View().
		Child(
			children,
		).
		Display("flex").
		MinWidth(0).
		AlignItems("center").
		Gap(5)
}
func rowTitle(m *model, text any) *ui.Element {
	return ui.Text(
		text,
	).
		TextColor(m.color(func(t theme) string { return t.Text })).
		FontSize(13.5).
		LineHeight(16).
		FontWeight(560).
		LineClamp(1).
		TextOverflow("ellipsis")
}
func rowMeta(m *model, text any) *ui.Element {
	return ui.Text(
		text,
	).
		TextColor(m.color(func(t theme) string { return t.TextTertiary })).
		FontSize(11.5).
		LineHeight(15).
		LineClamp(1).
		TextOverflow("ellipsis")
}
func spaceRow(m *model, s space) *ui.Element {
	selected := func() bool { return m.ActiveSpaceID.Read() == s.ID }
	agents := func() []*pane {
		var out []*pane
		for _, p := range m.Panes.Read() {
			if p.SpaceID == s.ID && p.Status.Read().Agent != "" {
				out = append(out, p)
			}
		}
		return out
	}
	return ui.View().
		Children(

			ui.Button().
				Style(m.rowStyle(selected)).
				Children(
					statusGlyph(m, func() string { return aggregateStatus(agents()) }, true),
					twoLineRow(func() *native.Node {
						return ui.Fragment([]*native.Node{
							rowLine(func() *native.Node {
								return ui.Fragment([]*native.Node{
									rowTitle(m, s.Name).Node,
									ui.View().Flex(1).Node,
									ui.Show(
										len(agents()) > 0,
										func() *ui.Element {
											return ui.Text(
												fmt.Sprint(len(agents())),
											).
												TextColor(m.color(func(t theme) string { return t.TextGhost })).
												FontSize(11.5).
												LineHeight(16)
										},
									),
								})
							}).Node,
							rowMeta(m, shortPath(s.Path, m.Home)).Node,
						})
					}),
				).
				AriaLabel("Open "+s.Name+" space").
				FocusOnPointer(ptr(false)).
				PaddingRight(choose(selected(), 30, 8)).
				OnClick(func() { m.selectSpace(s) }),

			ui.Show(
				selected() && len(m.Spaces.Read()) > 1,
				func() *ui.Element {
					return iconButton(m, "Remove "+s.Name+" space", "close", 20, func() { m.removeSpace(s) }).
						Style(ui.Style().
							Position("absolute").
							Top(14).
							Right(5))
				},
			),
		).
		Position("relative").
		Display("flex").
		FlexShrink(0).
		Height(48).
		BorderRadius(6)
}
func agentRow(m *model, p *pane) *ui.Element {
	return ui.Button().
		Style(m.rowStyle(func() bool { return m.ActivePaneID.Read() == p.ID })).
		Children(
			statusGlyph(m, func() string { return p.Status.Read().AgentStatus }, false),
			twoLineRow(func() *native.Node {
				return ui.Fragment([]*native.Node{
					rowLine(func() *native.Node {
						return ui.Fragment([]*native.Node{
							rowTitle(m, func() string {
								for _, s := range m.Spaces.Read() {
									if s.ID == p.SpaceID {
										return s.Name
									}
								}
								return ""
							}).Node,
							rowMeta(m, "·").Node,
							rowMeta(m, func() string {
								if t := m.tab(p.TabID); t != nil {
									return tabTitle(*t, m.Panes.Read())
								}
								return "Terminal"
							}).Node,
						})
					}).Node,
					rowLine(func() *native.Node {
						return ui.Fragment([]*native.Node{
							rowMeta(m, func() string { return agentLabel(p.Status.Read().Agent) }).Node,
							ui.View().Flex(1).Node,
							ui.Text(
								statusLabel(p.Status.Read().AgentStatus),
							).
								TextColor(
									choose(p.Status.Read().AgentStatus == "blocked", m.theme().Danger, m.theme().TextGhost),
								).
								FontSize(11.5).
								LineHeight(15).Node,
						})
					}).Node,
				})
			}),
		).
		AriaLabel("Open detected agent").
		FocusOnPointer(ptr(false)).
		OnClick(func() { m.selectPane(p) })

}
