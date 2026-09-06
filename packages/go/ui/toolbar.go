package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

type ToolbarRootProps struct {
	PartProps
	Items          []ComponentItem
	Active         func() *string
	DefaultActive  string
	Orientation    string
	LoopFocus      *bool
	OnActiveChange func(string, *native.Event)
}

type ToolbarItemProps struct {
	PartProps
	Value string
}

type ToolbarInputProps struct {
	InputPartProps
	PartValue string
	Element   string
}

var toolbarContext = createPartContext[string]()

func createToolbarItem(part string, props ToolbarItemProps) *native.Node {
	node := createButtonPart(props.PartProps)
	scope := ""
	if current := toolbarContext.Use(); current != nil {
		scope = *current
	}
	setPart(node, part, scope, props.Value)
	return finishPart(node, props.PartProps)
}

func toolbarElementTag(element string) uint8 {
	switch element {
	case "button":
		return protocol.TagButton
	case "view":
		return protocol.TagView
	case "text":
		return protocol.TagText
	default:
		return protocol.TagInput
	}
}

// Toolbar is a toolbar with a single roving Tab stop.
var Toolbar = toolbarAPI{}

type toolbarAPI struct{}

func (toolbarAPI) Root(props ToolbarRootProps) *native.Node {
	var initial *string
	if props.DefaultActive != "" {
		value := props.DefaultActive
		initial = &value
	}
	uncontrolled := reactive.NewSignal(initial)
	active := func() *string {
		if props.Active != nil {
			return props.Active()
		}
		return uncontrolled.Read()
	}
	scope := createComponentScope("qg-toolbar")
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartToolbar, scope, "")
	if props.Items != nil {
		setJson(node, protocol.Items, 65536, props.Items)
	}
	reactive.CreateRenderEffect(func() {
		current := active()
		if current == nil {
			setComponentValue(node, protocol.ActiveValue, "")
			return
		}
		setComponentValue(node, protocol.ActiveValue, *current)
	})
	if props.Orientation != "" {
		setString(node, protocol.Orientation, props.Orientation)
	}
	if props.LoopFocus != nil {
		setExplicitBool(node, protocol.LoopFocus, *props.LoopFocus)
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || !details.Active.present() || details.Active.Null {
			return
		}
		next := details.Active.Value
		if props.Active == nil {
			uncontrolled.Write(&next)
		}
		if props.OnActiveChange != nil {
			props.OnActiveChange(next, event)
		}
	})
	return reactive.Provide(toolbarContext, &scope, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (toolbarAPI) Item(props ToolbarItemProps) *native.Node {
	return createToolbarItem(protocol.PartToolbarItem, props)
}

func (toolbarAPI) Button(props ToolbarItemProps) *native.Node {
	return createToolbarItem(protocol.PartToolbarButton, props)
}

func (toolbarAPI) Link(props ToolbarItemProps) *native.Node {
	return createToolbarItem(protocol.PartToolbarLink, props)
}

func (toolbarAPI) Input(props ToolbarInputProps) *native.Node {
	node := createPart(toolbarElementTag(props.Element), props.PartProps)
	if props.Element == "" || props.Element == "input" {
		applyInputPart(node, props.InputPartProps)
	}
	scope := ""
	if current := toolbarContext.Use(); current != nil {
		scope = *current
	}
	setPart(node, protocol.PartToolbarInput, scope, props.PartValue)
	return finishPart(node, props.PartProps)
}

func (toolbarAPI) Group(props PartProps) *native.Node {
	node := createViewPart(props)
	scope := ""
	if current := toolbarContext.Use(); current != nil {
		scope = *current
	}
	setPart(node, protocol.PartToolbarGroup, scope, "")
	return finishPart(node, props)
}

func (toolbarAPI) Separator(props PartProps) *native.Node {
	node := createViewPart(props)
	scope := ""
	if current := toolbarContext.Use(); current != nil {
		scope = *current
	}
	setPart(node, protocol.PartToolbarSeparator, scope, "")
	return finishPart(node, props)
}

type SeparatorProps struct {
	PartProps
	Orientation string
}

// Separator is an unstyled semantic separator.
var Separator = separatorAPI{}

type separatorAPI struct{}

func (separatorAPI) Root(props SeparatorProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartSeparator, "", "")
	if props.Orientation != "" {
		setString(node, protocol.Orientation, props.Orientation)
	}
	return finishPart(node, props.PartProps)
}
