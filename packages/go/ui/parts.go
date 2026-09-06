package ui

import (
	"fmt"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// PartChildren are created after the part's own context exists.
type PartChildren func() *native.Node

// PartProps are the application-facing props every compound part accepts.
type PartProps struct {
	Ref            func(*native.Node)
	Style          any
	Children       PartChildren
	Disabled       any
	Role           string
	FocusOnPointer any
	Group          any
	AriaLabel      any
	TabIndex       *float64
	OnClick        func(*native.Event)
	OnDoubleClick  func(*native.Event)
	OnContextMenu  func(*native.Event)
	OnPointer      func(*native.Event)
	OnMouseEnter   func(*native.Event)
	OnMouseLeave   func(*native.Event)
	OnMouseDown    func(*native.Event)
	OnMouseUp      func(*native.Event)
	OnKeyDown      func(*native.Event)
	OnKeyUp        func(*native.Event)
	OnFocus        func(*native.Event)
	OnBlur         func(*native.Event)
}

// InputPartProps are the props of a part that hosts a native text input.
type InputPartProps struct {
	PartProps
	Type        string
	Value       any
	Placeholder string
	Multiline   bool
	OnInput     func(*native.Event)
	OnChange    func(*native.Event)
	OnSubmit    func(*native.Event)
}

func bindExplicitBool(node *native.Node, code uint16, value any) {
	switch typed := value.(type) {
	case func() bool:
		reactive.CreateRenderEffect(func() {
			setExplicitBool(node, code, typed())
		})
	case func() *bool:
		reactive.CreateRenderEffect(func() {
			setExplicitBool(node, code, typed())
		})
	case bool, *bool:
		setExplicitBool(node, code, typed)
	default:
		panic(fmt.Sprintf("QuickGUI expected a bool or accessor, got %T", value))
	}
}

func bindNumber(node *native.Node, code uint16, value any) {
	switch typed := value.(type) {
	case func() float64:
		reactive.CreateRenderEffect(func() {
			setNumber(node, code, typed())
		})
	case func() int:
		reactive.CreateRenderEffect(func() {
			setNumber(node, code, typed())
		})
	case func() *float64:
		reactive.CreateRenderEffect(func() {
			setNumber(node, code, typed())
		})
	case func() *int:
		reactive.CreateRenderEffect(func() {
			setNumber(node, code, typed())
		})
	case float64, float32, int, int32, int64, *float64, *float32, *int, *int32, *int64:
		setNumber(node, code, typed)
	default:
		panic(fmt.Sprintf("QuickGUI expected a number or accessor, got %T", value))
	}
}

func resolveNumber(value any) *float64 {
	if value == nil {
		return nil
	}
	if unwrapped, ok := unwrapNumericPointer(value); ok {
		if unwrapped == nil {
			return nil
		}
		return resolveNumber(unwrapped)
	}
	switch typed := value.(type) {
	case func() float64:
		number := typed()
		return &number
	case func() int:
		number := float64(typed())
		return &number
	case func() *float64:
		return typed()
	case func() *int:
		return resolveNumber(typed())
	case float64:
		return &typed
	case float32:
		number := float64(typed)
		return &number
	case int:
		number := float64(typed)
		return &number
	default:
		panic(fmt.Sprintf("QuickGUI expected a number or accessor, got %T", value))
	}
}

func bindString(node *native.Node, code uint16, value any) {
	switch typed := value.(type) {
	case func() string:
		reactive.CreateRenderEffect(func() {
			setString(node, code, typed())
		})
	case func() *string:
		reactive.CreateRenderEffect(func() {
			next := typed()
			if next == nil {
				setString(node, code, "")
				return
			}
			setString(node, code, *next)
		})
	case *string:
		if typed == nil {
			setString(node, code, "")
			return
		}
		setString(node, code, *typed)
	case string:
		setString(node, code, typed)
	default:
		panic(fmt.Sprintf("QuickGUI expected a string or accessor, got %T", value))
	}
}

func resolveBoolean(value any) *bool {
	if value == nil {
		return nil
	}
	switch typed := value.(type) {
	case func() bool:
		flag := typed()
		return &flag
	case func() *bool:
		return typed()
	case bool:
		return &typed
	case *bool:
		return typed
	default:
		panic(fmt.Sprintf("QuickGUI expected a bool or accessor, got %T", value))
	}
}

func resolveString(value any) *string {
	if value == nil {
		return nil
	}
	switch typed := value.(type) {
	case func() string:
		text := typed()
		return &text
	case func() *string:
		return typed()
	case string:
		return &typed
	case *string:
		return typed
	default:
		panic(fmt.Sprintf("QuickGUI expected a string or accessor, got %T", value))
	}
}

func resolveStyle(value any) *Style {
	if value == nil {
		return nil
	}
	switch typed := value.(type) {
	case Style:
		return &typed
	case *Style:
		return typed
	case []Style:
		merged := Style{}
		for _, style := range typed {
			mergeStyle(&merged, style)
		}
		return &merged
	case func() Style:
		style := typed()
		return &style
	case func() *Style:
		return typed()
	case func() []Style:
		return resolveStyle(typed())
	case func() any:
		return resolveStyle(typed())
	default:
		panic(fmt.Sprintf("QuickGUI style %T is not a style record or list", value))
	}
}

func bindPartStyle(node *native.Node, style any) {
	switch style.(type) {
	case func() Style, func() *Style, func() []Style, func() any:
		reactive.CreateRenderEffect(func() {
			if next := resolveStyle(style); next != nil {
				applyStyle(node, *next)
			}
		})
	default:
		if next := resolveStyle(style); next != nil {
			applyStyle(node, *next)
		}
	}
}

func applyPart(node *native.Node, props PartProps) {
	if props.Style != nil {
		bindPartStyle(node, props.Style)
	}
	applyPartBehavior(node, props)
}

func applyPartBehavior(node *native.Node, props PartProps) {
	if props.Disabled != nil {
		bindExplicitBool(node, protocol.Disabled, props.Disabled)
	}
	if props.FocusOnPointer != nil {
		bindExplicitBool(node, protocol.FocusOnPointer, props.FocusOnPointer)
	}
	if props.Group != nil {
		switch typed := props.Group.(type) {
		case func() bool:
			reactive.CreateRenderEffect(func() { setHoverGroup(node, protocol.Group, typed()) })
		case func() string:
			reactive.CreateRenderEffect(func() { setHoverGroup(node, protocol.Group, typed()) })
		case func() any:
			reactive.CreateRenderEffect(func() { setHoverGroup(node, protocol.Group, typed()) })
		default:
			setHoverGroup(node, protocol.Group, props.Group)
		}
	}
	if props.Role != "" {
		setString(node, protocol.Role, props.Role)
	}
	if props.AriaLabel != nil {
		bindString(node, protocol.AccessibilityLabel, props.AriaLabel)
	}
	if props.TabIndex != nil {
		native.SetNumber(node, protocol.TabIndex, float32(*props.TabIndex))
	}
	if props.OnClick != nil {
		setListener(node, protocol.EventClick, props.OnClick)
	}
	if props.OnDoubleClick != nil {
		setListener(node, protocol.EventDoubleClick, props.OnDoubleClick)
	}
	if props.OnContextMenu != nil {
		setListener(node, protocol.EventContextMenu, props.OnContextMenu)
	}
	if props.OnPointer != nil {
		setListener(node, protocol.EventPointer, props.OnPointer)
	}
	if props.OnMouseEnter != nil {
		setListener(node, protocol.EventMouseEnter, props.OnMouseEnter)
	}
	if props.OnMouseLeave != nil {
		setListener(node, protocol.EventMouseLeave, props.OnMouseLeave)
	}
	if props.OnMouseDown != nil {
		setListener(node, protocol.EventMouseDown, props.OnMouseDown)
	}
	if props.OnMouseUp != nil {
		setListener(node, protocol.EventMouseUp, props.OnMouseUp)
	}
	if props.OnKeyDown != nil {
		setListener(node, protocol.EventKeyDown, props.OnKeyDown)
	}
	if props.OnKeyUp != nil {
		setListener(node, protocol.EventKeyUp, props.OnKeyUp)
	}
	if props.OnFocus != nil {
		setListener(node, protocol.EventFocus, props.OnFocus)
	}
	if props.OnBlur != nil {
		setListener(node, protocol.EventBlur, props.OnBlur)
	}
}

func applyInputPart(node *native.Node, props InputPartProps) {
	if props.Type != "" {
		setInputType(node, props.Type)
	}
	if props.Value != nil {
		bindString(node, protocol.Value, props.Value)
	}
	if props.Placeholder != "" {
		setString(node, protocol.Placeholder, props.Placeholder)
	}
	if props.Multiline {
		setExplicitBool(node, protocol.Multiline, true)
	}
	if props.OnInput != nil {
		setListener(node, protocol.EventInput, props.OnInput)
	}
	if props.OnChange != nil {
		setListener(node, protocol.EventInput, props.OnChange)
	}
	if props.OnSubmit != nil {
		setListener(node, protocol.EventSubmit, props.OnSubmit)
	}
}

func createPart(tag uint8, props PartProps) *native.Node {
	node := native.CreateElement(tag)
	applyPart(node, props)
	return node
}

func createViewPart(props PartProps) *native.Node {
	return createPart(protocol.TagView, props)
}

func createButtonPart(props PartProps) *native.Node {
	return createPart(protocol.TagButton, props)
}

func setPart(node *native.Node, part string, scope, partValue string) {
	setString(node, protocol.Part, part)
	if scope != "" {
		setComponentValue(node, protocol.Scope, scope)
	}
	if partValue != "" {
		setComponentValue(node, protocol.PartValue, partValue)
	}
}

func finishPart(node *native.Node, props PartProps) *native.Node {
	if props.Children != nil {
		if child := props.Children(); child != nil {
			native.InsertNode(node, child, nil)
		}
	}
	if props.Ref != nil {
		props.Ref(node)
	}
	return node
}

func forwardClick(handler func(*native.Event), activate func(*native.Event)) func(*native.Event) {
	return func(event *native.Event) {
		if handler != nil {
			handler(event)
		}
		if !event.DefaultPrevented {
			activate(event)
		}
	}
}

var nextComponentScope = 1

func createComponentScope(prefix string) string {
	scope := fmt.Sprintf("%s-%d", prefix, nextComponentScope)
	nextComponentScope++
	return scope
}

func createPartContext[T any]() *reactive.Context[*T] {
	return reactive.CreateContext[*T](nil)
}

func requireContext[T any](context *reactive.Context[*T], part, root string) *T {
	value := context.Use()
	if value == nil {
		panic(fmt.Sprintf("%s must be used inside %s", part, root))
	}
	return value
}
