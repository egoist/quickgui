package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

type CollapsibleRootProps struct {
	PartProps
	Open         func() bool
	DefaultOpen  bool
	OnOpenChange func(bool, *native.Event)
	KeepMounted  bool
}

type collapsibleState struct {
	scope string
	props CollapsibleRootProps
	open  *reactive.Signal[bool]
}

func (state *collapsibleState) isOpen() bool {
	if state.props.Open != nil {
		return state.props.Open()
	}
	return state.open.Read()
}

func (state *collapsibleState) toggle(event *native.Event) {
	next := !state.isOpen()
	if state.props.Open == nil {
		state.open.Write(next)
	}
	if state.props.OnOpenChange != nil {
		state.props.OnOpenChange(next, event)
	}
}

func (state *collapsibleState) applyTo(node *native.Node, part string) {
	setPart(node, part, state.scope, "")
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Open, state.isOpen())
	})
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Disabled, resolveBoolean(state.props.Disabled) != nil && *resolveBoolean(state.props.Disabled))
	})
	setExplicitBool(node, protocol.KeepMounted, state.props.KeepMounted)
}

var collapsibleContext = createPartContext[collapsibleState]()

// Collapsible is a controlled disclosure.
var Collapsible = collapsibleAPI{}

type collapsibleAPI struct{}

func (collapsibleAPI) Root(props CollapsibleRootProps) *native.Node {
	state := &collapsibleState{
		scope: createComponentScope("qg-collapsible"),
		props: props,
		open:  reactive.NewSignal(props.DefaultOpen),
	}
	node := createViewPart(props.PartProps)
	state.applyTo(node, protocol.PartCollapsible)
	return reactive.Provide(collapsibleContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (collapsibleAPI) Trigger(props PartProps) *native.Node {
	state := requireContext(collapsibleContext, "Collapsible.Trigger", "Collapsible.Root")
	node := createButtonPart(props)
	state.applyTo(node, protocol.PartCollapsibleTrigger)
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		state.toggle(event)
	}))
	return finishPart(node, props)
}

func (collapsibleAPI) Panel(props PartProps) *native.Node {
	state := requireContext(collapsibleContext, "Collapsible.Panel", "Collapsible.Root")
	node := createViewPart(props)
	state.applyTo(node, protocol.PartCollapsiblePanel)
	return finishPart(node, props)
}

type AccordionRootProps struct {
	PartProps
	Value         func() []string
	DefaultValue  []string
	OnValueChange func([]string, *native.Event)
	Multiple      bool
	KeepMounted   bool
	HeadingLevel  any
}

type AccordionItemProps struct {
	PartProps
	Value string
	Index *int
}

type accordionState struct {
	scope string
	props AccordionRootProps
	open  *reactive.Signal[[]string]
}

func (state *accordionState) openValues() []string {
	if state.props.Value != nil {
		return state.props.Value()
	}
	return state.open.Read()
}

func (state *accordionState) isOpen(value string) bool {
	for _, candidate := range state.openValues() {
		if candidate == value {
			return true
		}
	}
	return false
}

func (state *accordionState) disabled() bool {
	flag := resolveBoolean(state.props.Disabled)
	return flag != nil && *flag
}

func (state *accordionState) toggle(value string, event *native.Event) {
	current := state.openValues()
	var next []string
	if state.isOpen(value) {
		for _, candidate := range current {
			if candidate != value {
				next = append(next, candidate)
			}
		}
	} else if state.props.Multiple {
		next = append(append([]string{}, current...), value)
	} else {
		next = []string{value}
	}
	if state.props.Value == nil {
		state.open.Write(next)
	}
	if state.props.OnValueChange != nil {
		state.props.OnValueChange(next, event)
	}
}

type accordionItemState struct {
	accordion *accordionState
	props     AccordionItemProps
}

func (item *accordionItemState) isOpen() bool {
	return item.accordion.isOpen(item.props.Value)
}

func (item *accordionItemState) disabled() bool {
	flag := resolveBoolean(item.props.Disabled)
	return (flag != nil && *flag) || item.accordion.disabled()
}

func (item *accordionItemState) applyTo(node *native.Node, part string) {
	setPart(node, part, item.accordion.scope, item.props.Value)
	index := 0
	if item.props.Index != nil {
		index = *item.props.Index
	}
	setNumber(node, protocol.ItemIndex, index)
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Open, item.isOpen())
	})
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Disabled, item.disabled())
	})
	setExplicitBool(node, protocol.KeepMounted, item.accordion.props.KeepMounted)
	heading := item.accordion.props.HeadingLevel
	if heading == nil {
		heading = 3
	}
	setNumber(node, protocol.HeadingLevel, heading)
}

var accordionContext = createPartContext[accordionState]()
var accordionItemContext = createPartContext[accordionItemState]()

// Accordion is a controlled accordion supporting single or multiple open items.
var Accordion = accordionAPI{}

type accordionAPI struct{}

func (accordionAPI) Root(props AccordionRootProps) *native.Node {
	state := &accordionState{
		scope: createComponentScope("qg-accordion"),
		props: props,
		open:  reactive.NewSignal(append([]string(nil), props.DefaultValue...)),
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartAccordion, state.scope, "")
	return reactive.Provide(accordionContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (accordionAPI) Item(props AccordionItemProps) *native.Node {
	accordion := requireContext(accordionContext, "Accordion.Item", "Accordion.Root")
	item := &accordionItemState{accordion: accordion, props: props}
	node := createViewPart(props.PartProps)
	item.applyTo(node, protocol.PartAccordionItem)
	return reactive.Provide(accordionItemContext, item, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (accordionAPI) Header(props PartProps) *native.Node {
	item := requireContext(accordionItemContext, "Accordion.Header", "Accordion.Item")
	node := createViewPart(props)
	item.applyTo(node, protocol.PartAccordionHeader)
	return finishPart(node, props)
}

func (accordionAPI) Trigger(props PartProps) *native.Node {
	item := requireContext(accordionItemContext, "Accordion.Trigger", "Accordion.Item")
	node := createButtonPart(props)
	item.applyTo(node, protocol.PartAccordionTrigger)
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		item.accordion.toggle(item.props.Value, event)
	}))
	return finishPart(node, props)
}

func (accordionAPI) Panel(props PartProps) *native.Node {
	item := requireContext(accordionItemContext, "Accordion.Panel", "Accordion.Item")
	node := createViewPart(props)
	item.applyTo(node, protocol.PartAccordionPanel)
	return finishPart(node, props)
}
