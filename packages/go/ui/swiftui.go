package ui

import (
	"fmt"
	"math"
	"strconv"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// SwiftUI is the macOS SwiftUI host family. Controls use dedicated node tags, not NativeParts.
var SwiftUI = swiftUIAPI{Popover: SwiftUIPopover}

type swiftUIAPI struct {
	Popover swiftUIPopoverAPI
}

// SwiftUIMatchContents selects intrinsic sizing on one or both axes.
type SwiftUIMatchContents struct {
	Horizontal bool
	Vertical   bool
}

// SwiftUIViewProps are the modifier props every SwiftUI control accepts.
type SwiftUIViewProps struct {
	TestID    any
	Modifiers any
	Ref       func(*native.Node)
}

type SwiftUIHostProps struct {
	PartProps
	MatchContents any
}

type SwiftUIButtonProps struct {
	TestID      any
	Modifiers   any
	Ref         func(*native.Node)
	Label       any
	SystemImage string
	Role        string
	OnPress     func(*native.Event)
	Target      string
	Children    func() *native.Node
}

type SwiftUISliderProps struct {
	TestID        any
	Modifiers     any
	Ref           func(*native.Node)
	Value         any
	Min           any
	Max           any
	Step          any
	Label         any
	OnValueChange func(float64, *native.Event)
}

type SwiftUIToggleProps struct {
	TestID       any
	Modifiers    any
	Ref          func(*native.Node)
	IsOn         any
	Label        any
	OnIsOnChange func(bool, *native.Event)
}

type SwiftUIProgressViewProps struct {
	TestID            any
	Modifiers         any
	Ref               func(*native.Node)
	Value             any
	Total             any
	Label             any
	CurrentValueLabel any
}

type SwiftUIStepperProps struct {
	TestID        any
	Modifiers     any
	Ref           func(*native.Node)
	Value         any
	Min           any
	Max           any
	Step          any
	Label         any
	OnValueChange func(float64, *native.Event)
}

type SwiftUITextFieldProps struct {
	TestID        any
	Modifiers     any
	Ref           func(*native.Node)
	Value         any
	Placeholder   any
	OnValueChange func(string, *native.Event)
	OnSubmit      func(*native.Event)
}

type SwiftUIPickerOption struct {
	Value       string `json:"value"`
	Label       string `json:"label"`
	SystemImage string `json:"systemImage,omitempty"`
	Disabled    bool   `json:"disabled,omitempty"`
}

type SwiftUIPickerProps struct {
	TestID            any
	Modifiers         any
	Ref               func(*native.Node)
	Selection         any
	Options           any
	Label             any
	Style             string
	OnSelectionChange func(string, *native.Event)
}

type SwiftUISegmentedControlProps struct {
	TestID            any
	Modifiers         any
	Ref               func(*native.Node)
	Selection         any
	Options           any
	Label             any
	Role              string
	OnSelectionChange func(string, *native.Event)
}

type SwiftUIDatePickerProps struct {
	TestID              any
	Modifiers           any
	Ref                 func(*native.Node)
	Value               any
	Min                 any
	Max                 any
	Label               any
	DisplayedComponents string
	Style               string
	OnValueChange       func(float64, *native.Event)
}

type SwiftUIColorPickerProps struct {
	TestID            any
	Modifiers         any
	Ref               func(*native.Node)
	Selection         any
	Label             any
	SupportsOpacity   any
	OnSelectionChange func(string, *native.Event)
}

type SwiftUIGaugeProps struct {
	TestID            any
	Modifiers         any
	Ref               func(*native.Node)
	Value             any
	Min               any
	Max               any
	Label             any
	CurrentValueLabel any
	MinimumValueLabel any
	MaximumValueLabel any
	Style             string
}

type SwiftUIQuickGUIHostViewProps struct {
	TestID        any
	Modifiers     any
	Ref           func(*native.Node)
	Children      func() *native.Node
	Width         any
	Height        any
	MatchContents any
	Background    any
}

type SwiftUIPopoverProps struct {
	TestID              any
	Modifiers           any
	Ref                 func(*native.Node)
	Children            func() *native.Node
	IsPresented         any
	OnIsPresentedChange func(bool)
	AttachmentAnchor    string
	ArrowEdge           string
}

type SwiftUIPopoverTriggerProps struct {
	Render  func() *native.Node
	OnPress func(*native.Event)
	Ref     func(*native.Node)
}

type SwiftUIPopoverContentProps struct {
	Children func() *native.Node
	Ref      func(*native.Node)
}

func bindSwiftUINumber(node *native.Node, code uint16, value any) {
	if value != nil {
		bindNumber(node, code, value)
	}
}

func bindSwiftUIString(node *native.Node, code uint16, value any) {
	if value != nil {
		bindString(node, code, value)
	}
}

func bindSwiftUIBool(node *native.Node, code uint16, value any) {
	if value != nil {
		bindExplicitBool(node, code, value)
	}
}

func swiftView(testID, modifiers any, ref func(*native.Node)) SwiftUIViewProps {
	return SwiftUIViewProps{TestID: testID, Modifiers: modifiers, Ref: ref}
}

func makeSwiftUI(tag uint8, props SwiftUIViewProps) *native.Node {
	node := native.CreateElement(tag)
	bindSwiftUIString(node, protocol.SwiftUITestId, props.TestID)
	if props.Modifiers != nil {
		switch typed := props.Modifiers.(type) {
		case func() []SwiftUIModifier:
			reactive.CreateRenderEffect(func() {
				setJson(node, protocol.SwiftUIModifiers, 16*1024, typed())
			})
		case []SwiftUIModifier:
			setJson(node, protocol.SwiftUIModifiers, 16*1024, typed)
		default:
			panic(fmt.Sprintf("QuickGUI SwiftUI modifiers %T are not a modifier list or accessor", props.Modifiers))
		}
	}
	return node
}

func finishSwiftUI(node *native.Node, ref func(*native.Node)) *native.Node {
	if ref != nil {
		ref(node)
	}
	return node
}

func insertSwiftUILabel(node *native.Node, value any) {
	if value == nil {
		return
	}
	switch typed := value.(type) {
	case func() string:
		native.InsertNode(node, DynamicText(typed), nil)
	case reactive.Accessor[string]:
		native.InsertNode(node, DynamicText(typed), nil)
	case string:
		native.InsertNode(node, native.CreateText(typed), nil)
	default:
		native.InsertNode(node, native.CreateText(fmt.Sprint(typed)), nil)
	}
}

func applyMatchContents(node *native.Node, value any) {
	horizontal, vertical := matchContentsAxes(value)
	setExplicitBool(node, protocol.SwiftUIMatchContentsHorizontal, horizontal)
	setExplicitBool(node, protocol.SwiftUIMatchContentsVertical, vertical)
}

func matchContentsAxes(value any) (horizontal, vertical bool) {
	switch typed := value.(type) {
	case bool:
		return typed, typed
	case SwiftUIMatchContents:
		return typed.Horizontal, typed.Vertical
	case *SwiftUIMatchContents:
		if typed == nil {
			return false, false
		}
		return typed.Horizontal, typed.Vertical
	default:
		return false, false
	}
}

func listenNumberInput(node *native.Node, callback func(float64, *native.Event)) {
	if callback == nil {
		return
	}
	setListener(node, protocol.EventInput, func(event *native.Event) {
		payload, ok := event.ValueOK()
		if !ok {
			return
		}
		value, err := strconv.ParseFloat(payload, 64)
		if err != nil || math.IsNaN(value) || math.IsInf(value, 0) {
			return
		}
		callback(value, event)
	})
}

func listenStringInput(node *native.Node, callback func(string, *native.Event)) {
	if callback == nil {
		return
	}
	setListener(node, protocol.EventInput, func(event *native.Event) {
		payload, _ := event.ValueOK()
		callback(payload, event)
	})
}

func bindCivilDate(node *native.Node, code uint16, value any) {
	if value == nil {
		return
	}
	write := func(milliseconds float64) {
		if math.IsNaN(milliseconds) || math.IsInf(milliseconds, 0) {
			panic("SwiftUI DatePicker requires a valid Date")
		}
		setString(node, code, strconv.FormatFloat(milliseconds/1000, 'f', -1, 64))
	}
	switch typed := value.(type) {
	case func() float64:
		reactive.CreateRenderEffect(func() { write(typed()) })
	case reactive.Accessor[float64]:
		reactive.CreateRenderEffect(func() { write(typed()) })
	case func() int64:
		reactive.CreateRenderEffect(func() { write(float64(typed())) })
	case float64:
		write(typed)
	case int64:
		write(float64(typed))
	case int:
		write(float64(typed))
	default:
		panic(fmt.Sprintf("QuickGUI expected a DatePicker timestamp or accessor, got %T", value))
	}
}

// Host is a native NSHostingView laid out as one QuickGUI leaf.
func (swiftUIAPI) Host(props SwiftUIHostProps) *native.Node {
	node := native.CreateElement(protocol.TagSwiftUIHost)
	if props.Style != nil {
		bindPartStyle(node, props.Style)
	}
	applyMatchContents(node, props.MatchContents)
	return finishPart(node, props.PartProps)
}

func (swiftUIAPI) Button(props SwiftUIButtonProps) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUIButton, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUIString(node, protocol.Value, props.Label)
	setString(node, protocol.SwiftUISystemImage, props.SystemImage)
	setString(node, protocol.Role, props.Role)
	setString(node, protocol.SwiftUITarget, props.Target)
	if props.OnPress != nil {
		setListener(node, protocol.EventClick, props.OnPress)
	}
	if props.Children != nil {
		if child := props.Children(); child != nil {
			native.InsertNode(node, child, nil)
		}
	}
	return finishSwiftUI(node, props.Ref)
}

