package ui

import (
	"encoding/json"
	"fmt"
	"reflect"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

// Props configure a primitive node. Accessor children and values create bindings.
type Props struct {
	Style                   StyleBuilder
	Children                any
	OnClick                 func(*native.Event)
	OnMouseEnter            func(*native.Event)
	OnMouseLeave            func(*native.Event)
	OnInput                 func(*native.Event)
	OnSubmit                func(*native.Event)
	OnDismiss               func(*native.Event)
	OnStatus                func(*native.Event)
	OnPointer               func(*native.Event)
	OnPresentationChange    func(*native.Event)
	OnSelect                func(*native.Event)
	OnKeyDown               func(*native.Event)
	OnKeyUp                 func(*native.Event)
	OnMouseDown             func(*native.Event)
	OnMouseUp               func(*native.Event)
	OnMouseMove             func(*native.Event)
	OnDoubleClick           func(*native.Event)
	OnWheel                 func(*native.Event)
	OnContextMenu           func(*native.Event)
	OnPinch                 func(*native.Event)
	OnRotate                func(*native.Event)
	OnSmartMagnify          func(*native.Event)
	OnPressure              func(*native.Event)
	OnFocus                 func(*native.Event)
	OnBlur                  func(*native.Event)
	OnAction                func(*native.Event)
	OnDragStart             func(*native.Event)
	OnDragEnd               func(*native.Event)
	OnDrop                  func(*native.Event)
	OnFilesDropped          func(*native.Event)
	OnComponentChange       func(*native.Event)
	OnCommit                func(*native.Event)
	Disabled                any
	Value                   any
	Placeholder             string
	Multiline               bool
	AriaLabel               any
	Selected                any
	Group                   any
	FocusOnPointer          any
	FocusableWhenDisabled   any
	HitSlop                 any
	HitSlopTop              any
	HitSlopRight            any
	HitSlopBottom           any
	HitSlopLeft             any
	Ref                     func(*native.Node)
	Password                any
	Streaming               any
	EstimatedItemHeight     any
	Overscan                any
	OverscanPixels          any
	ItemHeights             any
	ListAlignment           string
	FollowMode              string
	ObjectFit               string
	ShaderParameters        any
	Invalid                 any
	Role                    any
	TabIndex                any
	Overlay                 any
	FocusTrap               any
	RestorePreviousFocus    any
	AutoFocus               any
	AriaModal               any
	DismissOnEscape         any
	DismissOnPointerOutside any
	TooltipText             any
	TooltipPlacement        any
	TooltipDelay            any
	TooltipGap              any
	TooltipViewportMargin   any
	AnchorTarget            any
	AnchorPlacement         any
	AnchorGap               any
	ViewportMargin          any
	Keymap                  any
	Draggable               any
	DropKinds               any
}

func applyProps(node *native.Node, props Props) {
	applyPropValues(node, props)
	insertChildren(node, props.Children)
	if props.Ref != nil {
		props.Ref(node)
	}
}

func applyPropValues(node *native.Node, props Props) {
	applyStyle(node, props.Style.style)
	if props.Disabled != nil {
		bindExplicitBool(node, protocol.Disabled, props.Disabled)
	}
	if props.Password != nil {
		bindExplicitBool(node, protocol.Password, props.Password)
	}
	if props.Streaming != nil {
		bindExplicitBool(node, protocol.Streaming, props.Streaming)
	}
	if props.EstimatedItemHeight != nil {
		bindNumber(node, protocol.EstimatedItemHeight, props.EstimatedItemHeight)
	}
	if props.Overscan != nil {
		bindNumber(node, protocol.Overscan, props.Overscan)
	}
	if props.OverscanPixels != nil {
		bindNumber(node, protocol.OverscanPixels, props.OverscanPixels)
	}
	if props.ItemHeights != nil {
		bindItemHeights(node, props.ItemHeights)
	}
	if props.ListAlignment != "" {
		setString(node, protocol.ListAlignment, props.ListAlignment)
	}
	if props.FollowMode != "" {
		setString(node, protocol.FollowMode, props.FollowMode)
	}
	if props.ObjectFit != "" {
		setString(node, protocol.ObjectFit, props.ObjectFit)
	}
	if props.ShaderParameters != nil {
		bindShaderParameters(node, props.ShaderParameters)
	}
	if props.Value != nil {
		bindValue(node, protocol.Value, props.Value)
	}
	if props.Placeholder != "" {
		native.SetString(node, protocol.Placeholder, props.Placeholder)
	}
	if props.Multiline {
		native.SetBoolean(node, protocol.Multiline, true)
	}
	applyPrimitiveBehavior(node, props)
	if props.Group != nil {
		bindHoverGroup(node, props.Group)
	}
	for _, property := range []struct {
		code  uint16
		value any
	}{
		{protocol.HitSlop, props.HitSlop},
		{protocol.HitSlopTop, props.HitSlopTop},
		{protocol.HitSlopRight, props.HitSlopRight},
		{protocol.HitSlopBottom, props.HitSlopBottom},
		{protocol.HitSlopLeft, props.HitSlopLeft},
	} {
		if property.value != nil {
			bindNumber(node, property.code, property.value)
		}
	}
}

func setListener(node *native.Node, eventType int, handler func(*native.Event)) {
	owner := node.BindingOwner()
	native.SetEventListener(
		node,
		eventType,
		func(event *native.Event) {
			reactive.RunWithOwner(owner, func() struct{} {
				reactive.Batch(func() { handler(event) })
				return struct{}{}
			})
		},
	)
}

func bindValue(node *native.Node, code uint16, value any) {
	switch typed := value.(type) {
	case func() string:
		node.Bind(func() {
			native.SetString(node, code, typed())
		})
	case reactive.Accessor[string]:
		node.Bind(func() {
			native.SetString(node, code, typed())
		})
	case string:
		native.SetString(node, code, typed)
	default:
		native.SetString(node, code, fmt.Sprint(typed))
	}
}

func setExplicitBool(node *native.Node, code uint16, value any) {
	if value == nil {
		native.ClearProperty(node, code)
		return
	}
	switch typed := value.(type) {
	case bool:
		native.SetBoolean(node, code, typed)
	case *bool:
		if typed == nil {
			native.ClearProperty(node, code)
			return
		}
		native.SetBoolean(node, code, *typed)
	default:
		panic(fmt.Sprintf("QuickGUI expected a bool, got %T", value))
	}
}

func setComponentValue(node *native.Node, code uint16, value string) {
	if value == "" {
		native.ClearProperty(node, code)
		return
	}
	if len(value) > protocol.MaxComponentValueBytes {
		panic(fmt.Sprintf("QuickGUI component scopes and values are bounded to %d bytes", protocol.MaxComponentValueBytes))
	}
	native.SetString(node, code, value)
}

func setMilliseconds(node *native.Node, code uint16, value any) {
	if value == nil {
		native.ClearProperty(node, code)
		return
	}
	milliseconds, ok := durationMilliseconds(value)
	if !ok {
		panic(fmt.Sprintf("QuickGUI duration %v must be milliseconds or seconds", value))
	}
	native.SetNumber(node, code, float32(milliseconds))
}

func setExtent(node *native.Node, code uint16, value *Extent) {
	if value == nil {
		native.ClearProperty(node, code)
		return
	}
	if !isFinite(value.Width) || !isFinite(value.Height) {
		panic("QuickGUI extents must be finite numbers")
	}
	native.SetString(node, code, fmt.Sprintf("[%v,%v]", value.Width, value.Height))
}

func setMenuLink(node *native.Node, code uint16, value string) {
	if value == "" {
		native.ClearProperty(node, code)
		return
	}
	if len(value) > protocol.MaxMenuLinkBytes {
		panic(fmt.Sprintf("QuickGUI menu links are bounded to %d bytes", protocol.MaxMenuLinkBytes))
	}
	native.SetString(node, code, value)
}

func packedColor(value any) *uint32 {
	if value == nil || value == "" {
		return nil
	}
	color := native.ParseColor(value)
	return &color
}

func setJson(node *native.Node, code uint16, limit int, value any) {
	if value == nil {
		native.ClearProperty(node, code)
		return
	}
	payload, err := json.Marshal(value)
	if err != nil {
		panic(err)
	}
	if len(payload) > limit {
		panic(fmt.Sprintf("QuickGUI component declarations are bounded to %d bytes", limit))
	}
	native.SetString(node, code, string(payload))
}

func setInputType(node *native.Node, value string) {
	native.SetBoolean(node, protocol.Password, value == "password")
}

func bindHoverGroup(node *native.Node, value any) {
	bindDeclaration(node, value, func(value any) { setHoverGroup(node, value) })
}

func setHoverGroup(node *native.Node, value any) {
	const code = protocol.HoverGroup
	if value == nil {
		native.ClearProperty(node, code)
		return
	}
	switch typed := value.(type) {
	case bool:
		if typed {
			native.SetBoolean(node, code, true)
		} else {
			native.ClearProperty(node, code)
		}
	case string:
		name := strings.TrimSpace(typed)
		if name == "" {
			native.ClearProperty(node, code)
			return
		}
		if len(name) > protocol.MaxHoverGroupNameBytes {
			panic(fmt.Sprintf("QuickGUI hover group names are bounded to %d bytes", protocol.MaxHoverGroupNameBytes))
		}
		native.SetString(node, code, name)
	default:
		panic(fmt.Sprintf("QuickGUI group %T is not a bool or string", value))
	}
}

func insertChildren(parent *native.Node, children any) {
	switch typed := children.(type) {
	case []any:
		for _, child := range typed {
			insertChildren(parent, child)
		}
	case func() *Element, func() *native.Node:
		insertChildBlock(parent, func() []*native.Node { return childNodes(children) })
	default:
		if native.IsComponent(children) {
			insertChildBlock(parent, func() []*native.Node { return componentNodes(children) })
			return
		}
		for _, child := range childNodes(children) {
			native.InsertNode(parent, child, nil)
		}
	}
}

// DynamicText is a text node that follows a reactive string.
func DynamicText(value func() string) *native.Node {
	node := native.CreateText("")
	last := ""
	node.Bind(func() {
		next := value()
		if next != last {
			last = next
			native.ReplaceText(node, next)
		}
	})
	return node
}

// Region is one sentinel whose content is replaced reactively.
type Region struct {
	Sentinel    *native.Node
	ParentOwner *reactive.Owner
	owner       *reactive.Owner
}

func NewRegion() *Region {
	sentinel := native.CreateSentinel()
	return &Region{Sentinel: sentinel, ParentOwner: sentinel.BindingOwner()}
}

func (r *Region) Clear() {
	if r.owner != nil {
		owner := r.owner
		r.owner = nil
		reactive.DisposeOwner(owner)
	}
	if r.Sentinel.Group != nil {
		parent := r.Sentinel.Parent
		if parent != nil {
			for _, node := range r.Sentinel.Group {
				native.RemoveNode(parent, node)
			}
		}
		r.Sentinel.Group = nil
	}
}

func (r *Region) Replace(render func() []*native.Node) {
	r.Clear()
	owner := reactive.NewOwner(r.ParentOwner)
	owner.Controller = reactive.GetComputation()
	r.owner = owner
	nodes := reactive.RunWithOwner(owner, render)
	r.Sentinel.Group = nodes
	parent := r.Sentinel.Parent
	if parent != nil {
		for _, node := range nodes {
			native.InsertNode(parent, node, r.Sentinel)
		}
	}
}

// Fragment groups explicitly supplied children without a layout wrapper.
func Fragment(children ...any) *native.Node {
	sentinel := native.CreateSentinel()
	var collect func(any)
	collect = func(child any) {
		if list, ok := child.([]any); ok {
			for _, item := range list {
				collect(item)
			}
			return
		}
		var nodes []*native.Node
		if native.IsComponent(child) {
			nodes = reactive.RunWithOwner(sentinel.BindingOwner(), func() []*native.Node { return childNodes(child) })
		} else {
			nodes = childNodes(child)
		}
		sentinel.Group = append(sentinel.Group, nodes...)
	}
	for _, child := range children {
		collect(child)
	}
	return sentinel
}

// Dynamic mounts the selected component, replacing its subtree when the selector's
// signals change. Reads inside the component do not rerun the selector.
func Dynamic(selectComponent func() Component) *native.Node {
	region := NewRegion()
	region.Sentinel.Bind(func() {
		component := selectComponent()
		region.Replace(func() []*native.Node {
			return reactive.Untrack(func() []*native.Node { return componentNodes(component) })
		})
	})
	return region.Sentinel
}

// Show renders children while when is true, else fallback. Children are created lazily.
func Show(condition any, children Component, fallback ...Component) *native.Node {
	for _, component := range append([]Component{children}, fallback...) {
		if component != nil && !native.IsComponent(component) {
			panic("QuickGUI construction callbacks must return *ui.Element or *native.Node; func() declaration blocks are not supported")
		}
	}
	when := booleanRead(condition)
	region := NewRegion()
	shown := 0
	region.Sentinel.Bind(func() {
		if when() {
			if shown == 1 {
				return
			}
			shown = 1
			region.Replace(func() []*native.Node {
				return reactive.Untrack(func() []*native.Node { return componentNodes(children) })
			})
			return
		}
		if shown == 2 {
			return
		}
		shown = 2
		if len(fallback) == 0 || fallback[0] == nil {
			region.Clear()
			return
		}
		region.Replace(func() []*native.Node {
			return reactive.Untrack(func() []*native.Node { return componentNodes(fallback[0]) })
		})
	})
	return region.Sentinel
}

type forRow[T any] struct {
	item  *reactive.Signal[T]
	key   any
	node  *native.Node
	owner *reactive.Owner
	index *reactive.Signal[int]
}

func rowKey[T any](key func(T) any, item T, index int) any {
	if key == nil {
		return index
	}
	value := key(item)
	if value != nil && !reflect.ValueOf(value).Comparable() {
		panic("QuickGUI list keys must be comparable Go values")
	}
	return value
}

// For renders one row per item, reusing rows whose key survives.
func For[T any](each func() []T, children any, key func(T) any, fallback Component) *native.Node {
	build := rowRenderer[T](children)
	return renderList(each, key, func(item func() T, index func() int) *native.Node {
		return build(item(), index)
	}, fallback, false)
}

// KeyedFor keeps each row mounted while its keyed data changes.
func KeyedFor[T any](each func() []T, key func(T) any, children any, fallback Component) *native.Node {
	return renderList(each, key, rowRenderer[func() T](children), fallback, true)
}

// Adapt the row signature once when mounting the list. Updates and keyed moves
// retain the row node; they never reflect over a renderer or rebuild its tree.
func rowRenderer[T any](children any) func(T, func() int) *native.Node {
	switch build := children.(type) {
	case func(T, func() int) *Element:
		return func(item T, index func() int) *native.Node {
			return renderComponent(func() *Element { return build(item, index) })
		}
	case func(T, func() int) *native.Node:
		return func(item T, index func() int) *native.Node {
			return renderComponent(func() *native.Node { return build(item, index) })
		}
	case func(T) *Element:
		return func(item T, _ func() int) *native.Node {
			return renderComponent(func() *Element { return build(item) })
		}
	case func(T) *native.Node:
		return func(item T, _ func() int) *native.Node {
			return renderComponent(func() *native.Node { return build(item) })
		}
	default:
		value := reflect.ValueOf(children)
		if value.IsValid() {
			for _, signature := range []reflect.Type{
				reflect.TypeFor[func(T, func() int) *Element](),
				reflect.TypeFor[func(T, func() int) *native.Node](),
				reflect.TypeFor[func(T) *Element](),
				reflect.TypeFor[func(T) *native.Node](),
			} {
				if value.Type().ConvertibleTo(signature) {
					return rowRenderer[T](value.Convert(signature).Interface())
				}
			}
		}
		panic(fmt.Sprintf("QuickGUI row renderers must return *ui.Element or *native.Node and accept an item with an optional index accessor, got %T", children))
	}
}

func renderList[T any](
	each func() []T,
	keyFor func(T) any,
	render func(item func() T, index func() int) *native.Node,
	fallback Component,
	reactiveItems bool,
) *native.Node {
	region := NewRegion()
	var rows []forRow[T]
	showingFallback := false
	disposeRow := func(row forRow[T]) {
		reactive.DisposeOwner(row.owner)
		parent := region.Sentinel.Parent
		if parent != nil && row.node.Parent == parent {
			native.RemoveNode(parent, row.node)
		}
	}
	region.Sentinel.Bind(func() {
		items := each()
		reactive.Untrack(func() struct{} {
			if len(items) == 0 {
				for _, row := range rows {
					disposeRow(row)
				}
				rows = nil
				if showingFallback {
					return struct{}{}
				}
				region.Sentinel.Group = nil
				if fallback != nil && !showingFallback {
					showingFallback = true
					region.Replace(func() []*native.Node { return native.CollectChildren(fallback) })
				}
				return struct{}{}
			}
			if showingFallback {
				showingFallback = false
				region.Clear()
			}
			existing := map[any]forRow[T]{}
			for _, row := range rows {
				existing[row.key] = row
			}
			next := make([]forRow[T], 0, len(items))
			seen := make(map[any]bool, len(items))
			for index, item := range items {
				key := rowKey(keyFor, item, index)
				if seen[key] {
					panic(fmt.Sprintf("duplicate QuickGUI list key: %v", key))
				}
				seen[key] = true
				row, ok := existing[key]
				if ok && (reactiveItems || identicalItem(row.item.Peek(), item)) {
					delete(existing, key)
					row.item.Write(item)
					if row.index.Peek() != index {
						row.index.Write(index)
					}
					next = append(next, row)
					continue
				}
				if ok {
					delete(existing, key)
					disposeRow(row)
				}
				owner := reactive.NewOwner(region.ParentOwner)
				owner.Controller = reactive.GetComputation()
				indexSignal := reactive.NewSignal(index)
				itemSignal := reactive.NewSignal(item)
				node := reactive.RunWithOwner(owner, func() *native.Node {
					return render(func() T { return itemSignal.Read() }, func() int { return indexSignal.Read() })
				})
				next = append(next, forRow[T]{item: itemSignal, key: key, node: node, owner: owner, index: indexSignal})
			}
			for _, row := range existing {
				disposeRow(row)
			}
			rows = next
			nodes := make([]*native.Node, len(next))
			for i, row := range next {
				nodes[i] = row.node
			}
			region.Sentinel.Group = nodes
			parent := region.Sentinel.Parent
			if parent != nil {
				anchor := region.Sentinel
				for i := len(nodes) - 1; i >= 0; i-- {
					node := nodes[i]
					position := indexOf(parent.Children, node)
					anchorPosition := indexOf(parent.Children, anchor)
					if position < 0 || position+1 != anchorPosition {
						native.InsertNode(parent, node, anchor)
					}
					anchor = firstGroupNode(node)
				}
			}
			return struct{}{}
		})
	})
	return region.Sentinel
}

func identicalItem[T any](a, b T) bool {
	return reflect.ValueOf(&a).Elem().Comparable() && reflect.ValueOf(&b).Elem().Comparable() && any(a) == any(b)
}

// A fragment's sentinel follows its children. The next row must be inserted
// before the entire fragment, not between its children and its sentinel.
func firstGroupNode(node *native.Node) *native.Node {
	for len(node.Group) != 0 {
		node = node.Group[0]
	}
	return node
}

func indexOf(nodes []*native.Node, node *native.Node) int {
	for i, candidate := range nodes {
		if candidate == node {
			return i
		}
	}
	return -1
}
