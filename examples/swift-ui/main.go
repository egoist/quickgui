package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
	"log"
	"strconv"
	"time"
)

type demoID string
type demo struct {
	ID          demoID
	Label       string
	Description string
}

var demos = []demo{{ID: "button", Label: "Button", Description: "A native SwiftUI button with an SF Symbol, Liquid Glass styling, and an asynchronous press event delivered to QuickGUI."}, {ID: "slider", Label: "Slider", Description: "A controlled native slider with a bounded range and discrete steps. Its value is owned by the QuickGUI signal below."}, {ID: "toggle", Label: "Toggle", Description: "A controlled SwiftUI toggle that reports its native on/off state through the hosted event queue."}, {ID: "progress-view", Label: "Progress View", Description: "A determinate SwiftUI progress view with a semantic label and a formatted current-value label."}, {ID: "stepper", Label: "Stepper", Description: "A bounded native stepper. SwiftUI performs the interaction and QuickGUI receives the updated numeric value."}, {ID: "segmented-control", Label: "Segmented Control", Description: "A native segmented picker using the neutral tabs role—the Liquid Glass treatment used for Xcode-style navigation."}, {ID: "picker", Label: "Picker", Description: "A native menu picker backed by typed options and a controlled string selection."}, {ID: "date-picker", Label: "Date Picker", Description: "A native field-style date and time picker whose value crosses the bridge as a Unix timestamp in milliseconds."}, {ID: "color-picker", Label: "Color Picker", Description: "A native color well with opacity support. SwiftUI selections are returned as RGBA hex strings."}, {ID: "gauge", Label: "Gauge", Description: "A native accessory-capacity gauge with minimum, maximum, and current-value labels."}, {ID: "text-field", Label: "Text Field", Description: "A controlled native text field that reports edits and Return-key submissions independently."}, {ID: "secure-field", Label: "Secure Field", Description: "The secure variant uses SwiftUI's native concealed editor while keeping the same QuickGUI value contract."}, {ID: "popover", Label: "Popover", Description: "A SwiftUI button presents a native popover, which reverse-hosts an ordinary interactive QuickGUI subtree."}}

