package main

import (
	"fmt"
	"time"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/reactive"
	"github.com/egoist/quickgui/packages/go/ui"
)

type demoID string

type demo struct {
	ID          demoID
	Label       string
	Description string
}

var demos = []demo{
	{ID: "button", Label: "Button", Description: "A native SwiftUI button with an SF Symbol, Liquid Glass styling, and an asynchronous press event delivered to QuickGUI."},
	{ID: "slider", Label: "Slider", Description: "A controlled native slider with a bounded range and discrete steps. Its value is owned by the QuickGUI signal below."},
	{ID: "toggle", Label: "Toggle", Description: "A controlled SwiftUI toggle that reports its native on/off state through the hosted event queue."},
	{ID: "progress-view", Label: "Progress View", Description: "A determinate SwiftUI progress view with a semantic label and a formatted current-value label."},
	{ID: "stepper", Label: "Stepper", Description: "A bounded native stepper. SwiftUI performs the interaction and QuickGUI receives the updated numeric value."},
	{ID: "segmented-control", Label: "Segmented Control", Description: "A native segmented picker using the neutral tabs role—the Liquid Glass treatment used for Xcode-style navigation."},
	{ID: "picker", Label: "Picker", Description: "A native menu picker backed by typed options and a controlled string selection."},
	{ID: "date-picker", Label: "Date Picker", Description: "A native field-style date and time picker whose value crosses the bridge as a Unix timestamp in milliseconds."},
	{ID: "color-picker", Label: "Color Picker", Description: "A native color well with opacity support. SwiftUI selections are returned as RGBA hex strings."},
	{ID: "gauge", Label: "Gauge", Description: "A native accessory-capacity gauge with minimum, maximum, and current-value labels."},
	{ID: "text-field", Label: "Text Field", Description: "A controlled native text field that reports edits and Return-key submissions independently."},
	{ID: "secure-field", Label: "Secure Field", Description: "The secure variant uses SwiftUI's native concealed editor while keeping the same QuickGUI value contract."},
	{ID: "popover", Label: "Popover", Description: "A SwiftUI button presents a native popover, which reverse-hosts an ordinary interactive QuickGUI subtree."},
}

func main() {
	native.Run(func() {
		openMainWindow()
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				openMainWindow()
			}
		})
	})
}

func openMainWindow() {
	native.NewWindow(native.WindowOptions{
		Title:                "QuickGUI SwiftUI Components",
		Width:                920,
		Height:               680,
		MinimumWidth:         720,
		MinimumHeight:        500,
		Background:           "transparent",
		Vibrancy:             "sidebar",
		VisualEffectState:    "followWindow",
		TitleBarStyle:        "hiddenInset",
		TrafficLightPosition: &native.Point{X: 16, Y: 18},
		Renderer:             ui.CreateRenderer(Gallery),
	})
}

func Gallery() *native.Node {
	state := newGalleryState()
	selected := func() *string {
		current := string(state.page())
		return &current
	}
	return ui.View(ui.Props{
		Style: ui.Style{Display: "flex", FlexDirection: "row", Width: "100%", Height: "100%", BackgroundColor: "transparent"},
		Children: ui.Tabs.Root(ui.TabsRootProps{
			Value:         selected,
			OnValueChange: func(next string, _ *native.Event) { state.setPage(demoID(next)) },
			Orientation:   "vertical",
			Activation:    "manual",
			PartProps: ui.PartProps{
				Style: ui.Style{Display: "flex", FlexDirection: "row", Width: "100%", Height: "100%"},
				Children: func() *native.Node {
					return ui.Fragment([]*native.Node{sidebar(state), pane(state, func() *native.Node {
						return renderDemo(state)
					})})
				},
			},
		}),
	})
}

type galleryState struct {
	page             reactive.Accessor[demoID]
	setPage          reactive.Setter[demoID]
	presses          reactive.Accessor[int]
	setPresses       reactive.Setter[int]
	open             reactive.Accessor[bool]
	setOpen          reactive.Setter[bool]
	name             reactive.Accessor[string]
	setName          reactive.Setter[string]
	password         reactive.Accessor[string]
	setPassword      reactive.Setter[string]
	submitted        reactive.Accessor[string]
	setSubmitted     reactive.Setter[string]
	volume           reactive.Accessor[float64]
	setVolume        reactive.Setter[float64]
	notifications    reactive.Accessor[bool]
	setNotifications reactive.Setter[bool]
	copies           reactive.Accessor[float64]
	setCopies        reactive.Setter[float64]
	layout           reactive.Accessor[string]
	setLayout        reactive.Setter[string]
	interval         reactive.Accessor[string]
	setInterval      reactive.Setter[string]
	scheduledAt      reactive.Accessor[float64]
	setScheduledAt   reactive.Setter[float64]
	accent           reactive.Accessor[string]
	setAccent        reactive.Setter[string]
}

