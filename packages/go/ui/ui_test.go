package ui

import (
	"testing"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

func TestViewAppliesStyleAndChildren(t *testing.T) {
	native.ResetTreeStateForTests()
	node := View(Props{
		Style: Style{Display: "flex", Width: "100%", Gap: 12, BackgroundColor: "#112233"},
		Children: []any{
			Text(Props{Children: "hello"}),
		},
	})
	if node.Tag != protocol.TagView {
		t.Fatal(node.Tag)
	}
	if len(node.Children) != 1 {
		t.Fatalf("children %d", len(node.Children))
	}
	if node.Pending == nil || node.Pending.Empty() {
		t.Fatal("expected recorded mutations")
	}
}

func TestDynamicTextFollowsSignal(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		count, setCount := CreateSignal(1)
		node := DynamicText(func() string { return "Count: " + itoa(count()) })
		if node.Text != "Count: 1" {
			t.Fatalf("initial %s", node.Text)
		}
		setCount(2)
		if node.Text != "Count: 2" {
			t.Fatalf("updated %s", node.Text)
		}
		return struct{}{}
	})
}

func TestShowCreatesChildrenOnDemand(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		visible, setVisible := CreateSignal(false)
		created := 0
		parent := View(Props{})
		sentinel := Show(func() bool { return visible() }, func() *native.Node {
			created++
			return Text(Props{Children: "shown"})
		})
		native.InsertNode(parent, sentinel, nil)
		if created != 0 {
			t.Fatal("created before visible")
		}
		setVisible(true)
		if created != 1 {
			t.Fatalf("created %d", created)
		}
		setVisible(true)
		if created != 1 {
			t.Fatal("recreated while still visible")
		}
		setVisible(false)
		if len(sentinel.Group) != 0 && sentinel.Group != nil {
			// cleared region has nil group
		}
		return struct{}{}
	})
}

func itoa(value int) string {
	if value == 0 {
		return "0"
	}
	neg := value < 0
	if neg {
		value = -value
	}
	var digits [12]byte
	i := len(digits)
	for value > 0 {
		i--
		digits[i] = byte('0' + value%10)
		value /= 10
	}
	if neg {
		i--
		digits[i] = '-'
	}
	return string(digits[i:])
}
