package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
	"log"
	"strconv"
)

type demo struct {
	ID, Label, Source string
	Component         ui.Component
}

var demos = []demo{{"accordion", "Accordion", "", AccordionDemo}, {"alert-dialog", "Alert Dialog", "", AlertDialogDemo}, {"autocomplete", "Autocomplete", "", AutocompleteDemo}, {"avatar", "Avatar", "", AvatarDemo}, {"button", "Button", "", ButtonDemo}, {"calendar", "Calendar", "QuickGUI", CalendarDemo}, {"checkbox", "Checkbox", "", CheckboxDemo}, {"checkbox-group", "Checkbox Group", "", CheckboxGroupDemo}, {"collapsible", "Collapsible", "", CollapsibleDemo}, {"combobox", "Combobox", "", ComboboxDemo}, {"context-menu", "Context Menu", "", ContextMenuDemo}, {"system-context-menu", "System Context Menu", "System", SystemContextMenuDemo}, {"date-field", "Date Field", "QuickGUI", DateFieldDemo}, {"dialog", "Dialog", "", DialogDemo}, {"field", "Field", "", FieldDemo}, {"fieldset", "Fieldset", "", FieldsetDemo}, {"form", "Form", "", FormDemo}, {"input", "Input", "", InputDemo}, {"menu", "Menu", "", MenuDemo}, {"menubar", "Menubar", "", MenubarDemo}, {"meter", "Meter", "", MeterDemo}, {"navigation-menu", "Navigation Menu", "", NavigationMenuDemo}, {"number-field", "Number Field", "", NumberFieldDemo}, {"otp-field", "OTP Field", "", OtpFieldDemo}, {"popover", "Popover", "", PopoverDemo}, {"preview-card", "Preview Card", "", PreviewCardDemo}, {"progress", "Progress", "", ProgressDemo}, {"radio", "Radio", "", RadioDemo}, {"scroll-area", "Scroll Area", "", ScrollAreaDemo}, {"select", "Select", "", SelectDemo}, {"separator", "Separator", "", SeparatorDemo}, {"slider", "Slider", "", SliderDemo}, {"splitter", "Splitter", "QuickGUI", SplitterDemo}, {"switch", "Switch", "", SwitchDemo}, {"tabs", "Tabs", "", TabsDemo}, {"time-field", "Time Field", "QuickGUI", TimeFieldDemo}, {"toast", "Toast", "", ToastDemo}, {"toggle", "Toggle", "", ToggleDemo}, {"toggle-group", "Toggle Group", "", ToggleGroupDemo}, {"toolbar", "Toolbar", "", ToolbarDemo}, {"tooltip", "Tooltip", "", TooltipDemo}, {"table", "Table", "QuickGUI", TableDemo}, {"tree", "Tree", "QuickGUI", TreeDemo}}

