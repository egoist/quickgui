package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// TabsState is everything the core decided about one tab set.
type TabsState struct {
	ActivationDirection string
	Indicator           *TabsIndicatorGeometry
}

func settledTabs() TabsState {
	return TabsState{ActivationDirection: "none", Indicator: nil}
}

type TabsRootProps struct {
	PartProps
	Value             func() *string
	DefaultValue      string
	OnValueChange     func(string, *native.Event)
	Orientation       string
	Activation        string
	Loop              *bool
	KeepMounted       bool
	OnTabsStateChange func(TabsState, *native.Event)
}

type TabsTabProps struct {
	PartProps
	Value string
	Index *int
}

type TabsIndicatorProps struct {
	PartProps
	Value     string
	Placement string
}

type TabsPanelProps struct {
	PartProps
	Value string
}

type tabsRootState struct {
	scope string
	props TabsRootProps
	value *reactive.Signal[*string]
	state *reactive.Signal[TabsState]
}

func (state *tabsRootState) current() *string {
	if state.props.Value != nil {
		return state.props.Value()
	}
	return state.value.Read()
}

func (state *tabsRootState) currentValue() string {
	if current := state.current(); current != nil {
		return *current
	}
	return ""
}

func (state *tabsRootState) live() TabsState {
	return state.state.Read()
}

func (state *tabsRootState) selectValue(next string, event *native.Event) {
	if state.props.Value == nil {
		value := next
		state.value.Write(&value)
	}
	if state.props.OnValueChange != nil {
		state.props.OnValueChange(next, event)
	}
}

func (state *tabsRootState) applyTo(node *native.Node, part, partValue string) {
	setPart(node, part, state.scope, partValue)
	reactive.CreateRenderEffect(func() {
		setComponentValue(node, protocol.ActiveValue, state.currentValue())
	})
	orientation := state.props.Orientation
	if orientation == "" {
		orientation = "horizontal"
	}
	setString(node, protocol.Orientation, orientation)
	setExplicitBool(node, protocol.ActivateOnFocus, state.props.Activation == "automatic")
	loop := true
	if state.props.Loop != nil {
		loop = *state.props.Loop
	}
	setExplicitBool(node, protocol.LoopFocus, loop)
	setExplicitBool(node, protocol.KeepMounted, state.props.KeepMounted)
}

var tabsContext = createPartContext[tabsRootState]()
var tabValueContext = createPartContext[string]()

// UseTabsState reads the live tab-set state inside a Tabs.Root subtree.
func UseTabsState() func() TabsState {
	state := tabsContext.Use()
	if state == nil {
		return settledTabs
	}
	return state.live
}

// Tabs is a controlled tab set.
var Tabs = tabsAPI{}

type tabsAPI struct{}

func (tabsAPI) Root(props TabsRootProps) *native.Node {
	var initial *string
	if props.DefaultValue != "" {
		value := props.DefaultValue
		initial = &value
	}
	state := &tabsRootState{
		scope: createComponentScope("qg-tabs"),
		props: props,
		value: reactive.NewSignal(initial),
		state: reactive.NewSignal(settledTabs()),
	}
	node := createViewPart(props.PartProps)
	state.applyTo(node, protocol.PartTabs, "")
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || !details.ActivationDirection.present() || details.ActivationDirection.Null {
			return
		}
		next := TabsState{
			ActivationDirection: details.ActivationDirection.Value,
			Indicator:           details.Indicator.ptr(),
		}
		state.state.Write(next)
		if props.OnTabsStateChange != nil {
			props.OnTabsStateChange(next, event)
		}
	})
	return reactive.Provide(tabsContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (tabsAPI) List(props PartProps) *native.Node {
	state := requireContext(tabsContext, "Tabs.List", "Tabs.Root")
	node := createViewPart(props)
	state.applyTo(node, protocol.PartTabsList, "")
	return finishPart(node, props)
}

func (tabsAPI) Tab(props TabsTabProps) *native.Node {
	state := requireContext(tabsContext, "Tabs.Tab", "Tabs.Root")
	node := createButtonPart(props.PartProps)
	state.applyTo(node, protocol.PartTab, props.Value)
	if props.Index != nil {
		setNumber(node, protocol.ItemIndex, *props.Index)
	}
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		state.selectValue(props.Value, event)
	}))
	return reactive.Provide(tabValueContext, &props.Value, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (tabsAPI) Indicator(props TabsIndicatorProps) *native.Node {
	state := requireContext(tabsContext, "Tabs.Indicator", "Tabs.Root")
	inherited := tabValueContext.Use()
	node := createViewPart(props.PartProps)
	explicit := props.Value
	if explicit == "" && inherited != nil {
		explicit = *inherited
	}
	state.applyTo(node, protocol.PartTabIndicator, explicit)
	if explicit == "" {
		reactive.CreateRenderEffect(func() {
			setComponentValue(node, protocol.PartValue, state.currentValue())
		})
	}
	if props.Placement != "" {
		setString(node, protocol.AnchorPlacement, props.Placement)
	}
	return finishPart(node, props.PartProps)
}

func (tabsAPI) Panel(props TabsPanelProps) *native.Node {
	state := requireContext(tabsContext, "Tabs.Panel", "Tabs.Root")
	node := createViewPart(props.PartProps)
	state.applyTo(node, protocol.PartTabPanel, props.Value)
	return finishPart(node, props.PartProps)
}
