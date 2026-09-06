package ui

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// Props are the host element properties, matching the TypeScript NativeProps shape.
type Props struct {
	Style          Style
	Children       any
	OnClick        func(*native.Event)
	OnInput        func(*native.Event)
	OnSubmit       func(*native.Event)
	OnDoubleClick  func(*native.Event)
	OnContextMenu  func(*native.Event)
	OnPointer      func(*native.Event)
	Disabled       bool
	Value          any
	Placeholder    string
	Multiline      bool
	AriaLabel      string
	Selected       bool
	Group          bool
	FocusOnPointer *bool
}

func applyProps(node *native.Node, props Props) {
	applyStyle(node, props.Style)
	if props.Disabled {
		native.SetBoolean(node, protocol.Disabled, true)
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
	if props.OnClick != nil {
		setListener(node, protocol.EventClick, props.OnClick)
	}
	if props.OnInput != nil {
		setListener(node, protocol.EventInput, props.OnInput)
	}
	if props.OnSubmit != nil {
		setListener(node, protocol.EventSubmit, props.OnSubmit)
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
	if props.AriaLabel != "" {
		native.SetString(node, protocol.AccessibilityLabel, props.AriaLabel)
	}
	if props.Selected {
		native.SetBoolean(node, protocol.Selected, true)
	}
	if props.Group {
		native.SetBoolean(node, protocol.Group, true)
	}
	if props.FocusOnPointer != nil {
		native.SetBoolean(node, protocol.FocusOnPointer, *props.FocusOnPointer)
	}
	insertChildren(node, props.Children)
}

func setListener(node *native.Node, eventType int, handler func(*native.Event)) {
	native.SetEventListener(node, eventType, func(event *native.Event) {
		reactive.Batch(func() {
			handler(event)
		})
	})
}

func bindValue(node *native.Node, code uint16, value any) {
	switch typed := value.(type) {
	case func() string:
		reactive.CreateRenderEffect(func() {
			native.SetString(node, code, typed())
		})
	case string:
		native.SetString(node, code, typed)
	default:
		native.SetString(node, code, fmt.Sprint(typed))
	}
}

func setExplicitBool(node *native.Node, code uint16, value bool) {
	native.SetBoolean(node, code, value)
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

func setHoverGroup(node *native.Node, code uint16, value any) {
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
	if children == nil {
		return
	}
	switch typed := children.(type) {
	case *native.Node:
		native.InsertNode(parent, typed, nil)
	case string:
		native.InsertNode(parent, native.CreateText(typed), nil)
	case int, int32, int64, float32, float64:
		native.InsertNode(parent, native.CreateText(fmt.Sprint(typed)), nil)
	case func() string:
		native.InsertNode(parent, DynamicText(typed), nil)
	case []any:
		for _, child := range typed {
			insertChildren(parent, child)
		}
	case []*native.Node:
		for _, child := range typed {
			if child != nil {
				native.InsertNode(parent, child, nil)
			}
		}
	case bool:
		return
	default:
		panic(fmt.Sprintf("unsupported QuickGUI child %T", children))
	}
}

// DynamicText is a text node that follows a reactive string.
func DynamicText(value func() string) *native.Node {
	node := native.CreateText("")
	last := ""
	reactive.CreateRenderEffect(func() {
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
	return &Region{Sentinel: native.CreateSentinel(), ParentOwner: reactive.GetOwner()}
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

func Fragment(nodes []*native.Node) *native.Node {
	sentinel := native.CreateSentinel()
	sentinel.Group = nodes
	return sentinel
}

// DynamicMaybe is a region showing one node or nothing.
func DynamicMaybe(render func() *native.Node) *native.Node {
	region := NewRegion()
	reactive.CreateRenderEffect(func() {
		node := render()
		if node == nil {
			region.Clear()
			return
		}
		region.Replace(func() []*native.Node { return []*native.Node{node} })
	})
	return region.Sentinel
}

// CreateRenderer mounts render into a window. Pass the result as WindowOptions.Renderer.
func CreateRenderer(render func() *native.Node) native.Renderer {
	return func(window *native.Window) func() {
		disposed := false
		dispose := reactive.CreateRoot(func(disposeRoot func()) func() {
			node := render()
			native.InsertNode(window.Root, node, nil)
			return disposeRoot
		})
		window.Flush()
		return func() {
			if disposed {
				return
			}
			disposed = true
			dispose()
			window.Flush()
		}
	}
}

// Show renders children while when is true, else fallback. Children are created lazily.
func Show(when func() bool, children func() *native.Node, fallback ...func() *native.Node) *native.Node {
	region := NewRegion()
	shown := 0
	reactive.CreateRenderEffect(func() {
		if when() {
			if shown == 1 {
				return
			}
			shown = 1
			region.Replace(func() []*native.Node {
				return []*native.Node{reactive.Untrack(children)}
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
			return []*native.Node{reactive.Untrack(fallback[0])}
		})
	})
	return region.Sentinel
}

type forRow[T any] struct {
	item  *reactive.Signal[T]
	key   string
	node  *native.Node
	owner *reactive.Owner
	index *reactive.Signal[int]
}

func rowKey[T any](key func(T) any, item T, index int) string {
	if key == nil {
		return fmt.Sprintf("%d", index)
	}
	switch typed := key(item).(type) {
	case int:
		return fmt.Sprintf("n%d", typed)
	case string:
		return "s" + typed
	default:
		return fmt.Sprintf("v%v", typed)
	}
}

// For renders one row per item, reusing rows whose key survives.
func For[T any](each func() []T, children func(item T, index func() int) *native.Node, key func(T) any, fallback func() *native.Node) *native.Node {
	return renderList(each, key, func(item func() T, index func() int) *native.Node {
		return children(item(), index)
	}, fallback, false)
}

// KeyedFor keeps each row mounted while its keyed data changes.
func KeyedFor[T any](each func() []T, key func(T) any, children func(item func() T, index func() int) *native.Node, fallback func() *native.Node) *native.Node {
	return renderList(each, key, children, fallback, true)
}

func renderList[T any](
	each func() []T,
	keyFor func(T) any,
	render func(item func() T, index func() int) *native.Node,
	fallback func() *native.Node,
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
	reactive.CreateRenderEffect(func() {
		items := each()
		reactive.Untrack(func() struct{} {
			if len(items) == 0 {
				for _, row := range rows {
					disposeRow(row)
				}
				rows = nil
				region.Sentinel.Group = nil
				if fallback != nil && !showingFallback {
					showingFallback = true
					region.Replace(func() []*native.Node { return []*native.Node{fallback()} })
				}
				return struct{}{}
			}
			if showingFallback {
				showingFallback = false
				region.Clear()
			}
			existing := map[string]forRow[T]{}
			for _, row := range rows {
				existing[row.key] = row
			}
			next := make([]forRow[T], 0, len(items))
			for index, item := range items {
				key := rowKey(keyFor, item, index)
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
					anchor = node
				}
			}
			return struct{}{}
		})
	})
	return region.Sentinel
}

func identicalItem[T any](a, b T) bool {
	defer func() { _ = recover() }()
	return any(a) == any(b)
}

func indexOf(nodes []*native.Node, node *native.Node) int {
	for i, candidate := range nodes {
		if candidate == node {
			return i
		}
	}
	return -1
}