func (swiftUIAPI) Slider(props SwiftUISliderProps) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUISlider, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUINumber(node, protocol.Value, props.Value)
	bindSwiftUINumber(node, protocol.Minimum, props.Min)
	bindSwiftUINumber(node, protocol.Maximum, props.Max)
	bindSwiftUINumber(node, protocol.Step, props.Step)
	insertSwiftUILabel(node, props.Label)
	listenNumberInput(node, props.OnValueChange)
	return finishSwiftUI(node, props.Ref)
}

func (swiftUIAPI) Stepper(props SwiftUIStepperProps) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUIStepper, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUINumber(node, protocol.Value, props.Value)
	bindSwiftUINumber(node, protocol.Minimum, props.Min)
	bindSwiftUINumber(node, protocol.Maximum, props.Max)
	bindSwiftUINumber(node, protocol.Step, props.Step)
	insertSwiftUILabel(node, props.Label)
	listenNumberInput(node, props.OnValueChange)
	return finishSwiftUI(node, props.Ref)
}

func (swiftUIAPI) Toggle(props SwiftUIToggleProps) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUIToggle, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUIBool(node, protocol.Checked, props.IsOn)
	insertSwiftUILabel(node, props.Label)
	if props.OnIsOnChange != nil {
		setListener(node, protocol.EventInput, func(event *native.Event) {
			payload, ok := event.ValueOK()
			if ok && (payload == "true" || payload == "false") {
				props.OnIsOnChange(payload == "true", event)
			}
		})
	}
	return finishSwiftUI(node, props.Ref)
}

