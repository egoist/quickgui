package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/ui"
)

func agentSheet(m *model) *native.Node {
	dialog3 := ui.NewDialog(ui.DialogRootProps{
		Open: m.SheetOpen.Read,
		OnOpenChange: func(open bool, _ ui.DialogOpenChangeDetails) {
			if !open {
				m.closeAgentSheet()
			}
		},
	})
	return dialog3.Root().
		Children(func() *native.Node {
			return dialog3.Portal(ui.PartProps{Style: ui.Style().
				Position("absolute").
				Top(0).
				Right(0).
				Bottom(0).
				Left(0).
				Display("flex").
				AlignItems("center").
				JustifyContent("center").
				Padding(24)}).
				Children(func() *native.Node {
					return ui.Fragment([]*native.Node{
						dialog3.Backdrop(ui.PartProps{Style: ui.Style().
							Position("absolute").
							Top(0).
							Right(0).
							Bottom(0).
							Left(0).
							BackgroundColor(m.color(func(t theme) string {
								return t.Scrim
							}))}).
							NativeNode(),
						dialog3.Popup(ui.DialogPopupProps{PartProps: ui.PartProps{
							AriaLabel: "New agent",
							Style: ui.Style().
								Display("flex").
								Width(472).
								MaxWidth("100%").
								FlexDirection("column").
								Gap(17).
								Padding(20).
								BackgroundColor(m.color(func(t theme) string {
									return t.Surface
								})).
								BorderColor(m.color(func(t theme) string {
									return t.BorderStrong
								})).
								BorderWidth(1).
								BorderRadius(9),
						}}).
							Children(func() *native.Node {
								return ui.Fragment([]*native.Node{
									ui.View().
										Children(
											ui.View().
												Children(
													dialog3.Title(ui.PartProps{Style: ui.Style().
														TextColor(m.color(func(t theme) string {
															return t.Text
														})).
														FontSize(17).
														FontWeight(720)}).
														Children("New agent").
														NativeNode(),
													dialog3.Description(ui.PartProps{Style: ui.Style().
														TextColor(m.color(func(t theme) string {
															return t.TextTertiary
														})).
														FontSize(12)}).
														Children(func() *ui.Element {
															return ui.Text("Start a real CLI in " + m.activeSpace().Name)
														}).
														NativeNode(),
												).
												Display("flex").
												FlexDirection("column").
												Gap(4),
											ui.View().Flex(1),
											iconButton(m, "Close new agent", "close", 26, m.closeAgentSheet),
										).
										Display("flex").
										AlignItems("flex-start").Node,
									formGroup(func() *native.Node {
										return ui.Fragment([]*native.Node{
											formLabel(m, "Agent").Node,
											ui.View().
												Child(ui.KeyedFor(
													m.Launchers.Read,
													func(l launcher) any {
														return l.ID
													},
													func(read func() launcher, _ func() int) *ui.Element {
														selected := func() bool {
															return read().ID == m.SelectedLauncherID.Read()
														}
														return ui.Button().
															Children(
																ui.View().
																	Child(ui.Text(read().Mark).
																		FontSize(13).
																		FontWeight(750)).
																	Display("flex").
																	Width(24).
																	Height(24).
																	FlexShrink(0).
																	AlignItems("center").
																	JustifyContent("center").
																	BackgroundColor(m.color(func(t theme) string {
																		return t.AccentWash
																	})).
																	BorderRadius(5),
																ui.View().
																	Children(
																		ui.Text(read().Label).
																			FontSize(12).
																			FontWeight(620),
																		ui.Text(choose(read().installed(), "Available", "Not found")).
																			TextColor(m.color(func(t theme) string {
																				return t.TextGhost
																			})).
																			FontSize(10),
																	).
																	Display("flex").
																	FlexDirection("column").
																	Gap(2),
															).
															AriaLabel(read().Label).
															Disabled(!read().installed()).
															OnClick(func() {
																m.SelectedLauncherID.Write(read().ID)
															}).
															Display("flex").
															Flex(1).
															MinWidth(0).
															Height(54).
															AlignItems("center").
															Gap(8).
															PaddingLeft(8).
															PaddingRight(8).
															BackgroundColor(choose(selected(), m.theme().Selected, "transparent")).
															TextColor(choose(read().installed(), m.theme().Text, m.theme().TextGhost)).
															BorderColor(choose(selected(), m.theme().Accent, m.theme().Border)).
															BorderWidth(1).
															BorderRadius(6).
															Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
																return s.BackgroundColor(func() string {
																	return choose(read().installed(), m.theme().Hover, "transparent")
																})
															}).
															Opacity(choose(read().installed(), 1.0, .5)).
															Cursor("default")
													},
													nil,
												)).
												Display("flex").
												Gap(8).Node,
											ui.Text(func() string {
												if m.CatalogLoading.Read() {
													return "Reading your interactive login-shell PATH…"
												}
												return m.selectedLauncher().Description
											}).
												TextColor(m.color(func(t theme) string {
													return t.TextGhost
												})).
												FontSize(11).Node,
										})
									}).Node,
									formGroup(func() *native.Node {
										return ui.Fragment([]*native.Node{
											formLabel(m, "Initial instruction · optional").Node,
											ui.TextArea().
												AriaLabel("Initial instruction").
												Value(m.Prompt.Read).
												Placeholder("What should this agent work on?").
												OnInputEvent(func(e *native.Event) {
													m.Prompt.Write(ui.InputValue(e))
												}).
												Ref(func(node *native.Node) {
													native.SetBoolean(
														node,
														protocol.AutoFocus,
														true,
													)
												}).
												Display("flex").
												Height(88).
												Width("100%").
												PaddingLeft(10).
												PaddingRight(10).
												PaddingTop(9).
												PaddingBottom(9).
												BackgroundColor(m.color(func(t theme) string {
													return t.Terminal
												})).
												TextColor(m.color(func(t theme) string {
													return t.Text
												})).
												BorderColor(m.color(func(t theme) string {
													return t.BorderStrong
												})).
												BorderWidth(1).
												BorderRadius(6).
												FontSize(12.5).
												FocusStyle(func(s ui.StyleBuilder) ui.StyleBuilder {
													return s.OutlineWidth(1).OutlineColor(m.color(func(t theme) string {
														return t.Accent
													}))
												}).Node,
										})
									}).Node,
									ui.Show(
										func() bool {
											if m.CatalogLoading.Read() {
												return false
											}
											for _, l := range m.Launchers.Read() {
												if l.installed() {
													return false
												}
											}
											return true
										},
										func() *ui.Element {
											return ui.View().
												Child(ui.Text("No supported agent CLI was found. You can still open a terminal and run any installed agent; the sidebar detects it automatically.").
													TextColor(m.color(func(t theme) string {
														return t.Warning
													})).
													FontSize(11.5).
													LineHeight(16)).
												Display("flex").
												MinHeight(36).
												AlignItems("center").
												PaddingLeft(10).
												PaddingRight(10).
												PaddingTop(7).
												PaddingBottom(7).
												BackgroundColor(m.color(func(t theme) string {
													return t.AccentWash
												})).
												BorderRadius(5)
										},
									),
									ui.View().
										Child(func() *native.Node {
											var children_ []*native.Node
											children_ = append(children_, ui.Button().
												Style(m.buttonStyle(false)).
												Child("Cancel").
												OnClick(m.closeAgentSheet).Node)
											disabled := func() bool {
												return m.CatalogLoading.Read() || !m.selectedLauncher().installed()
											}
											children_ = append(children_, ui.Button().
												Style(m.buttonStyle(true)).
												Child("Start "+m.selectedLauncher().Label).
												Disabled(disabled).
												Opacity(choose(disabled(), .45, 1.0)).
												OnClick(m.launchAgent).Node)
											return ui.Fragment(children_)
										}).
										Display("flex").
										JustifyContent("flex-end").
										Gap(8).Node,
								})
							}).
							NativeNode(),
					})
				}).
				NativeNode()
		}).
		NativeNode()
}
func formGroup(children ui.Component) *ui.Element {
	return ui.View().Child(children).Display("flex").FlexDirection("column").Gap(7)
}
func formLabel(m *model, label string) *ui.Element {
	return ui.Text(label).
		TextColor(m.color(func(t theme) string {
			return t.TextSecondary
		})).
		FontSize(11.5).
		FontWeight(620)
}
