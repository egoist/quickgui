package native

import (
	"testing"

	"github.com/egoist/quickgui/packages/go/protocol"
)

func TestDetachedInsertRecordsCreateThenInsert(t *testing.T) {
	ResetTreeStateForTests()
	parent := CreateElement(protocol.TagView)
	child := CreateText("hi")
	InsertNode(parent, child, nil)
	if len(parent.Children) != 1 || parent.Children[0] != child {
		t.Fatal("child not attached")
	}
	if child.Pending != nil {
		t.Fatal("pending should be spliced into the parent")
	}
	if parent.Pending.MutationCount() != 3 {
		t.Fatalf("mutations %d", parent.Pending.MutationCount())
	}
	body := parent.Pending.Body()
	if body[0] != 1 || body[6] != 2 {
		t.Fatalf("expected create element then create text, got %v", body)
	}
}

func TestReplaceTextUpdatesPending(t *testing.T) {
	ResetTreeStateForTests()
	node := CreateText("a")
	ReplaceText(node, "b")
	if node.Text != "b" {
		t.Fatal(node.Text)
	}
	if node.Pending.MutationCount() != 2 {
		t.Fatalf("count %d", node.Pending.MutationCount())
	}
}