func newGalleryState() *galleryState {
	s := &galleryState{}
	s.page, s.setPage = ui.CreateSignal(demoID("button"))
	s.presses, s.setPresses = ui.CreateSignal(0)
	s.open, s.setOpen = ui.CreateSignal(false)
	s.name, s.setName = ui.CreateSignal("Ada")
	s.password, s.setPassword = ui.CreateSignal("")
	s.submitted, s.setSubmitted = ui.CreateSignal("none")
	s.volume, s.setVolume = ui.CreateSignal(0.4)
	s.notifications, s.setNotifications = ui.CreateSignal(true)
	s.copies, s.setCopies = ui.CreateSignal(2.0)
	s.layout, s.setLayout = ui.CreateSignal("grid")
	s.interval, s.setInterval = ui.CreateSignal("week")
	s.scheduledAt, s.setScheduledAt = ui.CreateSignal(float64(time.Now().UnixMilli()))
	s.accent, s.setAccent = ui.CreateSignal("#3366ffff")
	return s
}

func (s *galleryState) current() demo {
	for _, item := range demos {
		if item.ID == s.page() {
			return item
		}
	}
	return demos[0]
}

func sidebar(state *galleryState) *native.Node {
	return ui.View(ui.Props{
		Style: ui.Style{Display: "flex", FlexDirection: "column", Width: 220, Height: "100%", FlexShrink: 0, BackgroundColor: "transparent", BorderWidth: 1, BorderColor: "#c9cbd0"},
		Children: []any{
			ui.View(ui.Props{
				Style:    ui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Height: 54, FlexShrink: 0, PaddingLeft: 82, AppRegion: "drag"},
				Children: ui.Text(ui.Props{Style: ui.Style{Color: "#252a33", FontSize: 13, FontWeight: 700}, Children: "SwiftUI"}),
			}),
			ui.Text(ui.Props{Style: ui.Style{FlexShrink: 0, PaddingLeft: 18, PaddingBottom: 7, Color: "#747b87", FontSize: 10, FontWeight: 700, LetterSpacing: 0.7}, Children: "COMPONENTS"}),
			ui.View(ui.Props{
				Style: ui.Style{Display: "flex", FlexDirection: "column", Flex: 1, MinHeight: 0, OverflowY: "scroll"},
				Children: ui.Tabs.List(ui.PartProps{
					Style: ui.Style{Display: "flex", FlexDirection: "column", Gap: 2, PaddingLeft: 9, PaddingRight: 9, PaddingBottom: 12},
					Children: func() *native.Node {
						return ui.For(func() []demo { return demos }, func(item demo, index func() int) *native.Node {
							id := item.ID
							idx := index()
							return ui.Tabs.Tab(ui.TabsTabProps{
								Value: string(id),
								Index: &idx,
								PartProps: ui.PartProps{
									Style: func() ui.Style {
										background, color, weight := "transparent", "#303641", any(400)
										if state.page() == id {
											background, color, weight = "#2878d4", "#ffffff", 600
										}
										return ui.Style{
											Display: "flex", FlexDirection: "row", AlignItems: "center", Height: 29, FlexShrink: 0,
											PaddingLeft: 10, PaddingRight: 10, BorderRadius: 7, Cursor: "default", UserSelect: "none",
											BackgroundColor: background, Color: color, FontWeight: weight,
											Hover: &ui.Style{BackgroundColor: "#ffffff66"},
										}
									},
									Children: func() *native.Node {
										return ui.Text(ui.Props{Style: ui.Style{FontSize: 12}, Children: item.Label})
									},
								},
							})
						}, func(item demo) any { return item.ID }, nil)
					},
				}),
			}),
			ui.View(ui.Props{
				Style:    ui.Style{FlexShrink: 0, Padding: 13, BorderWidth: 1, BorderColor: "#c9cbd0"},
				Children: ui.Text(ui.Props{Style: ui.Style{Color: "#747b87", FontSize: 11}, Children: fmt.Sprintf("%d native components", len(demos))}),
			}),
		},
	})
}

