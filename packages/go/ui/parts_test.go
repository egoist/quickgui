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
