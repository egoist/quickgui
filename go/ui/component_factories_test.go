package ui

import (
	"fmt"
	"reflect"
	"strings"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

func TestVoidConstructionCallbacksAreUnsupported(t *testing.T) {
	for name, mount := range map[string]func(){
		"view":          func() { View().Child(func() {}) },
		"hidden branch": func() { Show(false, func() {}) },
		"rows":          func() { For(func() []int { return []int{1} }, func(int) {}, nil, nil) },
	} {
		t.Run(name, func(t *testing.T) {
			defer func() {
				failure := recover()
				if failure == nil || !strings.Contains(fmt.Sprint(failure), "must return") {
					t.Fatalf("void callback was accepted: %v", failure)
				}
			}()
			mount()
		})
	}
}

func TestReturnedChildFactoriesRetainContextAndCleanupWithoutWrapperNodes(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal("one")
		context := reactive.CreateContext("outside")
		builds, cleaned := 0, 0
		var child *Element
		root := reactive.Provide(context, "inside", func() *Element {
			return View().
				Child(func() *Element {
					builds++
					OnCleanup(func() { cleaned++ })
					child = Text(context.Use(), value)
					return child
				})
		})
		if len(root.Node.Children) != 1 || root.Node.Children[0] != child.Node {
			t.Fatal("returning a child added a wrapper node")
		}
		setValue("two")
		if builds != 1 || !reflect.DeepEqual(blockText(root.Node), []string{"inside", "two"}) {
			t.Fatal("returned child lost context or remounted")
		}
		parent := View().Child(root)
		native.RemoveNode(parent.Node, root.Node)
		if cleaned != 1 {
			t.Fatal("child factory cleanup did not follow its parent")
		}
		return struct{}{}
	})
}

func TestReturnedConditionalAndKeyedComponentsKeepIdentity(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		type item struct {
			ID   int
			Name string
		}
		items, setItems := CreateSignal([]item{{1, "one"}, {2, "two"}})
		visible, setVisible := CreateSignal(true)
		mounted, cleaned := 0, 0
		refs := map[int]*native.Node{}
		root := View().
			Child(Show(visible, func() *Element {
				return View().
					Child(KeyedFor(items, func(value item) any { return value.ID }, func(read func() item) *Element {
						mounted++
						OnCleanup(func() { cleaned++ })
						node := Text(func() string { return read().Name })
						refs[read().ID] = node.Node
						return node
					}, nil))
			}))
		first := refs[1]
		setItems([]item{{2, "second"}, {1, "first"}})
		if mounted != 2 || refs[1] != first || !reflect.DeepEqual(blockText(root.Node), []string{"second", "first"}) {
			t.Fatal("returning keyed rows lost identity")
		}
		setVisible(false)
		if cleaned != 2 {
			t.Fatal("returned lazy rows leaked their owners")
		}
		setVisible(true)
		if mounted != 4 {
			t.Fatal("the returned branch did not remount")
		}
		return struct{}{}
	})
}

func TestReturnedProviderChildRemainsTheMountedRoot(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		var child *Element
		roots := native.CollectChildren(func() *native.Node {
			return (toastAPI{}).Provider(ToastProviderProps{}, func() *Element {
				child = Text("provider child")
				return child
			})
		})
		if len(roots) != 1 || roots[0] != child.Node {
			t.Fatal("a returned provider child disappeared from its enclosing component")
		}
		return struct{}{}
	})
}