func pane(state *galleryState, body func() *native.Node) *native.Node {
	return ui.View(ui.Props{
		Style: ui.Style{Display: "flex", FlexDirection: "column", Flex: 1, MinWidth: 0, Height: "100%", BackgroundColor: "#f6f6f8"},
		Children: []any{
			ui.View(ui.Props{
				Style: ui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", JustifyContent: "space-between", Height: 54, FlexShrink: 0, PaddingLeft: 22, PaddingRight: 22, BorderWidth: 1, BorderColor: "#d7d8dc", AppRegion: "drag"},
				Children: []any{
					ui.Text(ui.Props{Style: ui.Style{Color: "#20242c", FontSize: 15, FontWeight: 700}, Children: func() string { return state.current().Label }}),
					ui.Text(ui.Props{Style: ui.Style{Color: "#858b96", FontSize: 11}, Children: "Native SwiftUI · QuickGUI state"}),
				},
			}),
			ui.View(ui.Props{
				Style:    ui.Style{Display: "flex", FlexDirection: "column", Flex: 1, MinHeight: 0, AlignItems: "center", OverflowY: "scroll", Padding: 30},
				Children: body(),
			}),
		},
	})
}

func renderDemo(state *galleryState) *native.Node {
	return demoPage(state.current().Description, demoStatus(state), demoControl(state))
}

func demoPage(description string, status func() string, control *native.Node) *native.Node {
	return ui.View(ui.Props{
		Style: ui.Style{Display: "flex", FlexDirection: "column", Width: "100%", MaxWidth: 680, Gap: 18},
		Children: []any{
			ui.Text(ui.Props{Style: ui.Style{Color: "#5f6672", FontSize: 14, LineHeight: 21}, Children: description}),
			ui.View(ui.Props{
				Style: ui.Style{Display: "flex", FlexDirection: "column", Width: "100%", MinHeight: 250, Padding: 22, Gap: 16, BorderWidth: 1, BorderColor: "#dedfe3", BorderRadius: 14, BackgroundColor: "#ffffff"},
				Children: []any{
					ui.Text(ui.Props{Style: ui.Style{Color: "#858b96", FontSize: 11, FontWeight: 700, LetterSpacing: 0.8}, Children: "LIVE SWIFTUI DEMO"}),
					ui.View(ui.Props{
						Style:    ui.Style{Display: "flex", Flex: 1, MinHeight: 170, Width: "100%", AlignItems: "center", JustifyContent: "center"},
						Children: control,
					}),
				},
			}),
			ui.View(ui.Props{
				Style: ui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", JustifyContent: "space-between", Gap: 16, Width: "100%", MinHeight: 42, PaddingLeft: 14, PaddingRight: 14, BorderRadius: 10, BackgroundColor: "#eceef2"},
				Children: []any{
					ui.Text(ui.Props{Style: ui.Style{Color: "#727985", FontSize: 11, FontWeight: 700}, Children: "NATIVE STATE"}),
					ui.Text(ui.Props{Style: ui.Style{Color: "#252a33", FontSize: 12}, Children: status}),
				},
			}),
		},
	})
}

func demoStatus(state *galleryState) func() string {
	return func() string {
		switch state.page() {
		case "button":
			suffix := "es"
			if state.presses() == 1 {
				suffix = ""
			}
			return fmt.Sprintf("%d press%s", state.presses(), suffix)
		case "slider", "progress-view", "gauge":
			return fmt.Sprintf("volume %d%%", int(state.volume()*100+0.5))
		case "toggle":
			if state.notifications() {
				return "notifications on"
			}
			return "notifications off"
		case "stepper":
			return fmt.Sprintf("%g copies", state.copies())
		case "segmented-control":
			return "layout " + state.layout()
		case "picker":
			return "interval " + state.interval()
		case "date-picker":
			return time.UnixMilli(int64(state.scheduledAt())).UTC().Format(time.RFC3339Nano)
		case "color-picker":
			return "accent " + state.accent()
		case "text-field":
			return fmt.Sprintf("value “%s” · last submitted %s", state.name(), state.submitted())
		case "secure-field":
			return fmt.Sprintf("%d characters · last submitted %s", len(state.password()), state.submitted())
		case "popover":
			if state.open() {
				return "popover presented"
			}
			return "popover dismissed"
		default:
			return ""
		}
	}
}

