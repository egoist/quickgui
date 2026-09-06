package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// NumberFieldState is everything the core decided about one number field.
type NumberFieldState struct {
	Scrubbing bool
	ReadOnly  bool
	Required  bool
}

func settledNumberField() NumberFieldState {
	return NumberFieldState{}
}

type NumberFieldRootProps struct {
	PartProps
	Value            func() *float64
	DefaultValue     *float64
	Min              any
	Max              any
	Step             any
	SmallStep        any
	LargeStep        any
	Precision        any
	SnapOnStep       *bool
	AllowWheelScrub  *bool
	ReadOnly         bool
	Required         bool
	ScrubDirection   string
	ScrubSensitivity any
	OnValueChange    func(*float64, bool, *native.Event)
	OnValueCommitted func(*float64, *native.Event)
}

type NumberFieldInputProps struct {
	InputPartProps
	OnCommit func(CommitDetails, *native.Event)
}

type numberFieldRootState struct {
	scope string
	state *reactive.Signal[NumberFieldState]
}

func (state *numberFieldRootState) live() NumberFieldState {
	return state.state.Read()
}

var numberFieldContext = createPartContext[numberFieldRootState]()

// UseNumberFieldState reads the live number-field state inside a NumberField.Root subtree.
func UseNumberFieldState() func() NumberFieldState {
	context := numberFieldContext.Use()
	if context == nil {
		return settledNumberField
	}
	return context.live
}

func numberFieldScope() string {
	context := numberFieldContext.Use()
	if context == nil {
		return ""
	}
	return context.scope
}

func createNumberFieldPart(tag uint8, part string, props PartProps) *native.Node {
	node := createPart(tag, props)
	setPart(node, part, numberFieldScope(), "")
	return finishPart(node, props)
}

// NumberField is a controlled number field. The core parses, clamps, formats, and steps.
var NumberField = numberFieldAPI{}

type numberFieldAPI struct{}

func (numberFieldAPI) Root(props NumberFieldRootProps) *native.Node {
	state := &numberFieldRootState{
		scope: createComponentScope("qg-number-field"),
		state: reactive.NewSignal(settledNumberField()),
	}
	uncontrolled := reactive.NewSignal(props.DefaultValue)
	value := func() *float64 {
		if props.Value != nil {
			return props.Value()
		}
		return uncontrolled.Read()
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartNumberField, state.scope, "")
	reactive.CreateRenderEffect(func() {
		current := value()
		if current == nil {
			setJson(node, protocol.Values, 65536, []float64{})
			return
		}
		setJson(node, protocol.Values, 65536, []float64{*current})
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
	if props.SmallStep != nil {
		setNumber(node, protocol.SmallStep, props.SmallStep)
	}
	if props.LargeStep != nil {
		setNumber(node, protocol.LargeStep, props.LargeStep)
	}
	if props.Precision != nil {
		setNumber(node, protocol.Precision, props.Precision)
	}
	setExplicitBool(node, protocol.SnapOnStep, props.SnapOnStep)
	setExplicitBool(node, protocol.AllowWheelScrub, props.AllowWheelScrub)
	if props.ReadOnly {
		setExplicitBool(node, protocol.ReadOnly, true)
	}
	if props.Required {
		setExplicitBool(node, protocol.Required, true)
	}
	setString(node, protocol.Orientation, props.ScrubDirection)
	if props.ScrubSensitivity != nil {
		setNumber(node, protocol.Pitch, props.ScrubSensitivity)
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || details.Text == "" {
			return
		}
		next := details.NumberValue.ptr()
		state.state.Write(NumberFieldState{
			Scrubbing: details.Scrubbing != nil && *details.Scrubbing,
			ReadOnly:  details.ReadOnly != nil && *details.ReadOnly,
			Required:  details.Required != nil && *details.Required,
		})
		if props.Value == nil {
			uncontrolled.Write(next)
		}
		if props.OnValueChange != nil {
			props.OnValueChange(next, details.Valid == nil || *details.Valid, event)
		}
		if details.Committed != nil && *details.Committed && props.OnValueCommitted != nil {
			props.OnValueCommitted(next, event)
		}
	})
	return reactive.Provide(numberFieldContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (numberFieldAPI) Group(props PartProps) *native.Node {
	return createNumberFieldPart(protocol.TagView, protocol.PartNumberFieldGroup, props)
}

func (numberFieldAPI) Input(props NumberFieldInputProps) *native.Node {
	node := createPart(protocol.TagInput, props.PartProps)
	applyInputPart(node, props.InputPartProps)
	setPart(node, protocol.PartNumberFieldInput, numberFieldScope(), "")
	if props.OnCommit != nil {
		setListener(node, protocol.EventCommit, func(event *native.Event) {
			details := CommitFromEvent(event)
			if details != nil {
				props.OnCommit(*details, event)
			}
		})
	}
	return finishPart(node, props.PartProps)
}

func (numberFieldAPI) Increment(props PartProps) *native.Node {
	node := createButtonPart(props)
	setPart(node, protocol.PartNumberFieldIncrement, numberFieldScope(), "")
	return finishPart(node, props)
}

func (numberFieldAPI) Decrement(props PartProps) *native.Node {
	node := createButtonPart(props)
	setPart(node, protocol.PartNumberFieldDecrement, numberFieldScope(), "")
	return finishPart(node, props)
}

func (numberFieldAPI) ScrubArea(props PartProps) *native.Node {
	return createNumberFieldPart(protocol.TagView, protocol.PartNumberFieldScrubArea, props)
}

func (numberFieldAPI) ScrubAreaCursor(props PartProps) *native.Node {
	return createNumberFieldPart(protocol.TagView, protocol.PartNumberFieldScrubAreaCursor, props)
}