func (swiftUIAPI) ProgressView(props SwiftUIProgressViewProps) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUIProgressView, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUINumber(node, protocol.Value, props.Value)
	bindSwiftUINumber(node, protocol.Maximum, props.Total)
	bindSwiftUIString(node, protocol.ValueText, props.CurrentValueLabel)
	insertSwiftUILabel(node, props.Label)
	return finishSwiftUI(node, props.Ref)
}

func swiftUITextField(props SwiftUITextFieldProps, secure bool) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUITextField, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUIString(node, protocol.Value, props.Value)
	bindSwiftUIString(node, protocol.Placeholder, props.Placeholder)
	if secure {
		setInputType(node, "password")
	} else {
		setInputType(node, "text")
	}
	listenStringInput(node, props.OnValueChange)
	if props.OnSubmit != nil {
		setListener(node, protocol.EventSubmit, props.OnSubmit)
	}
	return finishSwiftUI(node, props.Ref)
}

func (swiftUIAPI) TextField(props SwiftUITextFieldProps) *native.Node {
	return swiftUITextField(props, false)
}

func (swiftUIAPI) SecureField(props SwiftUITextFieldProps) *native.Node {
	return swiftUITextField(props, true)
}

func swiftUIPicker(props SwiftUIPickerProps, style, role string) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUIPicker, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUIString(node, protocol.Value, props.Selection)
	setString(node, protocol.SwiftUIPickerStyle, style)
	setString(node, protocol.Role, role)
	if props.Options != nil {
		switch typed := props.Options.(type) {
		case func() []SwiftUIPickerOption:
			reactive.CreateRenderEffect(func() {
				setJson(node, protocol.Items, 64*1024, typed())
			})
		case []SwiftUIPickerOption:
			setJson(node, protocol.Items, 64*1024, typed)
		default:
			panic(fmt.Sprintf("QuickGUI SwiftUI picker options %T are not an option list or accessor", props.Options))
		}
	}
	insertSwiftUILabel(node, props.Label)
	listenStringInput(node, props.OnSelectionChange)
	return finishSwiftUI(node, props.Ref)
}