func host(match any, style ui.Style, child *native.Node) *native.Node {
	return ui.SwiftUI.Host(ui.SwiftUIHostProps{
		MatchContents: match,
		PartProps: ui.PartProps{
			Style:    style,
			Children: func() *native.Node { return child },
		},
	})
}

func demoControl(state *galleryState) *native.Node {
	wide := ui.Style{Width: 440}
	field := ui.Style{Width: 360}
	switch state.page() {
	case "button":
		return host(true, ui.Style{}, ui.SwiftUI.Button(ui.SwiftUIButtonProps{
			Label: "Continue", SystemImage: "arrow.right",
			Modifiers: []ui.SwiftUIModifier{ui.SwiftUI.ButtonStyle("glass"), ui.SwiftUI.ControlSize("large")},
			OnPress:   func(*native.Event) { state.setPresses(state.presses() + 1) },
		}))
	case "slider":
		return host(ui.SwiftUIMatchContents{Vertical: true}, wide, ui.SwiftUI.Slider(ui.SwiftUISliderProps{
			Label: func() string { return fmt.Sprintf("Volume %d%%", int(state.volume()*100+0.5)) },
			Value: state.volume, Min: 0, Max: 1, Step: 0.05, OnValueChange: func(next float64, _ *native.Event) { state.setVolume(next) },
		}))
	case "toggle":
		return host(true, ui.Style{}, ui.SwiftUI.Toggle(ui.SwiftUIToggleProps{
			Label: "Notifications", IsOn: state.notifications, OnIsOnChange: func(next bool, _ *native.Event) { state.setNotifications(next) },
		}))
	case "progress-view":
		return host(ui.SwiftUIMatchContents{Vertical: true}, wide, ui.SwiftUI.ProgressView(ui.SwiftUIProgressViewProps{
			Label: "Setup progress", Value: state.volume, Total: 1,
			CurrentValueLabel: func() string { return fmt.Sprintf("%d%%", int(state.volume()*100+0.5)) },
		}))
	case "stepper":
		return host(true, ui.Style{}, ui.SwiftUI.Stepper(ui.SwiftUIStepperProps{
			Label: func() string { return fmt.Sprintf("Copies: %g", state.copies()) },
			Value: state.copies, Min: 1, Max: 10, OnValueChange: func(next float64, _ *native.Event) { state.setCopies(next) },
		}))
	case "segmented-control":
		return host(ui.SwiftUIMatchContents{Vertical: true}, ui.Style{Width: 340}, ui.SwiftUI.SegmentedControl(ui.SwiftUISegmentedControlProps{
			Role: "tabs", Selection: state.layout,
			Options:           []ui.SwiftUIPickerOption{{Value: "list", Label: "List"}, {Value: "grid", Label: "Grid"}},
			OnSelectionChange: func(next string, _ *native.Event) { state.setLayout(next) },
		}))
	case "picker":
		return host(true, ui.Style{}, ui.SwiftUI.Picker(ui.SwiftUIPickerProps{
			Label: "Report interval", Selection: state.interval, Style: "menu",
			Options:           []ui.SwiftUIPickerOption{{Value: "day", Label: "Daily"}, {Value: "week", Label: "Weekly"}, {Value: "month", Label: "Monthly"}},
			OnSelectionChange: func(next string, _ *native.Event) { state.setInterval(next) },
		}))
	case "date-picker":
		return host(true, ui.Style{}, ui.SwiftUI.DatePicker(ui.SwiftUIDatePickerProps{
			Label: "Schedule", Value: state.scheduledAt, DisplayedComponents: "dateAndTime", Style: "field",
			OnValueChange: func(next float64, _ *native.Event) { state.setScheduledAt(next) },
		}))
	case "color-picker":
		return host(true, ui.Style{}, ui.SwiftUI.ColorPicker(ui.SwiftUIColorPickerProps{
			Label: "Accent", Selection: state.accent, OnSelectionChange: func(next string, _ *native.Event) { state.setAccent(next) },
		}))
	case "gauge":
		return host(ui.SwiftUIMatchContents{Vertical: true}, wide, ui.SwiftUI.Gauge(ui.SwiftUIGaugeProps{
			Label: "Volume", Value: state.volume, Min: 0, Max: 1, Style: "accessoryLinearCapacity",
			CurrentValueLabel: func() string { return fmt.Sprintf("%d%%", int(state.volume()*100+0.5)) },
			MinimumValueLabel: "0%", MaximumValueLabel: "100%",
		}))
	case "text-field":
		return ui.View(ui.Props{
			Style: ui.Style{Display: "flex", FlexDirection: "column", Gap: 16, AlignItems: "center"},
			Children: []any{
				host(ui.SwiftUIMatchContents{Vertical: true}, field, ui.SwiftUI.TextField(ui.SwiftUITextFieldProps{
					Value: state.name, Placeholder: "Name",
					OnValueChange: func(next string, _ *native.Event) { state.setName(next) },
					OnSubmit:      func(*native.Event) { state.setSubmitted("text field") },
				})),
				ui.Input(ui.Props{
					Value: state.name, Placeholder: "Framework input bound to the same value",
					OnInput: func(event *native.Event) {
						if text, ok := event.ValueOK(); ok {
							state.setName(text)
						}
					},
					Style: ui.Style{Width: 360, Height: 28, FlexShrink: 0, PaddingLeft: 8, PaddingRight: 8, Color: "#111827", BackgroundColor: "#ffffff", BorderWidth: 1, BorderColor: "#d1d5db", BorderRadius: 6, Focus: &ui.Style{BorderColor: "#2563eb", Outline: "3px solid #2563eb55"}},
				}),
			},
		})
	case "secure-field":
		return host(ui.SwiftUIMatchContents{Vertical: true}, field, ui.SwiftUI.SecureField(ui.SwiftUITextFieldProps{
			Value: state.password, Placeholder: "Password",
			OnValueChange: func(next string, _ *native.Event) { state.setPassword(next) },
			OnSubmit:      func(*native.Event) { state.setSubmitted("secure field") },
		}))
	case "popover":
		return host(true, ui.Style{}, ui.SwiftUI.Popover.Root(ui.SwiftUIPopoverProps{
			IsPresented:         state.open,
			OnIsPresentedChange: state.setOpen,
			AttachmentAnchor:    "bottom",
			ArrowEdge:           "top",
			Children: func() *native.Node {
				return ui.Fragment([]*native.Node{
					ui.SwiftUI.Popover.Trigger(ui.SwiftUIPopoverTriggerProps{
						Render: func() *native.Node {
							return ui.SwiftUI.Button(ui.SwiftUIButtonProps{
								Label:     "Open QuickGUI popover",
								Modifiers: []ui.SwiftUIModifier{ui.SwiftUI.ButtonStyle("glass"), ui.SwiftUI.ControlSize("large")},
							})
						},
					}),
					ui.SwiftUI.Popover.Content(ui.SwiftUIPopoverContentProps{
						Children: func() *native.Node {
							return ui.SwiftUI.QuickGUIHostView(ui.SwiftUIQuickGUIHostViewProps{
								Width: 300, Height: 200,
								Children: func() *native.Node {
									return ui.View(ui.Props{
										Style: ui.Style{Display: "flex", FlexDirection: "column", Width: 300, Height: "100%", Padding: 20, Gap: 12, OverflowY: "auto", BackgroundColor: "transparent"},
										Children: []any{
											ui.Text(ui.Props{Style: ui.Style{Color: "#111827", FontSize: 15, FlexShrink: 0}, Children: "QuickGUI inside SwiftUI"}),
											ui.Input(ui.Props{
												Value: state.name,
												OnInput: func(event *native.Event) {
													if text, ok := event.ValueOK(); ok {
														state.setName(text)
													}
												},
												Style: ui.Style{Width: "100%", Height: 36, FlexShrink: 0, Padding: 8, Color: "#111827", BackgroundColor: "#ffffff", BorderWidth: 1, BorderColor: "#d1d5db", BorderRadius: 8},
											}),
											ui.Button(ui.Props{
												OnClick:  func(*native.Event) { state.setOpen(false) },
												Style:    ui.Style{Width: "100%", Height: 34, FlexShrink: 0, Padding: 8, Color: "#ffffff", BackgroundColor: "#2563eb", BorderRadius: 8, JustifyContent: "center"},
												Children: func() string { return "Save " + state.name() },
											}),
										},
									})
								},
							})
						},
					}),
				})
			},
		}))
	default:
		return ui.Text(ui.Props{Children: "Unknown demo"})
	}
}