func main() {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:         "QuickGUI Components",
				Width:         1080,
				Height:        780,
				MinimumWidth:  880,
				MinimumHeight: 620,
				TitleBarStyle: "hiddenInset",
				TrafficLightPosition: &native.Point{
					X: 16,
					Y: 18,
				},
				Background: lightPalette.Window,
				Component: func() *native.Node {
					window := native.CurrentWindow()
					appearance, setAppearance := ui.CreateSignal("light")
					size, setSize := ui.CreateSignal(native.Point{X: 1080, Y: 780})
					update := func() {
						window.GetState(func(state native.WindowState, err error) {
							if err == nil {
								setAppearance(state.Appearance)
								setSize(native.Point{
									X: state.ViewportWidth,
									Y: state.ViewportHeight,
								})
							}
						})
					}
					ui.OnCleanup(window.On(native.WindowReadyToShow, func(native.WindowEvent) {
						update()
					}))
					ui.OnCleanup(window.On(native.WindowResize, func(native.WindowEvent) {
						update()
					}))
					ui.OnCleanup(window.On(native.WindowAppearance, func(event native.WindowEvent) {
						setAppearance(event.Appearance)
					}))
					return reactive.Provide(galleryContext, galleryTheme{Appearance: appearance, Size: size}, Gallery)
				},
			})
		}
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
func Gallery() *native.Node {
	selected, setSelected := ui.CreateSignal("accordion")
	current := func() demo {
		for _, entry := range demos {
			if entry.ID == selected() {
				return entry
			}
		}
		return demos[0]
	}
	tabs4 := ui.NewTabs(ui.TabsRootProps{
		Value: func() *string {
			return ptr(selected())
		},
		OnValueChange: func(value string, _ *native.Event) {
			setSelected(value)
		},
		Orientation: "vertical",
		Activation:  "manual",
		PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
			return ui.Style().
				Display("flex").
				Width("100%").
				Height("100%").
				BackgroundColor(p().Window).
				TextColor(p().Ink)
		}},
	})
	return tabs4.Root().
		Children(func() *native.Node {
			return ui.Fragment([]*native.Node{
				ui.View().
					Children(
						ui.View().
							Display("flex").
							Height(52).
							FlexShrink(0).
							AlignItems("center").
							PaddingLeft(82).
							AppRegion("drag"),
						ui.View().
							Child(tabs4.List(ui.PartProps{Style: ui.Style().
								Display("flex").
								FlexDirection("column").
								Gap(1).
								PaddingLeft(8).
								PaddingRight(8).
								PaddingBottom(12)}).
								Children(func() *native.Node {
									var children []*native.Node
									for index, entry := range demos {
										children = append(children, tabs4.TabWith(ui.TabsTabProps{
											Value: entry.ID,
											Index: ptr(index),
											PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
												background, ink, weight := "transparent", p().Ink, 400
												if selected() == entry.ID {
													background, ink, weight = p().Accent, p().OnAccent, 600
												}
												return ui.Style().
													Display("flex").
													AlignItems("center").
													Height(28).
													FlexShrink(0).
													PaddingLeft(10).
													PaddingRight(10).
													BorderRadius(7).
													Cursor("default").
													UserSelect("none").
													BackgroundColor(background).
													TextColor(ink).
													FontSize(13).
													FontWeight(weight).
													Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
														return s.BackgroundColor(choose(selected() == entry.ID, p().Accent, p().ControlHover))
													}).
													FocusStyle(func(s ui.StyleBuilder) ui.StyleBuilder {
														return s.Outline("2px solid " + p().Accent)
													}).
													OutlineOffset(-2)
											}},
										}).
											Children(entry.Label).
											NativeNode())
									}
									return ui.Fragment(children)
								}).
								NativeNode()).
							Display("flex").
							FlexDirection("column").
							Flex(1).
							MinHeight(0).
							OverflowY("scroll"),
					).
					Display("flex").
					FlexDirection("column").
					Width(214).
					FlexShrink(0).
					Height("100%").
					BackgroundColor(color(func(p palette) string {
						return p.Sidebar
					})).
					BorderRightWidth(1).
					BorderColor(color(func(p palette) string {
						return p.Border
					})).Node,
				ui.View().
					Children(
						ui.View().
							Children(
								row(func() *native.Node {
									return ui.Fragment([]*native.Node{
										ui.Text(current().Label).FontSize(15).FontWeight(700).Node,
										ui.Text(func() string {
											switch current().Source {
											case "QuickGUI":
												return "QuickGUI component"
											case "System":
												return "Native system menu"
											default:
												return "Base UI part set"
											}
										}).
											FontSize(11).
											TextColor(color(func(p palette) string {
												return p.Faint
											})).Node,
									})
								}),
								ui.Text(func() string {
									theme := galleryContext.Use()
									size := theme.Size()
									return strconv.Itoa(len(demos)) + " components · " + theme.Appearance() + " appearance · " + strconv.FormatFloat(size.X, 'f', 0, 64) + "×" + strconv.FormatFloat(size.Y, 'f', 0, 64)
								}).
									FontSize(11).
									TextColor(color(func(p palette) string {
										return p.Faint
									})),
							).
							Display("flex").
							AlignItems("center").
							JustifyContent("space-between").
							Height(52).
							FlexShrink(0).
							PaddingLeft(20).
							PaddingRight(20).
							BorderBottomWidth(1).
							BorderColor(color(func(p palette) string {
								return p.Border
							})).
							AppRegion("drag"),
						ui.View().
							Child(func() *native.Node {
								var children []*native.Node
								for _, entry := range demos {
									children = append(children, tabs4.PanelWith(ui.TabsPanelProps{
										Value: entry.ID,
										PartProps: ui.PartProps{Style: ui.Style().
											Display("flex").
											FlexDirection("column").
											Gap(16).
											MaxWidth(720).
											FlexShrink(0)},
									}).
										Children(entry.Component).
										NativeNode())
								}
								return ui.Fragment(children)
							}).
							Display("flex").
							FlexDirection("column").
							Flex(1).
							MinHeight(0).
							Padding(20).
							Gap(16).
							OverflowY("scroll"),
					).
					Display("flex").
					FlexDirection("column").
					Flex(1).
					MinWidth(0).
					Height("100%").Node,
			})
		}).
		NativeNode()
}
