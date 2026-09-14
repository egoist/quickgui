package ui

import (
	"fmt"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
)

type primitiveListener struct {
	kind    int
	handler func(*native.Event)
}

func primitiveListeners(props Props) []primitiveListener {
	return []primitiveListener{
		{protocol.EventClick, props.OnClick},
		{protocol.EventMouseEnter, props.OnMouseEnter},
		{protocol.EventMouseLeave, props.OnMouseLeave},
		{protocol.EventInput, props.OnInput},
		{protocol.EventSubmit, props.OnSubmit},
		{protocol.EventDismiss, props.OnDismiss},
		{protocol.EventPointer, props.OnPointer},
		{protocol.EventPresentation, props.OnPresentationChange},
		{protocol.EventMenuSelect, props.OnSelect},
		{protocol.EventKeyDown, props.OnKeyDown},
		{protocol.EventKeyUp, props.OnKeyUp},
		{protocol.EventMouseDown, props.OnMouseDown},
		{protocol.EventMouseUp, props.OnMouseUp},
		{protocol.EventMouseMove, props.OnMouseMove},
		{protocol.EventDoubleClick, props.OnDoubleClick},
		{protocol.EventWheel, props.OnWheel},
		{protocol.EventContextMenu, props.OnContextMenu},
		{protocol.EventPinch, props.OnPinch},
		{protocol.EventRotate, props.OnRotate},
		{protocol.EventSmartMagnify, props.OnSmartMagnify},
		{protocol.EventPressure, props.OnPressure},
		{protocol.EventFocus, props.OnFocus},
		{protocol.EventBlur, props.OnBlur},
		{protocol.EventAction, props.OnAction},
		{protocol.EventDragStart, props.OnDragStart},
		{protocol.EventDragEnd, props.OnDragEnd},
		{protocol.EventDrop, props.OnDrop},
		{protocol.EventFilesDropped, props.OnFilesDropped},
		{protocol.EventComponentChange, props.OnComponentChange},
		{protocol.EventCommit, props.OnCommit},
	}
}

func applyPrimitiveBehavior(node *native.Node, props Props) {
	for _, listener := range primitiveListeners(props) {
		if listener.handler != nil {
			setListener(node, listener.kind, listener.handler)
		}
	}
	for _, property := range []struct {
		code  uint16
		value any
	}{
		{protocol.Invalid, props.Invalid},
		{protocol.Selected, props.Selected},
		{protocol.FocusOnPointer, props.FocusOnPointer},
		{protocol.FocusableWhenDisabled, props.FocusableWhenDisabled},
		{protocol.Overlay, props.Overlay},
		{protocol.FocusTrap, props.FocusTrap},
		{protocol.RestorePreviousFocus, props.RestorePreviousFocus},
		{protocol.AutoFocus, props.AutoFocus},
		{protocol.AccessibilityModal, props.AriaModal},
		{protocol.DismissOnEscape, props.DismissOnEscape},
		{protocol.DismissOnPointerOutside, props.DismissOnPointerOutside},
	} {
		if property.value != nil {
			bindExplicitBool(node, property.code, property.value)
		}
	}
	for _, property := range []struct {
		code  uint16
		value any
	}{
		{protocol.AccessibilityLabel, props.AriaLabel},
		{protocol.Role, props.Role},
		{protocol.TooltipPlacement, props.TooltipPlacement},
		{protocol.AnchorPlacement, props.AnchorPlacement},
	} {
		if property.value != nil {
			bindString(node, property.code, property.value)
		}
	}
	for _, property := range []struct {
		code  uint16
		value any
	}{
		{protocol.TabIndex, props.TabIndex},
		{protocol.TooltipGap, props.TooltipGap},
		{protocol.TooltipViewportMargin, props.TooltipViewportMargin},
		{protocol.AnchorTarget, props.AnchorTarget},
		{protocol.AnchorGap, props.AnchorGap},
		{protocol.ViewportMargin, props.ViewportMargin},
	} {
		if property.value != nil {
			bindNumber(node, property.code, property.value)
		}
	}
	if props.TooltipDelay != nil {
		bindDeclaration(node, props.TooltipDelay, func(value any) { setMilliseconds(node, protocol.TooltipDelay, value) })
	}
	if props.TooltipText != nil {
		bindDeclaration(node, props.TooltipText, func(value any) {
			if value == nil {
				native.ClearProperty(node, protocol.Tooltip)
				return
			}
			text, ok := value.(string)
			if !ok {
				panic(fmt.Sprintf("QuickGUI tooltip must be text, got %T", value))
			}
			if len(text) > maxTooltipTextBytes {
				panic("QuickGUI tooltip text is too long")
			}
			setString(node, protocol.Tooltip, text)
		})
	}
	if props.Keymap != nil {
		bindDeclaration(node, props.Keymap, func(value any) { setKeymap(node, value) })
	}
	if props.Draggable != nil {
		bindDeclaration(node, props.Draggable, func(value any) { setDragSource(node, value) })
	}
	if props.DropKinds != nil {
		bindDeclaration(node, props.DropKinds, func(value any) { setDropKinds(node, value) })
	}
}

