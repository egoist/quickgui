package ui

import (
	"bytes"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

// Exercise the real native dispatch path for every primitive event, including
// conditional removal. A protocol event with no public prop is a parity failure.
func TestEveryNativeEventHasAConditionalPrimitiveHandler(t *testing.T) {
	names := []string{
		"Click", "MouseEnter", "MouseLeave", "Input", "Submit", "Dismiss", "Pointer",
		"PresentationChange", "Select", "KeyDown", "KeyUp", "MouseDown", "MouseUp", "MouseMove",
		"DoubleClick", "Wheel", "ContextMenu", "Pinch", "Rotate", "SmartMagnify", "Pressure",
		"Focus", "Blur", "Action", "DragStart", "DragEnd", "Drop", "FilesDropped", "ComponentChange", "Commit",
	}
	for i, name := range names {
		t.Run(name, func(t *testing.T) {
			reactive.CreateRoot(func(dispose func()) struct{} {
				defer dispose()
				enabled, setEnabled := CreateSignal(true)
				calls := 0
				handler := func(event *native.Event) {
					calls++
					if event.Value != "payload" || event.Type != i+1 {
						t.Fatalf("event lost its native payload: %+v", event)
					}
					event.PreventDefault()
				}
				option := propertyOption(func(props *Props) {
					reflect.ValueOf(props).Elem().FieldByName("On" + name).Set(reflect.ValueOf(handler))
				})
				node := newElement(protocol.TagView, []any{When(enabled, option)})
				host := &native.NodeHost{Nodes: map[uint32]*native.Node{node.ID: node.Node}}
				native.DispatchEvent(host, i+1, node.ID, "payload", true)
				setEnabled(false)
				native.DispatchEvent(host, i+1, node.ID, "payload", true)
				if calls != 1 || len(node.Listeners) != 0 {
					t.Fatal("inactive condition retained its event handler")
				}
				setEnabled(true)
				native.DispatchEvent(host, i+1, node.ID, "payload", true)
				if calls != 2 || len(node.Listeners) != 1 {
					t.Fatal("reactivating a handler failed or installed it twice")
				}
				return struct{}{}
			})
		})
	}
}

func TestConditionalHoverHandlerPreservesSharedListener(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		enabled, setEnabled := CreateSignal(true)
		enters := 0
		node := newElement(protocol.TagView, []any{OnMouseEnter(func(*native.Event) { enters++ }), When(enabled, OnMouseLeave(func(*native.Event) {}))})
		offset := len(node.Pending.Body())
		setEnabled(false)
		disabled := protocol.NewBatch()
		disabled.SetBoolean(node.ID, protocol.HoverListener, false)
		if bytes.Contains(node.Pending.Body()[offset:], disabled.Body()) {
			t.Fatal("removing mouseleave disabled the shared native mouseenter subscription")
		}
		native.DispatchEvent(
			&native.NodeHost{Nodes: map[uint32]*native.Node{node.ID: node.Node}},
			protocol.EventMouseEnter,
			node.ID,
			"",
			false,
		)
		if enters != 1 {
			t.Fatal("base mouseenter listener stopped working")
		}
		return struct{}{}
	})
}

func TestPrimitiveAccessibilityAndDragDeclarationsAreReactive(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		label := reactive.NewSignal("first")
		selected := reactive.NewSignal(true)
		drag := reactive.NewSignal(DragSource{ID: "first", Text: "first"})
		node := View().
			AriaLabel(label.Read).Selected(selected.Read).TabIndex(0).Role("button").
			FocusableWhenDisabled(selected.Read).
			TooltipText("Drag this item").TooltipDelay("0.2s").
			Keymap([]KeyBinding{{Keys: "cmd+s", Action: "save"}}).
			Draggable(drag.Read).DropKinds([]string{"local", "files"})
		for _, declaration := range []string{`{"cmd+s":"save"}`, `{"id":"first","text":"first"}`, `["local","files"]`} {
			if !bytes.Contains(node.Pending.Body(), []byte(declaration)) {
				t.Fatalf("missing native declaration %s", declaration)
			}
		}
		offset := len(node.Pending.Body())
		label.Write("second")
		selected.Write(false)
		drag.Write(DragSource{ID: "second"})
		expected := protocol.NewBatch()
		expected.SetString(node.ID, protocol.AccessibilityLabel, "second")
		expected.SetBoolean(node.ID, protocol.Selected, false)
		expected.SetBoolean(node.ID, protocol.FocusableWhenDisabled, false)
		expected.SetString(node.ID, protocol.Draggable, `{"id":"second"}`)
		if !bytes.Equal(node.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("updating accessibility or drag props changed unrelated properties")
		}
		return struct{}{}
	})
}

func TestInputEventDetailsKeepNativeCoordinatesAndModifiers(t *testing.T) {
	var key *KeyEventDetails
	var drop *DropEventDetails
	var wheel *WheelEventDetails
	node := View().
		OnKeyDown(func(event *native.Event) { key = KeyFromEvent(event) }).
		OnFilesDropped(func(event *native.Event) { drop = DropFromEvent(event) }).
		OnWheel(func(event *native.Event) { wheel = WheelFromEvent(event) })
	host := &native.NodeHost{Nodes: map[uint32]*native.Node{node.ID: node.Node}}
	native.DispatchEvent(
		host,
		protocol.EventKeyDown,
		node.ID,
		`{"key":"ArrowUp","text":"","meta":true,"shift":true,"repeat":true}`,
		true,
	)
	native.DispatchEvent(
		host,
		protocol.EventFilesDropped,
		node.ID,
		`{"x":21.5,"y":4,"paths":["/tmp/a b.txt"],"origin":"external","alt":true}`,
		true,
	)
	native.DispatchEvent(
		host,
		protocol.EventWheel,
		node.ID,
		`{"x":10,"y":20,"deltaX":1.5,"deltaY":-8,"precise":true,"phase":"moved","control":true}`,
		true,
	)
	if key == nil || key.Key != "ArrowUp" || !key.Meta || !key.Shift || !key.Repeat {
		t.Fatalf("key payload = %+v", key)
	}
	if drop == nil || drop.X != 21.5 || len(drop.Paths) != 1 || drop.Paths[0] != "/tmp/a b.txt" || !drop.Alt || drop.Origin != "external" {
		t.Fatalf("drop payload = %+v", drop)
	}
	if wheel == nil || wheel.DeltaY != -8 || !wheel.Precise || !wheel.Control {
		t.Fatalf("wheel payload = %+v", wheel)
	}
	for _, payload := range []string{"null", "malformed", "[]"} {
		native.DispatchEvent(host, protocol.EventKeyDown, node.ID, payload, true)
		if key != nil {
			t.Fatal("invalid native payload produced fabricated details")
		}
	}
	native.DispatchEvent(host, protocol.EventKeyDown, node.ID, `{"key":"ignored"}`, false)
	if key != nil || KeyFromEvent(nil) != nil || DropFromEvent(&native.Event{Type: protocol.EventClick}) != nil {
		t.Fatal("event decoder accepted a missing or wrong event")
	}
}
