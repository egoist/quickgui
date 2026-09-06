package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

type FieldRootProps struct {
	PartProps
	Invalid                func() bool
	Required               bool
	Touched                func() bool
	Dirty                  func() bool
	Filled                 func() bool
	ValidationMessage      func() string
	ValidationMode         string
	ValidationDebounceTime any
	OnValidationChange     func(FieldValidationDetails, *native.Event)
}

type FieldValidityProps struct {
	PartProps
	Visible *bool
}

type FieldLabelProps struct {
	PartProps
	Passive bool
}

type FieldControlElement string

type FieldControlProps struct {
	InputPartProps
	Element FieldControlElement
}

type fieldsetState struct {
	props PartProps
}

func (state *fieldsetState) disabled() bool {
	flag := resolveBoolean(state.props.Disabled)
	return flag != nil && *flag
}

var fieldsetContext = createPartContext[fieldsetState]()

type fieldState struct {
	scope    string
	props    FieldRootProps
	fieldset *fieldsetState
}

func (state *fieldState) disabled() bool {
	flag := resolveBoolean(state.props.Disabled)
	if flag != nil && *flag {
		return true
	}
	return state.fieldset != nil && state.fieldset.disabled()
}

func (state *fieldState) applyTo(node *native.Node, part string) {
	setPart(node, part, state.scope, "")
	props := state.props
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Disabled, state.disabled())
		setExplicitBool(node, protocol.Invalid, readFlag(props.Invalid))
		setExplicitBool(node, protocol.Required, props.Required)
		setExplicitBool(node, protocol.Touched, readFlag(props.Touched))
		setExplicitBool(node, protocol.Dirty, readFlag(props.Dirty))
		setExplicitBool(node, protocol.Filled, readFlag(props.Filled))
		if props.ValidationMessage == nil {
			setString(node, protocol.ValidationMessage, "")
		} else {
			setString(node, protocol.ValidationMessage, props.ValidationMessage())
		}
	})
}

func readFlag(value func() bool) bool {
	return value != nil && value()
}

var fieldContext = createPartContext[fieldState]()

func controlTag(element FieldControlElement) uint8 {
	switch element {
	case "button":
		return protocol.TagButton
	case "view", "text":
		return protocol.TagView
	default:
		return protocol.TagInput
	}
}

// Field is labelling and validation composition for one form control.
var Field = fieldAPI{}

type fieldAPI struct{}

func (fieldAPI) Root(props FieldRootProps) *native.Node {
	state := &fieldState{scope: createComponentScope("qg-field"), props: props, fieldset: fieldsetContext.Use()}
	node := createViewPart(props.PartProps)
	state.applyTo(node, protocol.PartField)
	if props.ValidationMode != "" {
		setString(node, protocol.ValidationMode, props.ValidationMode)
	}
	if props.ValidationDebounceTime != nil {
		setMilliseconds(node, protocol.ValidationDebounceTime, props.ValidationDebounceTime)
	}
	if props.OnValidationChange != nil {
		setListener(node, protocol.EventComponentChange, func(event *native.Event) {
			details := ComponentChangeFromEvent(event)
			if details == nil || details.Validation == nil || details.ValidationDelay == nil {
				return
			}
			props.OnValidationChange(FieldValidationDetails{
				Triggers: *details.Validation,
				Delay:    *details.ValidationDelay,
			}, event)
		})
	}
	return reactive.Provide(fieldContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (fieldAPI) Label(props FieldLabelProps) *native.Node {
	state := requireContext(fieldContext, "Field.Label", "Field.Root")
	node := createViewPart(props.PartProps)
	part := protocol.PartFieldLabel
	if props.Passive {
		part = protocol.PartFieldPassiveLabel
	}
	state.applyTo(node, part)
	return finishPart(node, props.PartProps)
}

func (fieldAPI) Control(props FieldControlProps) *native.Node {
	state := requireContext(fieldContext, "Field.Control", "Field.Root")
	node := createPart(controlTag(props.Element), props.PartProps)
	if props.Element == "textarea" {
		input := props.InputPartProps
		input.Multiline = true
		applyInputPart(node, input)
	} else if props.Element == "" || props.Element == "input" {
		applyInputPart(node, props.InputPartProps)
	}
	state.applyTo(node, protocol.PartFieldControl)
	return finishPart(node, props.PartProps)
}

func (fieldAPI) Description(props PartProps) *native.Node {
	state := requireContext(fieldContext, "Field.Description", "Field.Root")
	node := createViewPart(props)
	state.applyTo(node, protocol.PartFieldDescription)
	return finishPart(node, props)
}

func (fieldAPI) Item(props PartProps) *native.Node {
	state := requireContext(fieldContext, "Field.Item", "Field.Root")
	node := createViewPart(props)
	state.applyTo(node, protocol.PartFieldItem)
	return finishPart(node, props)
}

func (fieldAPI) Validity(props FieldValidityProps) *native.Node {
	state := requireContext(fieldContext, "Field.Validity", "Field.Root")
	node := createViewPart(props.PartProps)
	state.applyTo(node, protocol.PartFieldValidity)
	visible := true
	if props.Visible != nil {
		visible = *props.Visible
	}
	setExplicitBool(node, protocol.Open, visible)
	return finishPart(node, props.PartProps)
}

func (fieldAPI) Error(props PartProps) *native.Node {
	state := requireContext(fieldContext, "Field.Error", "Field.Root")
	node := createViewPart(props)
	state.applyTo(node, protocol.PartFieldError)
	return finishPart(node, props)
}

// Fieldset is a semantic field group.
var Fieldset = fieldsetAPI{}

type fieldsetAPI struct{}

func (fieldsetAPI) Root(props PartProps) *native.Node {
	state := &fieldsetState{props: props}
	node := createViewPart(props)
	setPart(node, protocol.PartFieldset, createComponentScope("qg-fieldset"), "")
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Disabled, state.disabled())
	})
	return reactive.Provide(fieldsetContext, state, func() *native.Node {
		return finishPart(node, props)
	})
}

func (fieldsetAPI) Legend(props PartProps) *native.Node {
	node := createViewPart(props)
	setPart(node, protocol.PartFieldsetLegend, "", "")
	return finishPart(node, props)
}

func (fieldsetAPI) Description(props PartProps) *native.Node {
	node := createViewPart(props)
	setPart(node, protocol.PartFieldsetDescription, "", "")
	return finishPart(node, props)
}

func (fieldsetAPI) Control(props FieldControlProps) *native.Node {
	fieldset := fieldsetContext.Use()
	node := createPart(controlTag(props.Element), props.PartProps)
	if props.Element == "textarea" {
		input := props.InputPartProps
		input.Multiline = true
		applyInputPart(node, input)
	} else if props.Element == "" || props.Element == "input" {
		applyInputPart(node, props.InputPartProps)
	}
	setPart(node, protocol.PartFieldsetControl, "", "")
	reactive.CreateRenderEffect(func() {
		flag := resolveBoolean(props.Disabled)
		disabled := flag != nil && *flag || (fieldset != nil && fieldset.disabled())
		setExplicitBool(node, protocol.Disabled, disabled)
	})
	return finishPart(node, props.PartProps)
}
