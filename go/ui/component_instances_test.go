package ui

import (
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func clickComponent(node *native.Node) {
	for _, listener := range node.Listeners {
		if listener.Type == protocol.EventClick {
			listener.Listener(&native.Event{Type: protocol.EventClick, Target: node})
			return
		}
	}
	panic("missing click listener")
}

func TestComponentInstancesComposeWithoutCallbacksAndKeepStateSeparate(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		changes := []bool{}
		first := NewPopover().OnOpenChange(func(open bool, _ PopoverOpenChangeDetails) { changes = append(changes, open) })
		secondChanges := 0
		second := NewPopover().OnOpenChange(func(bool, PopoverOpenChangeDetails) { secondChanges++ })
		trigger := first.Trigger().Child("First").Px4()
		other := second.Trigger().Child("Second")
		root := View().Children(first.Root().Child(trigger), second.Root().Child(other))
		var mounted []*native.Node
		for _, child := range root.Node.Children {
			if child.Tag != protocol.TagSentinel {
				mounted = append(mounted, child)
			}
		}
		if len(mounted) != 2 || mounted[0] != trigger.NativeNode() || mounted[1] != other.NativeNode() {
			t.Fatal("compound part did not mount directly")
		}
		id := trigger.ID
		clickComponent(trigger.NativeNode())
		clickComponent(trigger.NativeNode())
		if !reflect.DeepEqual(changes, []bool{true, false}) || secondChanges != 0 || trigger.ID != id {
			t.Fatal("instance state or node identity changed")
		}
		clickComponent(other.NativeNode())
		if secondChanges != 1 {
			t.Fatal("second instance lost its default action")
		}
		return struct{}{}
	})
}

func TestTransparentComponentRootsDoNotAddLayoutViews(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		content := View().SizeFull().Child("window content")
		provider := NewToast().Provider().Child(content)
		parent := View().Child(provider)
		root := provider.NativeNode()
		if root.Tag != protocol.TagSentinel || len(root.Group) != 1 || root.Group[0] != content.Node {
			t.Fatal("a transparent provider inserted a layout node around its content")
		}
		if len(parent.Node.Children) != 2 || parent.Node.Children[0] != content.Node || parent.Node.Children[1] != root {
			t.Fatal("transparent provider content did not mount directly in its parent")
		}
		return struct{}{}
	})
}

func TestDetachedRegionsRetireNestedFragmentGroups(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		visible, setVisible := CreateSignal(true)
		var fragment, child *native.Node
		Show(visible, func() *native.Node {
			child = View().Child("nested").Node
			fragment = Fragment(child)
			return fragment
		})
		setVisible(false)
		if fragment == nil || child == nil || !fragment.Removed || !child.Removed {
			t.Fatal("clearing a detached region leaked a nested fragment group")
		}
		return struct{}{}
	})
}

func TestCompoundFluentHandlersPreserveDefaultActionsAndCancellation(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		changes, clicks := 0, 0
		control := NewSwitch().OnCheckedChange(func(bool, *native.Event) { changes++ })
		button := control.Root().OnClick(func() { clicks++ }).Child(control.Thumb())
		root := View().Child(button)
		clickComponent(button.NativeNode())
		if changes != 1 || clicks != 1 {
			t.Fatalf("initial handler: changes=%d clicks=%d", changes, clicks)
		}
		button.OnClickEvent(func(event *native.Event) { event.PreventDefault() })
		clickComponent(button.NativeNode())
		if changes != 1 || clicks != 1 {
			t.Fatal("PreventDefault did not cancel the component action")
		}
		button.OnClick(nil)
		clickComponent(button.NativeNode())
		if changes != 2 {
			t.Fatal("clearing a user handler removed the component action")
		}
		button.Flex(1).When(true, Style().Bg("#ff0000"))
		clickComponent(button.NativeNode())
		if changes != 3 {
			t.Fatal("style reconciliation lost or duplicated the component action")
		}
		native.RemoveNode(root.Node, button.NativeNode())
		return struct{}{}
	})
}

func TestCompoundBindingsAndDeferredChildrenDisposeWithRoot(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		open, setOpen := CreateSignal(false)
		reads, builds, cleanups := 0, 0, 0
		popover := NewPopover().Open(func() bool { reads++; return open() })
		trigger := popover.Trigger().Child("Open")
		root := popover.Root().
			Children(
				trigger,
				func() *Element {
					builds++
					OnCleanup(func() { cleanups++ })
					return popover.Positioner().
						Child(popover.Popup().Child(popover.Title().Child("Title")))
				},
			)
		parent := View().Child(root)
		node := trigger.NativeNode()
		setOpen(true)
		if builds != 1 || node != trigger.NativeNode() {
			t.Fatal("a state update reconstructed compound children")
		}
		native.RemoveNode(parent.Node, root.NativeNode())
		before := reads
		setOpen(false)
		if cleanups != 1 || reads != before || !popover.instance.owner.Disposed {
			t.Fatal("component ownership survived unmount")
		}
		return struct{}{}
	})
}

