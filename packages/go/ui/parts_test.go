package ui

import (
	"testing"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

func TestSetPartWritesScopeAndName(t *testing.T) {
	native.ResetTreeStateForTests()
	node := createViewPart(PartProps{})
	setPart(node, protocol.PartSlider, "qg-slider-1", "0")
	if node.Pending == nil || node.Pending.MutationCount() < 4 {
		t.Fatalf("mutations %v", node.Pending)
	}
}

func TestFinishPartMountsChildrenAndRef(t *testing.T) {
	native.ResetTreeStateForTests()
	var ref *native.Node
	reactive.CreateRoot(func(dispose func()) struct{} {
		node := finishPart(createButtonPart(PartProps{}), PartProps{
			Children: func() *native.Node { return Text(Props{Children: "ok"}) },
			Ref:      func(n *native.Node) { ref = n },
		})
		if ref != node || len(node.Children) != 1 {
			t.Fatalf("ref=%v children=%d", ref, len(node.Children))
		}
		dispose()
		return struct{}{}
	})
}

func TestPartPropsAcceptPrimitiveBoolPointers(t *testing.T) {
	native.ResetTreeStateForTests()
	focus := false
	disabled := true
	label := "Stage file"
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		node := Checkbox.Root(CheckboxProps{
			PartProps: PartProps{
				FocusOnPointer: &focus,
				Disabled:       &disabled,
				AriaLabel:      &label,
			},
			DefaultChecked: false,
		})
		if node.Pending == nil || node.Pending.Empty() {
			t.Fatal("expected checkbox mutations")
		}
		return struct{}{}
	})
}

func TestResolveHelpersAcceptPointers(t *testing.T) {
	flag := true
	if got := resolveBoolean(&flag); got == nil || !*got {
		t.Fatalf("bool pointer %#v", got)
	}
	if resolveBoolean((*bool)(nil)) != nil {
		t.Fatal("nil bool pointer should be absent")
	}
	text := "label"
	if got := resolveString(&text); got == nil || *got != "label" {
		t.Fatalf("string pointer %#v", got)
	}
	number := 12.0
	if got := resolveNumber(&number); got == nil || *got != 12 {
		t.Fatalf("number pointer %#v", got)
	}
	if resolveNumber((*float64)(nil)) != nil {
		t.Fatal("nil number pointer should be absent")
	}
}

func TestForwardClickRespectsPreventDefault(t *testing.T) {
	activated := false
	handler := forwardClick(func(event *native.Event) {
		event.PreventDefault()
	}, func(*native.Event) { activated = true })
	handler(&native.Event{})
	if activated {
		t.Fatal("activate ran after preventDefault")
	}
	forwardClick(nil, func(*native.Event) { activated = true })(&native.Event{})
	if !activated {
		t.Fatal("activate skipped")
	}
}