// Binding declarations use the same owner as scalar properties. Conditional
// options therefore dispose their subscriptions before restoring the base props.
func bindDeclaration(node *native.Node, value any, write func(any)) {
	node.Bind(func() { write(resolveDeclaration(value)) })
}

// KeyBinding maps one accelerator to the action id delivered to OnAction.
type KeyBinding struct {
	Keys   string
	Action string
}

// DragFile is a native file URL offered by a drag source.
type DragFile struct {
	Path      string `json:"path"`
	Directory bool   `json:"directory,omitempty"`
}

// DragSource declares a local id and optional native text, URL, or files payload.
type DragSource struct {
	ID    string     `json:"id,omitempty"`
	Text  string     `json:"text,omitempty"`
	URL   string     `json:"url,omitempty"`
	Files []DragFile `json:"files,omitempty"`
}

func setKeymap(node *native.Node, value any) {
	if value == nil {
		native.ClearProperty(node, protocol.Keymap)
		return
	}
	bindings := map[string]string{}
	switch value := value.(type) {
	case []KeyBinding:
		for _, binding := range value {
			bindings[binding.Keys] = binding.Action
		}
	case map[string]string:
		bindings = value
	default:
		panic(fmt.Sprintf("QuickGUI keymap must be []KeyBinding or map[string]string, got %T", value))
	}
	for keys, action := range bindings {
		if keys == "" || action == "" {
			panic("QuickGUI keymap keys and action ids must not be empty")
		}
	}
	setJson(node, protocol.Keymap, maxKeymapJSONBytes, bindings)
}

func setDragSource(node *native.Node, value any) {
	if enabled, ok := value.(bool); ok {
		if enabled {
			value = DragSource{}
		} else {
			value = nil
		}
	}
	switch value.(type) {
	case nil, DragSource, *DragSource:
		setJson(node, protocol.Draggable, maxDragJSONBytes, value)
	default:
		panic(fmt.Sprintf("QuickGUI draggable must be bool or DragSource, got %T", value))
	}
}

func setDropKinds(node *native.Node, value any) {
	if value == nil {
		native.ClearProperty(node, protocol.DropKinds)
		return
	}
	var kinds []string
	switch value := value.(type) {
	case string:
		kinds = []string{value}
	case []string:
		kinds = value
	default:
		panic(fmt.Sprintf("QuickGUI drop kinds must be a string or []string, got %T", value))
	}
	for _, kind := range kinds {
		if kind != "local" && kind != "files" {
			panic("QuickGUI drop kinds must be local or files")
		}
	}
	if kinds == nil {
		kinds = []string{}
	}
	setJson(node, protocol.DropKinds, maxDragJSONBytes, kinds)
}

// Aliases share one native listener slot.
func OnChange(handler func(*native.Event)) Option       { return OnInput(handler) }
func OnPointerEnter(handler func(*native.Event)) Option { return OnMouseEnter(handler) }
func OnPointerLeave(handler func(*native.Event)) Option { return OnMouseLeave(handler) }
func OnMenuSelect(handler func(*native.Event)) Option   { return OnSelect(handler) }
func OnActivate(handler func(*native.Event)) Option     { return OnCommit(handler) }
