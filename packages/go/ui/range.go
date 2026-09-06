package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// SliderState is everything the core decided about one slider since the last frame.
type SliderState struct {
	Values       []float64
	Dragging     bool
	DisplayValue *string
}

func settledSlider() SliderState {
	return SliderState{Values: nil, Dragging: false, DisplayValue: nil}
}

type SliderRootProps struct {
	PartProps
	Value                 func() []float64
	DefaultValue          []float64
	OnValueChange         func([]float64, *native.Event)
	OnValueCommitted      func([]float64, *native.Event)
	Min                   any
	Max                   any
	Step                  any
	LargeStep             any
	MinStepsBetweenValues any
	ThumbAlignment        string
	Format                string
	Orientation           string
}

type SliderThumbProps struct {
	PartProps
	Index *int
}

type sliderRootState struct {
	scope string
	state *reactive.Signal[SliderState]
}

func (state *sliderRootState) live() SliderState {
	return state.state.Read()
}

var sliderContext = createPartContext[sliderRootState]()

// UseSliderState reads the live slider state inside a Slider.Root subtree.
func UseSliderState() func() SliderState {
	context := sliderContext.Use()
	if context == nil {
		return settledSlider
	}
	return context.live
}

func sliderScope() string {
	context := sliderContext.Use()
	if context == nil {
		return ""
	}
	return context.scope
}

func createSliderPart(part string, props PartProps) *native.Node {
	node := createViewPart(props)
	setPart(node, part, sliderScope(), "")
	return finishPart(node, props)
}

// Slider is a controlled slider. Value carries one entry per thumb.
var Slider = sliderAPI{}

type sliderAPI struct{}

func (sliderAPI) Root(props SliderRootProps) *native.Node {
	state := &sliderRootState{
		scope: createComponentScope("qg-slider"),
		state: reactive.NewSignal(settledSlider()),
	}
	initial := props.DefaultValue
	if initial == nil {
		initial = []float64{0}
	}
	uncontrolled := reactive.NewSignal(append([]float64(nil), initial...))
	values := func() []float64 {
		if props.Value != nil {
			return props.Value()
		}
		return uncontrolled.Read()
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartSlider, state.scope, "")
	reactive.CreateRenderEffect(func() {
		setJson(node, protocol.Values, 65536, values())
	})
	if props.Min != nil {
		setNumber(node, protocol.Minimum, props.Min)
	}
	if props.Max != nil {
		setNumber(node, protocol.Maximum, props.Max)
	}
	if props.Step != nil {
		setNumber(node, protocol.Step, props.Step)
	}
	if props.LargeStep != nil {
		setNumber(node, protocol.LargeStep, props.LargeStep)
	}
	if props.MinStepsBetweenValues != nil {
		setNumber(node, protocol.MinStepsBetweenValues, props.MinStepsBetweenValues)
	}
	if props.ThumbAlignment != "" {
		setString(node, protocol.ThumbAlignment, props.ThumbAlignment)
	}
	if props.Format != "" {
		setString(node, protocol.Format, props.Format)
	}
	if props.Orientation != "" {
		setString(node, protocol.Orientation, props.Orientation)
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || details.Values == nil {
			return
		}
		next := SliderState{
			Values:       details.Values,
			Dragging:     details.Dragging != nil && *details.Dragging,
			DisplayValue: details.DisplayValue.ptr(),
		}
		state.state.Write(next)
		if props.Value == nil {
			uncontrolled.Write(details.Values)
		}
		if props.OnValueChange != nil {
			props.OnValueChange(details.Values, event)
		}
		if details.Committed != nil && *details.Committed && props.OnValueCommitted != nil {
			props.OnValueCommitted(details.Values, event)
		}
	})
	return reactive.Provide(sliderContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (sliderAPI) Label(props PartProps) *native.Node {
	return createSliderPart(protocol.PartSliderLabel, props)
}

func (sliderAPI) Value(props PartProps) *native.Node {
	return createSliderPart(protocol.PartSliderValue, props)
}

func (sliderAPI) Control(props PartProps) *native.Node {
	return createSliderPart(protocol.PartSliderControl, props)
}

func (sliderAPI) Track(props PartProps) *native.Node {
	return createSliderPart(protocol.PartSliderTrack, props)
}

func (sliderAPI) Range(props PartProps) *native.Node {
	return createSliderPart(protocol.PartSliderRange, props)
}

func (sliderAPI) Indicator(props PartProps) *native.Node {
	return createSliderPart(protocol.PartSliderIndicator, props)
}

func (sliderAPI) Thumb(props SliderThumbProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartSliderThumb, sliderScope(), "")
	if props.Index != nil {
		setNumber(node, protocol.ItemIndex, *props.Index)
	}
	return finishPart(node, props.PartProps)
}

type SplitterPaneDeclaration struct {
	Min         *float64 `json:"min,omitempty"`
	Collapsible bool     `json:"collapsible,omitempty"`
}

type SplitterRootProps struct {
	PartProps
	Value         func() []float64
	DefaultValue  []float64
	Panes         []SplitterPaneDeclaration
	Step          any
	Orientation   string
	OnSizesChange func([]float64, *native.Event)
}

type SplitterPaneProps struct {
	PartProps
	Index *int
}

var splitterContext = createPartContext[string]()

func createSplitterPart(part string, props SplitterPaneProps) *native.Node {
	node := createViewPart(props.PartProps)
	scope := ""
	if current := splitterContext.Use(); current != nil {
		scope = *current
	}
	setPart(node, part, scope, "")
	if props.Index != nil {
		setNumber(node, protocol.ItemIndex, *props.Index)
	}
	return finishPart(node, props.PartProps)
}

// Splitter is a controlled pane splitter.
var Splitter = splitterAPI{}

type splitterAPI struct{}

func (splitterAPI) Root(props SplitterRootProps) *native.Node {
	uncontrolled := reactive.NewSignal(append([]float64(nil), props.DefaultValue...))
	sizes := func() []float64 {
		if props.Value != nil {
			return props.Value()
		}
		return uncontrolled.Read()
	}
	scope := createComponentScope("qg-splitter")
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartSplitter, scope, "")
	reactive.CreateRenderEffect(func() {
		setJson(node, protocol.Values, 65536, sizes())
	})
	if props.Panes != nil {
		setJson(node, protocol.Items, 65536, props.Panes)
	}
	if props.Step != nil {
		setNumber(node, protocol.Step, props.Step)
	}
	if props.Orientation != "" {
		setString(node, protocol.Orientation, props.Orientation)
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || details.Sizes == nil {
			return
		}
		if props.Value == nil {
			uncontrolled.Write(details.Sizes)
		}
		if props.OnSizesChange != nil {
			props.OnSizesChange(details.Sizes, event)
		}
	})
	return reactive.Provide(splitterContext, &scope, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (splitterAPI) Pane(props SplitterPaneProps) *native.Node {
	return createSplitterPart(protocol.PartSplitterPane, props)
}

func (splitterAPI) Handle(props SplitterPaneProps) *native.Node {
	return createSplitterPart(protocol.PartSplitterHandle, props)
}