func main() {
	if err := native.Run(func() {
		openMainWindow()
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				openMainWindow()
			}
		})
	}); err != nil {
		log.Fatal(err)
	}
}
func openMainWindow() {
	native.NewWindow(native.WindowOptions{
		Title:             "QuickGUI SwiftUI Components",
		Width:             920,
		Height:            680,
		MinimumWidth:      720,
		MinimumHeight:     500,
		Background:        "transparent",
		Vibrancy:          "sidebar",
		VisualEffectState: "followWindow",
		TitleBarStyle:     "hiddenInset",
		TrafficLightPosition: &native.Point{
			X: 16,
			Y: 18,
		},
		Component: Gallery,
	})
}
func Gallery() *ui.Element {
	state := newGalleryState()
	selected := func() *string {
		current := string(state.page())
		return &current
	}
	return ui.View().
		Child(func() *native.Node {
			tabs2 := ui.NewTabs(ui.TabsRootProps{
				Value: selected,
				OnValueChange: func(next string, _ *native.Event) {
					state.setPage(demoID(next))
				},
				Orientation: "vertical",
				Activation:  "manual",
				PartProps:   ui.PartProps{Style: ui.Style().Display("flex").FlexDirection("row").Width("100%").Height("100%")},
			})
			return tabs2.Root().
				Children(func() *native.Node {
					return ui.Fragment([]*native.Node{
						sidebar(tabs2, state).Node,
						pane(state, func() *native.Node {
							return renderDemo(state)
						}).Node,
					})
				}).
				NativeNode()
		}()).
		Display("flex").
		FlexDirection("row").
		Width("100%").
		Height("100%").
		BackgroundColor("transparent")
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
func sidebar(tabs *ui.TabsComponent, state *galleryState) *ui.Element {
	return ui.View().
		Children(
			ui.View().
				Child(ui.Text("SwiftUI").TextColor("#252a33").FontSize(13).FontWeight(700)).
				Display("flex").
				FlexDirection("row").
				AlignItems("center").
				Height(54).
				FlexShrink(0).
				PaddingLeft(82).
				AppRegion("drag"),
			ui.Text("COMPONENTS").
				FlexShrink(0).
				PaddingLeft(18).
				PaddingBottom(7).
				TextColor("#747b87").
				FontSize(10).
				FontWeight(700).
				LetterSpacing(0.7),
			ui.View().
				Child(tabs.List(ui.PartProps{Style: ui.Style().
					Display("flex").
					FlexDirection("column").
					Gap(2).
					PaddingLeft(9).
					PaddingRight(9).
					PaddingBottom(12)}).
					Child(func() *native.Node {
						return ui.For(
							func() []demo {
								return demos
							},
							func(item demo, index func() int) *native.Node {
								id := item.ID
								idx := index()
								return tabs.TabWith(ui.TabsTabProps{
									Value: string(id),
									Index: &idx,
									PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
										background, color, weight := "transparent", "#303641", any(400)
										var hover ui.StyleBuilder
										if state.page() == id {
											background, color, weight = "#2878d4", "#ffffff", 600
										} else {
											hover = ui.Style().BackgroundColor("#ffffff66")
										}
										return ui.Style().
											Display("flex").
											FlexDirection("row").
											AlignItems("center").
											Height(29).
											FlexShrink(0).
											PaddingLeft(10).
											PaddingRight(10).
											BorderRadius(7).
											Cursor("default").
											UserSelect("none").
											BackgroundColor(background).
											TextColor(color).
											FontWeight(weight).
											Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
												return s.Merge(hover)
											})
									}},
								}).
									Child(func() *ui.Element {
										return ui.Text(item.Label).FontSize(12)
									}).
									NativeNode()
							},
							func(item demo) any {
								return item.ID
							},
							nil,
						)
					}).
					NativeNode()).
				Display("flex").
				FlexDirection("column").
				Flex(1).
				MinHeight(0).
				OverflowY("scroll"),
			ui.View().
				Child(ui.Text(strconv.Itoa(len(demos))+" native components").
					TextColor("#747b87").
					FontSize(11)).
				FlexShrink(0).
				Padding(13).
				BorderWidth(1).
				BorderColor("#c9cbd0"),
		).
		Display("flex").
		FlexDirection("column").
		Width(220).
		Height("100%").
		FlexShrink(0).
		BackgroundColor("transparent").
		BorderWidth(1).
		BorderColor("#c9cbd0")
}
func pane(state *galleryState, body ui.Component) *ui.Element {
	return ui.View().
		Children(
			ui.View().
				Children(
					ui.Text(state.current().Label).TextColor("#20242c").FontSize(15).FontWeight(700),
					ui.Text("Native SwiftUI · QuickGUI state").TextColor("#858b96").FontSize(11),
				).
				Display("flex").
				FlexDirection("row").
				AlignItems("center").
				JustifyContent("space-between").
				Height(54).
				FlexShrink(0).
				PaddingLeft(22).
				PaddingRight(22).
				BorderWidth(1).
				BorderColor("#d7d8dc").
				AppRegion("drag"),
			ui.View().
				Child(body).
				Display("flex").
				FlexDirection("column").
				Flex(1).
				MinHeight(0).
				AlignItems("center").
				OverflowY("scroll").
				Padding(30),
		).
		Display("flex").
		FlexDirection("column").
		Flex(1).
		MinWidth(0).
		Height("100%").
		BackgroundColor("#f6f6f8")
}
func renderDemo(state *galleryState) *native.Node {
	return ui.Dynamic(func() ui.Component {
		page := state.current()
		return func() *ui.Element {
			return demoPage(page.Description, demoStatus(state), func() *native.Node {
				return demoControl(state)
			})
		}
	})
}
func demoPage(description string, status func() string, control ui.Component) *ui.Element {
	return ui.View().
		Children(
			ui.Text(description).TextColor("#5f6672").FontSize(14).LineHeight(21),
			ui.View().
				Children(
					ui.Text("LIVE SWIFTUI DEMO").
						TextColor("#858b96").
						FontSize(11).
						FontWeight(700).
						LetterSpacing(0.8),
					ui.View().
						Child(control).
						Display("flex").
						Flex(1).
						MinHeight(170).
						Width("100%").
						AlignItems("center").
						JustifyContent("center"),
				).
				Display("flex").
				FlexDirection("column").
				Width("100%").
				MinHeight(250).
				Padding(22).
				Gap(16).
				BorderWidth(1).
				BorderColor("#dedfe3").
				BorderRadius(14).
				BackgroundColor("#ffffff"),
			ui.View().
				Children(
					ui.Text("NATIVE STATE").TextColor("#727985").FontSize(11).FontWeight(700),
					ui.Text(status()).TextColor("#252a33").FontSize(12),
				).
				Display("flex").
				FlexDirection("row").
				AlignItems("center").
				JustifyContent("space-between").
				Gap(16).
				Width("100%").
				MinHeight(42).
				PaddingLeft(14).
				PaddingRight(14).
				BorderRadius(10).
				BackgroundColor("#eceef2"),
		).
		Display("flex").
		FlexDirection("column").
		Width("100%").
		MaxWidth(680).
		Gap(18)
}
func demoStatus(state *galleryState) func() string {
	return func() string {
		switch state.page() {
		case "button":
			suffix := "es"
			if state.presses() == 1 {
				suffix = ""
			}
			return strconv.Itoa(state.presses()) + " press" + suffix
		case "slider", "progress-view", "gauge":
			return "volume " + strconv.Itoa(int(state.volume()*100+0.5)) + "%"
		case "toggle":
			if state.notifications() {
				return "notifications on"
			}
			return "notifications off"
		case "stepper":
			return strconv.FormatFloat(state.copies(), 'g', -1, 64) + " copies"
		case "segmented-control":
			return "layout " + state.layout()
		case "picker":
			return "interval " + state.interval()
		case "date-picker":
			return time.UnixMilli(int64(state.scheduledAt())).UTC().Format(time.RFC3339Nano)
		case "color-picker":
			return "accent " + state.accent()
		case "text-field":
			return "value “" + state.name() + "” · last submitted " + state.submitted()
		case "secure-field":
			return strconv.Itoa(len(state.password())) + " characters · last submitted " + state.submitted()
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
func host(match any, style ui.StyleBuilder, child *native.Node) *native.Node {
	return ui.SwiftUI.Host(
		ui.SwiftUIHostProps{
			MatchContents: match,
			PartProps:     ui.PartProps{Style: style},
		},
		child,
	)
}
func demoControl(state *galleryState) *native.Node {
	var children []*native.Node
	wide := ui.Style().Width(440)
	field := ui.Style().Width(360)
	switch state.page() {
	case "button":
		children = append(children, host(true, ui.Style(), ui.SwiftUI.Button(ui.SwiftUIButtonProps{
			Label:       "Continue",
			SystemImage: "arrow.right",
			Modifiers: []ui.SwiftUIModifier{
				ui.SwiftUI.ButtonStyle("glass"),
				ui.SwiftUI.ControlSize("large"),
			},
			OnPress: func(*native.Event) {
				state.setPresses(state.presses() + 1)
			},
		})))
		return ui.Fragment(children)
	case "slider":
		children = append(children, host(ui.SwiftUIMatchContents{Vertical: true}, wide, ui.SwiftUI.Slider(ui.SwiftUISliderProps{
			Label: func() string {
				return "Volume " + strconv.Itoa(int(state.volume()*100+0.5)) + "%"
			},
			Value: state.volume,
			Min:   0,
			Max:   1,
			Step:  0.05,
			OnValueChange: func(next float64, _ *native.Event) {
				state.setVolume(next)
			},
		})))
		return ui.Fragment(children)
	case "toggle":
		children = append(children, host(true, ui.Style(), ui.SwiftUI.Toggle(ui.SwiftUIToggleProps{
			Label: "Notifications",
			IsOn:  state.notifications,
			OnIsOnChange: func(next bool, _ *native.Event) {
				state.setNotifications(next)
			},
		})))
		return ui.Fragment(children)
	case "progress-view":
		children = append(children, host(ui.SwiftUIMatchContents{Vertical: true}, wide, ui.SwiftUI.ProgressView(ui.SwiftUIProgressViewProps{
			Label: "Setup progress",
			Value: state.volume,
			Total: 1,
			CurrentValueLabel: func() string {
				return strconv.Itoa(int(state.volume()*100+0.5)) + "%"
			},
		})))
		return ui.Fragment(children)
	case "stepper":
		children = append(children, host(true, ui.Style(), ui.SwiftUI.Stepper(ui.SwiftUIStepperProps{
			Label: func() string {
				return "Copies: " + strconv.FormatFloat(state.copies(), 'g', -1, 64)
			},
			Value: state.copies,
			Min:   1,
			Max:   10,
			OnValueChange: func(next float64, _ *native.Event) {
				state.setCopies(next)
			},
		})))
		return ui.Fragment(children)
	case "segmented-control":
		children = append(children, host(ui.SwiftUIMatchContents{Vertical: true}, ui.Style().Width(340), ui.SwiftUI.SegmentedControl(ui.SwiftUISegmentedControlProps{
			Role:      "tabs",
			Selection: state.layout,
			Options: []ui.SwiftUIPickerOption{
				{Value: "list", Label: "List"},
				{Value: "grid", Label: "Grid"},
			},
			OnSelectionChange: func(next string, _ *native.Event) {
				state.setLayout(next)
			},
		})))
		return ui.Fragment(children)
	case "picker":
		children = append(children, host(true, ui.Style(), ui.SwiftUI.Picker(ui.SwiftUIPickerProps{
			Label:     "Report interval",
			Selection: state.interval,
			Style:     "menu",
			Options: []ui.SwiftUIPickerOption{
				{Value: "day", Label: "Daily"},
				{Value: "week", Label: "Weekly"},
				{Value: "month", Label: "Monthly"},
			},
			OnSelectionChange: func(next string, _ *native.Event) {
				state.setInterval(next)
			},
		})))
		return ui.Fragment(children)
	case "date-picker":
		children = append(children, host(true, ui.Style(), ui.SwiftUI.DatePicker(ui.SwiftUIDatePickerProps{
			Label:               "Schedule",
			Value:               state.scheduledAt,
			DisplayedComponents: "dateAndTime",
			Style:               "field",
			OnValueChange: func(next float64, _ *native.Event) {
				state.setScheduledAt(next)
			},
		})))
		return ui.Fragment(children)
	case "color-picker":
		children = append(children, host(true, ui.Style(), ui.SwiftUI.ColorPicker(ui.SwiftUIColorPickerProps{
			Label:     "Accent",
			Selection: state.accent,
			OnSelectionChange: func(next string, _ *native.Event) {
				state.setAccent(next)
			},
		})))
		return ui.Fragment(children)
	case "gauge":
		children = append(children, host(ui.SwiftUIMatchContents{Vertical: true}, wide, ui.SwiftUI.Gauge(ui.SwiftUIGaugeProps{
			Label: "Volume",
			Value: state.volume,
			Min:   0,
			Max:   1,
			Style: "accessoryLinearCapacity",
			CurrentValueLabel: func() string {
				return strconv.Itoa(int(state.volume()*100+0.5)) + "%"
			},
			MinimumValueLabel: "0%",
			MaximumValueLabel: "100%",
		})))
		return ui.Fragment(children)
	case "text-field":
		children = append(children, ui.View().
			Children(
				host(ui.SwiftUIMatchContents{Vertical: true}, field, ui.SwiftUI.TextField(ui.SwiftUITextFieldProps{
					Value:       state.name,
					Placeholder: "Name",
					OnValueChange: func(next string, _ *native.Event) {
						state.setName(next)
					},
					OnSubmit: func(*native.Event) {
						state.setSubmitted("text field")
					},
				})),
				ui.Input().
					Value(state.name).
					Placeholder("Framework input bound to the same value").
					OnInputEvent(func(event *native.Event) {
						if text, ok := event.ValueOK(); ok {
							state.setName(text)
						}
					}).
					Width(360).
					Height(28).
					FlexShrink(0).
					PaddingLeft(8).
					PaddingRight(8).
					TextColor("#111827").
					BackgroundColor("#ffffff").
					BorderWidth(1).
					BorderColor("#d1d5db").
					BorderRadius(6).
					FocusStyle(func(s ui.StyleBuilder) ui.StyleBuilder {
						return s.BorderColor("#2563eb").Outline("3px solid #2563eb55")
					}),
			).
			Display("flex").
			FlexDirection("column").
			Gap(16).
			AlignItems("center").Node)
		return ui.Fragment(children)
	case "secure-field":
		children = append(children, host(ui.SwiftUIMatchContents{Vertical: true}, field, ui.SwiftUI.SecureField(ui.SwiftUITextFieldProps{
			Value:       state.password,
			Placeholder: "Password",
			OnValueChange: func(next string, _ *native.Event) {
				state.setPassword(next)
			},
			OnSubmit: func(*native.Event) {
				state.setSubmitted("secure field")
			},
		})))
		return ui.Fragment(children)
	case "popover":
		children = append(children, host(true, ui.Style(), ui.SwiftUI.Popover.Root(
			ui.SwiftUIPopoverProps{
				IsPresented:         state.open,
				OnIsPresentedChange: state.setOpen,
				AttachmentAnchor:    "bottom",
				ArrowEdge:           "top",
			},
			func() *native.Node {
				return ui.Fragment([]*native.Node{
					ui.SwiftUI.Popover.Trigger(ui.SwiftUIPopoverTriggerProps{Render: func() *native.Node {
						return ui.SwiftUI.Button(ui.SwiftUIButtonProps{
							Label: "Open QuickGUI popover",
							Modifiers: []ui.SwiftUIModifier{
								ui.SwiftUI.ButtonStyle("glass"),
								ui.SwiftUI.ControlSize("large"),
							},
						})
					}}),
					ui.SwiftUI.Popover.Content(
						ui.SwiftUIPopoverContentProps{},
						func() *native.Node {
							return ui.SwiftUI.QuickGUIHostView(
								ui.SwiftUIQuickGUIHostViewProps{
									Width:  300,
									Height: 200,
								},
								func() *ui.Element {
									return ui.View().
										Children(
											ui.Text("QuickGUI inside SwiftUI").
												TextColor("#111827").
												FontSize(15).
												FlexShrink(0),
											ui.Input().
												Value(state.name).
												OnInputEvent(func(event *native.Event) {
													if text, ok := event.ValueOK(); ok {
														state.setName(text)
													}
												}).
												Width("100%").
												Height(36).
												FlexShrink(0).
												Padding(8).
												TextColor("#111827").
												BackgroundColor("#ffffff").
												BorderWidth(1).
												BorderColor("#d1d5db").
												BorderRadius(8),
											ui.Button().
												Child("Save "+state.name()).
												OnClick(func() {
													state.setOpen(false)
												}).
												Width("100%").
												Height(34).
												FlexShrink(0).
												Padding(8).
												TextColor("#ffffff").
												BackgroundColor("#2563eb").
												BorderRadius(8).
												JustifyContent("center"),
										).
										Display("flex").
										FlexDirection("column").
										Width(300).
										Height("100%").
										Padding(20).
										Gap(12).
										OverflowY("auto").
										BackgroundColor("transparent")
								},
							)
						},
					),
				})
			},
		)))
		return ui.Fragment(children)
	default:
		children = append(children, ui.Text("Unknown demo").Node)
		return ui.Fragment(children)
	}
	return ui.Fragment(children)
}
