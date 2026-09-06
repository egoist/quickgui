package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// CheckedState is a checkbox's on, off, or mixed state.
type CheckedState any

const CheckedIndeterminate = "indeterminate"

type CheckboxProps struct {
	PartProps
	Value           string
	Parent          bool
	ChildrenChecked func() []bool
	ReadOnly        bool
	Checked         func() CheckedState
	DefaultChecked  CheckedState
	OnCheckedChange func(bool, *native.Event)
}

type CheckboxGroupProps struct {
	PartProps
	Value         func() []string
	DefaultValue  []string
	OnValueChange func([]string, *native.Event)
	AllValues     []string
}

type checkboxGroupState struct {
	scope string
}

var checkboxGroupContext = createPartContext[checkboxGroupState]()

// Checkbox is a controlled, unstyled checkbox whose toggle state belongs to the core.
var Checkbox = checkboxAPI{}

type checkboxAPI struct{}

func (checkboxAPI) Root(props CheckboxProps) *native.Node {
	group := checkboxGroupContext.Use()
	node := createButtonPart(props.PartProps)
	if props.ReadOnly {
		setExplicitBool(node, protocol.ReadOnly, true)
	}
	if group != nil && (props.Value != "" || props.Parent) {
		if props.Parent {
			setPart(node, protocol.PartCheckboxGroupParent, group.scope, "")
		} else {
			setPart(node, protocol.PartCheckboxGroupItem, group.scope, props.Value)
		}
		return finishPart(node, props.PartProps)
	}
	uncontrolled := reactive.NewSignal(defaultCheckedState(props.DefaultChecked))
	checked := func() CheckedState {
		if props.Checked != nil {
			return props.Checked()
		}
		return uncontrolled.Read()
	}
	setPart(node, protocol.PartCheckbox, "", "")
	if props.Parent {
		setExplicitBool(node, protocol.Parent, true)
		if props.ChildrenChecked != nil {
			reactive.CreateRenderEffect(func() {
				setJson(node, protocol.Values, 65536, props.ChildrenChecked())
			})
		}
	}
	reactive.CreateRenderEffect(func() {
		state := checked()
		setExplicitBool(node, protocol.Checked, state == true)
		setExplicitBool(node, protocol.Indeterminate, state == CheckedIndeterminate)
	})
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		next := checked() != true
		if props.Checked == nil {
			uncontrolled.Write(next)
		}
		if props.OnCheckedChange != nil {
			props.OnCheckedChange(next, event)
		}
	}))
	return finishPart(node, props.PartProps)
}

func (checkboxAPI) Indicator(props PartProps) *native.Node {
	group := checkboxGroupContext.Use()
	node := createViewPart(props)
	if group != nil {
		setPart(node, protocol.PartCheckboxGroupIndicator, group.scope, "")
	} else {
		setPart(node, protocol.PartCheckboxIndicator, "", "")
	}
	return finishPart(node, props)
}

func defaultCheckedState(value CheckedState) CheckedState {
	if value == nil {
		return false
	}
	return value
}

// CheckboxGroup keeps checked values in the declared order.
var CheckboxGroup = checkboxGroupAPI{}

type checkboxGroupAPI struct{}

func (checkboxGroupAPI) Root(props CheckboxGroupProps) *native.Node {
	scope := createComponentScope("qg-checkbox-group")
	uncontrolled := reactive.NewSignal(append([]string(nil), props.DefaultValue...))
	values := func() []string {
		if props.Value != nil {
			return props.Value()
		}
		return uncontrolled.Read()
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartCheckboxGroup, scope, "")
	reactive.CreateRenderEffect(func() {
		setJson(node, protocol.Values, 65536, values())
	})
	if props.AllValues != nil {
		setJson(node, protocol.Items, 65536, props.AllValues)
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || details.CheckedValues == nil {
			return
		}
		if props.Value == nil {
			uncontrolled.Write(details.CheckedValues)
		}
		if props.OnValueChange != nil {
			props.OnValueChange(details.CheckedValues, event)
		}
	})
	return reactive.Provide(checkboxGroupContext, &checkboxGroupState{scope: scope}, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

type RadioGroupProps struct {
	PartProps
	Value         func() *string
	DefaultValue  string
	OnValueChange func(string, *native.Event)
	ReadOnly      bool
	Required      bool
}

type RadioProps struct {
	PartProps
	ReadOnly        bool
	Value           string
	Checked         func() bool
	DefaultChecked  bool
	OnCheckedChange func(bool, *native.Event)
}

type radioGroupState struct {
	props RadioGroupProps
	value *reactive.Signal[*string]
}

func (state *radioGroupState) current() *string {
	if state.props.Value != nil {
		return state.props.Value()
	}
	return state.value.Read()
}

func (state *radioGroupState) selectValue(next string, event *native.Event) {
	if state.props.Value == nil {
		value := next
		state.value.Write(&value)
	}
	if state.props.OnValueChange != nil {
		state.props.OnValueChange(next, event)
	}
}

var radioGroupContext = createPartContext[radioGroupState]()

// RadioGroup supplies roving Tab and arrow behavior from the core.
var RadioGroup = radioGroupAPI{}

type radioGroupAPI struct{}

func (radioGroupAPI) Root(props RadioGroupProps) *native.Node {
	var initial *string
	if props.DefaultValue != "" {
		value := props.DefaultValue
		initial = &value
	}
	state := &radioGroupState{props: props, value: reactive.NewSignal(initial)}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartRadioGroup, "", "")
	if props.ReadOnly {
		setExplicitBool(node, protocol.ReadOnly, true)
	}
	if props.Required {
		setExplicitBool(node, protocol.Required, true)
	}
	return reactive.Provide(radioGroupContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

// Radio is a controlled radio button.
var Radio = radioAPI{}

type radioAPI struct{}

func (radioAPI) Root(props RadioProps) *native.Node {
	group := radioGroupContext.Use()
	uncontrolled := reactive.NewSignal(props.DefaultChecked)
	checked := func() bool {
		if group != nil {
			current := group.current()
			return current != nil && *current == props.Value
		}
		if props.Checked != nil {
			return props.Checked()
		}
		return uncontrolled.Read()
	}
	node := createButtonPart(props.PartProps)
	setPart(node, protocol.PartRadio, "", props.Value)
	if props.ReadOnly {
		setExplicitBool(node, protocol.ReadOnly, true)
	}
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Checked, checked())
	})
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		if group != nil {
			if props.Value != "" {
				group.selectValue(props.Value, event)
			}
			return
		}
		if props.Checked == nil {
			uncontrolled.Write(true)
		}
		if props.OnCheckedChange != nil {
			props.OnCheckedChange(true, event)
		}
	}))
	return finishPart(node, props.PartProps)
}