func (swiftUIAPI) Picker(props SwiftUIPickerProps) *native.Node {
	return swiftUIPicker(props, props.Style, "")
}

func (swiftUIAPI) SegmentedControl(props SwiftUISegmentedControlProps) *native.Node {
	return swiftUIPicker(SwiftUIPickerProps{
		TestID:            props.TestID,
		Modifiers:         props.Modifiers,
		Ref:               props.Ref,
		Selection:         props.Selection,
		Options:           props.Options,
		Label:             props.Label,
		OnSelectionChange: props.OnSelectionChange,
	}, "segmented", props.Role)
}

func (swiftUIAPI) DatePicker(props SwiftUIDatePickerProps) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUIDatePicker, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindCivilDate(node, protocol.CivilValue, props.Value)
	bindCivilDate(node, protocol.CivilMinimum, props.Min)
	bindCivilDate(node, protocol.CivilMaximum, props.Max)
	setString(node, protocol.SwiftUIDatePickerComponents, props.DisplayedComponents)
	setString(node, protocol.SwiftUIDatePickerStyle, props.Style)
	insertSwiftUILabel(node, props.Label)
	if props.OnValueChange != nil {
		listenNumberInput(node, func(value float64, event *native.Event) {
			props.OnValueChange(value*1000, event)
		})
	}
	return finishSwiftUI(node, props.Ref)
}

func (swiftUIAPI) ColorPicker(props SwiftUIColorPickerProps) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUIColorPicker, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUIString(node, protocol.Value, props.Selection)
	bindSwiftUIBool(node, protocol.SwiftUIColorSupportsOpacity, props.SupportsOpacity)
	insertSwiftUILabel(node, props.Label)
	listenStringInput(node, props.OnSelectionChange)
	return finishSwiftUI(node, props.Ref)
}

