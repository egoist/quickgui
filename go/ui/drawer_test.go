package ui

import (
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

func TestDrawerDeclaresCorePartsAndToggles(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		changes := []bool{}
		snap := 1
		var trigger, closeControl *native.Node
		root := (drawerAPI{}).Root(DrawerRootProps{
			Modal:          DrawerModalityTrapFocus,
			SwipeDirection: "left",
			SnapPoints:     []float64{0.4, 0.9},
			SnapPoint:      &snap,
			OnOpenChange: func(next bool, _ *native.Event) {
				changes = append(changes, next)
			},
			PartProps: PartProps{Children: func() *native.Node {
				trigger = (drawerAPI{}).Trigger(PartProps{})
				return Fragment(
					trigger,
					(drawerAPI{}).Portal(PartProps{Children: func() *native.Node {
						return (drawerAPI{}).Popup(PartProps{Children: func() *native.Node {
							closeControl = (drawerAPI{}).Close(PartProps{})
							return Fragment(
								(drawerAPI{}).SwipeArea(PartProps{}),
								(drawerAPI{}).Content(PartProps{Children: func() *native.Node {
									return Fragment(
										(drawerAPI{}).Title(PartProps{}),
										(drawerAPI{}).Description(PartProps{}),
									)
								}}),
								closeControl,
							)
						}})
					}}),
				)
			}},
		})
		if root.Pending == nil || root.Pending.Empty() {
			t.Fatal("expected drawer mutations")
		}
		if trigger == nil || closeControl == nil {
			t.Fatal("drawer parts did not mount")
		}
		clickComponent(trigger)
		clickComponent(closeControl)
		if !reflect.DeepEqual(changes, []bool{true, false}) {
			t.Fatalf("changes %v", changes)
		}
		return struct{}{}
	})
}

func TestDrawerControlledOpenSkipsUncontrolledWrites(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		changes := 0
		var trigger *native.Node
		(drawerAPI{}).Root(DrawerRootProps{
			Open: func() bool { return false },
			OnOpenChange: func(bool, *native.Event) {
				changes++
			},
			PartProps: PartProps{Children: func() *native.Node {
				trigger = (drawerAPI{}).Trigger(PartProps{})
				return trigger
			}},
		})
		clickComponent(trigger)
		if changes != 1 {
			t.Fatalf("controlled trigger emitted %d changes", changes)
		}
		return struct{}{}
	})
}

func TestDrawerReportsSwipeSnapAndOpenChanges(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		var swipes []DrawerSwipeState
		snaps := []int{}
		opens := []bool{}
		var state *drawerState
		var readSwipe func() DrawerSwipeState
		(drawerAPI{}).Root(DrawerRootProps{
			OnOpenChange: func(next bool, _ *native.Event) {
				opens = append(opens, next)
			},
			OnSnapPointChange: func(index int, _ *native.Event) {
				snaps = append(snaps, index)
			},
			OnSwipeChange: func(next DrawerSwipeState, _ *native.Event) {
				swipes = append(swipes, next)
			},
			PartProps: PartProps{Children: func() *native.Node {
				state = drawerContext.Use()
				readSwipe = UseDrawerSwipe()
				return Fragment(nil)
			}},
		})
		if state == nil || readSwipe == nil {
			t.Fatal("drawer context or swipe reader did not resolve inside Drawer.Root")
		}
		swiping, open := true, true
		offset, snap := 12.5, 1
		state.applyChange(&ComponentChangeDetails{
			Open:        &open,
			SnapPoint:   &snap,
			Swiping:     &swiping,
			SwipeOffset: &offset,
		}, &native.Event{})
		if !reflect.DeepEqual(swipes, []DrawerSwipeState{{Swiping: true, SwipeOffset: 12.5}}) {
			t.Fatalf("swipes %#v", swipes)
		}
		if got := readSwipe(); !got.Swiping || got.SwipeOffset != 12.5 {
			t.Fatalf("live swipe %#v", got)
		}
		if !reflect.DeepEqual(snaps, []int{1}) || !reflect.DeepEqual(opens, []bool{true}) {
			t.Fatalf("snaps %v opens %v", snaps, opens)
		}
		// A settled gesture clears the offset and leaves the snap point alone.
		swiping = false
		offset = 0
		state.applyChange(&ComponentChangeDetails{Swiping: &swiping, SwipeOffset: &offset}, &native.Event{})
		if got := readSwipe(); got.Swiping || got.SwipeOffset != 0 {
			t.Fatalf("settled swipe %#v", got)
		}
		if len(snaps) != 1 || len(opens) != 1 {
			t.Fatal("settled swipe repeated snap or open callbacks")
		}
		return struct{}{}
	})
}

func TestDrawerInstancesKeepStateSeparate(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		changes := []bool{}
		secondChanges := 0
		first := NewDrawer().OnOpenChange(func(open bool, _ *native.Event) { changes = append(changes, open) })
		second := NewDrawer().OnOpenChange(func(bool, *native.Event) { secondChanges++ })
		trigger := first.Trigger().Child("First")
		other := second.Trigger().Child("Second")
		root := View().Children(first.Root().Child(trigger), second.Root().Child(other))
		root.NativeNode()
		clickComponent(trigger.NativeNode())
		if !reflect.DeepEqual(changes, []bool{true}) || secondChanges != 0 {
			t.Fatal("first instance lost its open change")
		}
		clickComponent(other.NativeNode())
		if secondChanges != 1 || len(changes) != 1 {
			t.Fatal("second instance shared the first instance's state")
		}
		return struct{}{}
	})
}