func TestCompoundNestedItemContextAndControlledValues(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		selected, setSelected := CreateSignal("account")
		tabs := NewTabs().Value(selected).OnValueChange(func(value string, _ *native.Event) { setSelected(value) })
		profile := tabs.Tab("profile").Child(tabs.Indicator()).Child("Profile")
		root := tabs.Root().
			Children(
				tabs.List().
					Children(
						tabs.Tab("account").Child("Account"),
						profile,
					),
				tabs.Panel("profile").Child("Profile content"),
			)
		node := root.NativeNode()
		before := profile.ID
		clickComponent(profile.NativeNode())
		if selected() != "profile" || before != profile.ID || !reflect.DeepEqual(blockText(node), []string{"Account", "Profile", "Profile content"}) {
			t.Fatalf("selection=%q IDs=%d/%d children=%d", selected(), before, profile.ID, len(node.Children))
		}
		return struct{}{}
	})
}

func TestNestedInstancesKeepExplicitPartOwnership(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		outerChanges, innerChanges := 0, 0
		outer := NewPopover().OnOpenChange(func(bool, PopoverOpenChangeDetails) { outerChanges++ })
		inner := NewPopover().OnOpenChange(func(bool, PopoverOpenChangeDetails) { innerChanges++ })
		outerTrigger := outer.Trigger().Child("Outer")
		innerTrigger := inner.Trigger().Child("Inner")
		root := outer.Root().Child(inner.Root().Children(outerTrigger, innerTrigger))
		root.NativeNode()
		clickComponent(outerTrigger.NativeNode())
		if outerChanges != 1 || innerChanges != 0 {
			t.Fatal("a nested instance captured another instance's part")
		}
		clickComponent(innerTrigger.NativeNode())
		if outerChanges != 1 || innerChanges != 1 {
			t.Fatal("nested instance lost its own part")
		}
		return struct{}{}
	})
}

func TestPendingCompoundOverridesClearHandlers(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		clicks, changes := 0, 0
		control := NewSwitch().OnCheckedChange(func(bool, *native.Event) { changes++ })
		button := control.Root().OnClick(func() { clicks++ }).OnClick(nil).Child("some text")
		clickComponent(button.NativeNode())
		if clicks != 0 || changes != 1 || button.Node.Children[0].Text != "some text" {
			t.Fatal("pending overrides or direct text were lost")
		}
		return struct{}{}
	})
}

func TestPopoverContentReopensWithFluentStylesAndDirectText(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		open, setOpen := CreateSignal(false)
		popover := NewPopover().Open(open)
		var body *native.Node
		content := popover.Content().Width(180).Child("some text").Ref(func(node *native.Node) { body = node })
		root := popover.Root().Children(popover.Trigger().Child("Open"), content)
		root.NativeNode()
		if body != nil {
			t.Fatal("closed content mounted eagerly")
		}
		setOpen(true)
		if body == nil || !reflect.DeepEqual(blockText(body), []string{"some text"}) {
			t.Fatal("content did not mount direct text")
		}
		before := root.Pending.MutationCount()
		native.SetNumber(body, protocol.Width, 180)
		if root.Pending.MutationCount() != before {
			t.Fatal("fluent width was applied to the placeholder rather than the content")
		}
		first := body
		for width := 180; width < 400; width++ {
			content.Width(width)
		}
		content.Width(220)
		if len(content.content.declaration.declarations) != 1 {
			t.Fatal("content property updates retained declaration history")
		}
		setOpen(false)
		if !first.Removed {
			t.Fatal("closing content did not dispose its node")
		}
		setOpen(true)
		if body == first || !reflect.DeepEqual(blockText(body), []string{"some text"}) {
			t.Fatal("reopening lost its text or reused a removed node")
		}
		before = root.Pending.MutationCount()
		native.SetNumber(body, protocol.Width, 220)
		if root.Pending.MutationCount() != before {
			t.Fatal("reopening lost fluent declarations")
		}
		return struct{}{}
	})
}

func TestNumericInstanceSettingsAcceptGoIntegerValuesAndAccessors(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		count, setCount := CreateSignal(2)
		reads := 0
		table := NewTable().Columns([]TableColumnDeclaration{{ID: "name", Track: "1fr"}}).RowCount(func() int { reads++; return count() })
		root := table.Root()
		parent := View().Child(root)
		before := reads
		setCount(3)
		if reads <= before {
			t.Fatal("integer accessor did not remain reactive")
		}
		native.RemoveNode(parent.Node, root.NativeNode())
		before = reads
		setCount(4)
		if reads != before {
			t.Fatal("integer accessor survived unmount")
		}
		NewTable().
			Columns([]TableColumnDeclaration{{ID: "name", Track: "1fr"}}).
			RowCount(2).
			Root().
			NativeNode()
		NewProgress().SetValue(1).Root().NativeNode()
		return struct{}{}
	})
}