func (swiftUIAPI) Gauge(props SwiftUIGaugeProps) *native.Node {
	node := makeSwiftUI(protocol.TagSwiftUIGauge, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUINumber(node, protocol.Value, props.Value)
	bindSwiftUINumber(node, protocol.Minimum, props.Min)
	bindSwiftUINumber(node, protocol.Maximum, props.Max)
	bindSwiftUIString(node, protocol.ValueText, props.CurrentValueLabel)
	bindSwiftUIString(node, protocol.SwiftUIGaugeMinimumValueLabel, props.MinimumValueLabel)
	bindSwiftUIString(node, protocol.SwiftUIGaugeMaximumValueLabel, props.MaximumValueLabel)
	setString(node, protocol.SwiftUIGaugeStyle, props.Style)
	insertSwiftUILabel(node, props.Label)
	return finishSwiftUI(node, props.Ref)
}

// QuickGUIHostView is an independently owned QuickGUI renderer inside a SwiftUI hierarchy.
func (swiftUIAPI) QuickGUIHostView(props SwiftUIQuickGUIHostViewProps) *native.Node {
	parent := native.CurrentWindow()
	node := makeSwiftUI(protocol.TagSwiftUIQuickGUIHost, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUINumber(node, protocol.Width, props.Width)
	bindSwiftUINumber(node, protocol.Height, props.Height)
	applyMatchContents(node, props.MatchContents)
	var embedded *native.Window
	disposed := false
	native.Dispatch(func() {
		if disposed || parent.Closed {
			return
		}
		horizontal, vertical := matchContentsAxes(props.MatchContents)
		width, height := 320.0, 200.0
		if next := resolveNumber(props.Width); next != nil {
			width = *next
		}
		if next := resolveNumber(props.Height); next != nil {
			height = *next
		}
		visible := false
		decorated := false
		shadow := false
		embedded = native.NewEmbeddedWindow(parent, native.WindowOptions{
			Title:                "QuickGUI SwiftUI embedded view",
			Width:                width,
			Height:               height,
			Visible:              &visible,
			Decorated:            &decorated,
			Shadow:               &shadow,
			Background:           coalesceBackground(props.Background),
			BackgroundAppearance: "transparent",
			Renderer: CreateRenderer(func() *native.Node {
				surface := native.CreateElement(protocol.TagView)
				if props.Children != nil {
					if child := props.Children(); child != nil {
						native.InsertNode(surface, child, nil)
					}
				}
				return surface
			}),
		}, horizontal, vertical)
		setNumber(node, protocol.SwiftUIEmbeddedWindow, embedded.NativeID)
		parent.Flush()
		embedded.OnClose(func(*native.Window) {
			embedded = nil
			if !disposed {
				setNumber(node, protocol.SwiftUIEmbeddedWindow, nil)
			}
		})
	})
	reactive.OnCleanup(func() {
		disposed = true
		if embedded != nil {
			embedded.Close()
			embedded = nil
		}
	})
	return finishSwiftUI(node, props.Ref)
}

func coalesceBackground(value any) any {
	if value == nil {
		return "transparent"
	}
	return value
}

type swiftUIPopoverState struct {
	props SwiftUIPopoverProps
}

func (state *swiftUIPopoverState) presented() bool {
	flag := resolveBoolean(state.props.IsPresented)
	return flag != nil && *flag
}

func (state *swiftUIPopoverState) request(value bool) {
	if value != state.presented() && state.props.OnIsPresentedChange != nil {
		state.props.OnIsPresentedChange(value)
	}
}

var swiftUIPopoverContext = createPartContext[swiftUIPopoverState]()

var SwiftUIPopover = swiftUIPopoverAPI{}

type swiftUIPopoverAPI struct{}

func (swiftUIPopoverAPI) Root(props SwiftUIPopoverProps) *native.Node {
	state := &swiftUIPopoverState{props: props}
	node := makeSwiftUI(protocol.TagSwiftUIPopover, swiftView(props.TestID, props.Modifiers, props.Ref))
	bindSwiftUIBool(node, protocol.SwiftUIIsPresented, props.IsPresented)
	anchor := props.AttachmentAnchor
	if anchor == "" {
		anchor = "center"
	}
	edge := props.ArrowEdge
	if edge == "" {
		edge = "bottom"
	}
	setString(node, protocol.SwiftUIAttachmentAnchor, anchor)
	setString(node, protocol.SwiftUIArrowEdge, edge)
	setListener(node, protocol.EventPresentation, func(event *native.Event) {
		payload, ok := event.ValueOK()
		state.request(ok && payload == "true")
	})
	return reactive.Provide(swiftUIPopoverContext, state, func() *native.Node {
		if props.Children != nil {
			if child := props.Children(); child != nil {
				native.InsertNode(node, child, nil)
			}
		}
		return finishSwiftUI(node, props.Ref)
	})
}

func (swiftUIPopoverAPI) Trigger(props SwiftUIPopoverTriggerProps) *native.Node {
	state := requireContext(swiftUIPopoverContext, "SwiftUI Popover.Trigger", "Popover.Root")
	node := native.CreateElement(protocol.TagSwiftUIPopoverTrigger)
	rendered := props.Render()
	if rendered != nil {
		native.InsertNode(node, rendered, nil)
	}
	if props.Ref != nil {
		props.Ref(rendered)
	}
	setListener(node, protocol.EventClick, func(event *native.Event) {
		if props.OnPress != nil {
			props.OnPress(event)
		}
		if !event.DefaultPrevented {
			state.request(true)
		}
	})
	return node
}

func (swiftUIPopoverAPI) Content(props SwiftUIPopoverContentProps) *native.Node {
	requireContext(swiftUIPopoverContext, "SwiftUI Popover.Content", "Popover.Root")
	node := native.CreateElement(protocol.TagSwiftUIPopoverContent)
	if props.Children != nil {
		if child := props.Children(); child != nil {
			native.InsertNode(node, child, nil)
		}
	}
	return finishSwiftUI(node, props.Ref)
}

func (swiftUIAPI) PopoverTrigger(props SwiftUIPopoverTriggerProps) *native.Node {
	return SwiftUIPopover.Trigger(props)
}

func (swiftUIAPI) PopoverContent(props SwiftUIPopoverContentProps) *native.Node {
	return SwiftUIPopover.Content(props)
}