func (radioAPI) Indicator(props PartProps) *native.Node {
	node := createViewPart(props)
	setPart(node, protocol.PartRadioIndicator, "", "")
	return finishPart(node, props)
}

type SwitchProps struct {
	PartProps
	Checked         func() bool
	DefaultChecked  bool
	OnCheckedChange func(bool, *native.Event)
	ReadOnly        bool
}

// Switch is a controlled switch.
var Switch = switchAPI{}

type switchAPI struct{}

func (switchAPI) Root(props SwitchProps) *native.Node {
	uncontrolled := reactive.NewSignal(props.DefaultChecked)
	checked := func() bool {
		if props.Checked != nil {
			return props.Checked()
		}
		return uncontrolled.Read()
	}
	node := createButtonPart(props.PartProps)
	setPart(node, protocol.PartSwitch, "", "")
	if props.ReadOnly {
		setExplicitBool(node, protocol.ReadOnly, true)
	}
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Checked, checked())
	})
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		next := !checked()
		if props.Checked == nil {
			uncontrolled.Write(next)
		}
		if props.OnCheckedChange != nil {
			props.OnCheckedChange(next, event)
		}
	}))
	return finishPart(node, props.PartProps)
}

func (switchAPI) Thumb(props PartProps) *native.Node {
	node := createViewPart(props)
	setPart(node, protocol.PartSwitchThumb, "", "")
	return finishPart(node, props)
}

type ToggleProps struct {
	PartProps
	Pressed         func() bool
	DefaultPressed  bool
	OnPressedChange func(bool, *native.Event)
}

// Toggle is a button that stays pressed, not a checkbox.
var Toggle = toggleAPI{}

type toggleAPI struct{}

func (toggleAPI) Root(props ToggleProps) *native.Node {
	uncontrolled := reactive.NewSignal(props.DefaultPressed)
	pressed := func() bool {
		if props.Pressed != nil {
			return props.Pressed()
		}
		return uncontrolled.Read()
	}
	node := createButtonPart(props.PartProps)
	setPart(node, protocol.PartToggle, "", "")
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Pressed, pressed())
	})
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		next := !pressed()
		if props.Pressed == nil {
			uncontrolled.Write(next)
		}
		if props.OnPressedChange != nil {
			props.OnPressedChange(next, event)
		}
	}))
	return finishPart(node, props.PartProps)
}

func (toggleAPI) Indicator(props PartProps) *native.Node {
	node := createViewPart(props)
	setPart(node, protocol.PartToggleIndicator, "", "")
	return finishPart(node, props)
}

type ToggleGroupProps struct {
	PartProps
	Items         []ComponentItem
	Value         func() []string
	DefaultValue  []string
	OnValueChange func([]string, *native.Event)
	Multiple      bool
}

type ToggleGroupItemProps struct {
	PartProps
	Value string
}

var toggleGroupContext = createPartContext[string]()

// ToggleGroup is a single- or multiple-selection toggle set.
var ToggleGroup = toggleGroupAPI{}

type toggleGroupAPI struct{}

func (toggleGroupAPI) Root(props ToggleGroupProps) *native.Node {
	scope := createComponentScope("qg-toggle-group")
	uncontrolled := reactive.NewSignal(append([]string(nil), props.DefaultValue...))
	pressed := func() []string {
		if props.Value != nil {
			return props.Value()
		}
		return uncontrolled.Read()
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartToggleGroup, scope, "")
	setJson(node, protocol.Items, 65536, props.Items)
	if props.Multiple {
		setExplicitBool(node, protocol.Multiple, true)
	}
	reactive.CreateRenderEffect(func() {
		setJson(node, protocol.Values, 65536, pressed())
	})
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || details.Pressed == nil {
			return
		}
		if props.Value == nil {
			uncontrolled.Write(details.Pressed)
		}
		if props.OnValueChange != nil {
			props.OnValueChange(details.Pressed, event)
		}
	})
	return reactive.Provide(toggleGroupContext, &scope, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (toggleGroupAPI) Item(props ToggleGroupItemProps) *native.Node {
	node := createButtonPart(props.PartProps)
	scope := ""
	if current := toggleGroupContext.Use(); current != nil {
		scope = *current
	}
	setPart(node, protocol.PartToggleGroupItem, scope, props.Value)
	return finishPart(node, props.PartProps)
}
