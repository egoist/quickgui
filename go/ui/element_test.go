package ui

import (
	"bytes"
	"fmt"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestFluentChildrenDeclareOneRetainedRoot(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		var view *Element
		roots := native.CollectChildren(func() *Element {
			view = View().
				Flex().
				Child(Text("hello")).
				PaddingLeft(20).
				Child(Input().Value("xxx")).
				TextAlign("center")
			return view
		})
		if len(roots) != 1 || roots[0] != view.Node || len(view.Node.Children) != 2 {
			t.Fatal("chained children escaped their parent or were declared twice")
		}
		label, input := view.Node.Children[0], view.Node.Children[1]
		if label.Children[0].Text != "hello" || input.Tag != protocol.TagInput {
			t.Fatal("fluent nesting lost text or input content")
		}
		before := view.Pending.MutationCount()
		native.SetString(view.Node, protocol.Display, "flex")
		native.SetNumber(view.Node, protocol.PaddingLeft, 20)
		native.SetString(view.Node, protocol.TextAlign, "center")
		native.SetString(input, protocol.Value, "xxx")
		if view.Pending.MutationCount() != before {
			t.Fatal("fluent declarations did not reach the native properties")
		}
		var empty *Element
		other := View().
			Child("before").
			Children([]*Element{Text("one"), nil, empty, Text("two")}).
			Children([]*native.Node{nil, Text("three").Node}).
			Children("four", []any{"five", nil}).
			Child("after")
		if got := blockText(other.Node); fmt.Sprint(got) != "[before one two three four five after]" {
			t.Fatalf("appending children lost order or nil handling: %v", got)
		}
		before = other.Pending.MutationCount()
		if other.Children().Child(nil).Children(false, empty, (*native.Node)(nil)) != other || other.Pending.MutationCount() != before {
			t.Fatal("empty children must return the same element without mutations")
		}
		return struct{}{}
	})
}

func TestFluentChildrenRejectRemovedParentsBeforeRunningFactories(t *testing.T) {
	for _, method := range []string{"Child", "Children"} {
		t.Run(method, func(t *testing.T) {
			reactive.CreateRoot(func(dispose func()) struct{} {
				defer dispose()
				view := View()
				parent := View().Child(view)
				native.RemoveNode(parent.Node, view.Node)
				before := parent.Pending.MutationCount()
				mounts := 0
				build := func() *Element { mounts++; return Text("removed") }
				var recovered any
				func() {
					defer func() { recovered = recover() }()
					if method == "Child" {
						view.Child(build)
					} else {
						view.Children(build)
					}
				}()
				if recovered == nil || mounts != 0 || parent.Pending.MutationCount() != before {
					t.Fatal("appending to a removed parent must fail before constructing children")
				}
				return struct{}{}
			})
		})
	}
}

func TestFluentReplacementDisposesBindingsAndClearsProperties(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		first, second := reactive.NewSignal("first"), reactive.NewSignal("second")
		input := Input().Value(first.Read).Placeholder("hint").Multiline(true)
		input.Value(second.Read).Placeholder("").Multiline(false)
		if len(first.Observers) != 0 || len(second.Observers) != 1 {
			t.Fatal("replacing Value retained its old binding")
		}
		before := input.Pending.MutationCount()
		first.Write("stale")
		native.ClearProperty(input.Node, protocol.Placeholder)
		native.ClearProperty(input.Node, protocol.Multiline)
		if input.Pending.MutationCount() != before {
			t.Fatal("empty or false declarations failed to clear native properties")
		}
		offset := len(input.Pending.Body())
		input.Value(nil)
		expected := protocol.NewBatch()
		expected.ClearProperty(input.ID, protocol.Value)
		if !bytes.Equal(input.Pending.Body()[offset:], expected.Body()) || len(second.Observers) != 0 {
			t.Fatal("clearing Value retained its property or subscription")
		}
		return struct{}{}
	})
}

func TestFluentBindingsStayIndependentAndChildrenKeepTheirOwners(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		width, color, text := reactive.NewSignal(100), reactive.NewSignal("#112233"), reactive.NewSignal("first")
		mounts, cleanups := 0, 0
		parent := View()
		view := View().
			Child(func() *Element {
				mounts++
				OnCleanup(func() { cleanups++ })
				return Text(text.Read)
			}).
			Width(width.Read).
			Bg(color.Read).
			PaddingLeft(20)
		native.InsertNode(parent.Node, view.Node, nil)
		child := view.Node.Children[0]
		offset := len(parent.Pending.Body())
		Batch(func() { width.Write(120); width.Write(140) })
		expected := protocol.NewBatch()
		expected.SetNumber(view.ID, protocol.Width, 140)
		if !bytes.Equal(parent.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("width updated unrelated styles or children")
		}
		view.Width(160)
		if len(width.Observers) != 0 || len(color.Observers) != 1 || len(text.Observers) != 1 {
			t.Fatal("fluent override leaked bindings or disposed child ownership")
		}
		text.Write("second")
		if view.Node.Children[0] != child || child.Children[0].Text != "second" || mounts != 1 || cleanups != 0 {
			t.Fatal("fluent configuration remounted or disposed children")
		}
		native.RemoveNode(parent.Node, view.Node)
		if len(color.Observers) != 0 || len(text.Observers) != 0 || cleanups != 1 {
			t.Fatal("removing a fluent element retained its bindings or children")
		}
		return struct{}{}
	})
}

func TestFluentConditionsRestoreStylesAndReleaseHandlers(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		selected, setSelected := CreateSignal(false)
		base, active := reactive.NewSignal(10), reactive.NewSignal(30)
		calls := 0
		view := View().
			Child(Text("kept")).
			Width(base.Read).
			When(selected, styleWidth(active.Read), OnClick(func() { calls++ })).
			PaddingLeft(20)
		child := view.Node.Children[0]
		setSelected(true)
		view.Listeners[0].Listener(&native.Event{})
		setSelected(false)
		if calls != 1 || len(view.Listeners) != 0 || len(active.Observers) != 0 || len(base.Observers) != 1 || view.Node.Children[0] != child {
			t.Fatal("conditional styles or listeners leaked across a fluent chain")
		}
		before := view.Pending.MutationCount()
		native.SetNumber(view.Node, protocol.Width, 10)
		if view.Pending.MutationCount() != before {
			t.Fatal("condition did not restore the base width")
		}
		return struct{}{}
	})
}

func TestFluentConfigurationIsImmediateInsideBatchesAndEventsBatchWrites(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal("initial")
		reads, refs := 0, 0
		label := Text(func() string { reads++; return value() })
		var input *Element
		Batch(func() {
			input = Input().
				Value("ready").
				PaddingLeft(20).
				OnInput(func(text string) {
					setValue("intermediate")
					setValue(text)
				}).
				Ref(func(node *native.Node) {
					refs++
					before := node.Pending.MutationCount()
					native.SetString(node, protocol.Value, "ready")
					native.SetNumber(node, protocol.PaddingLeft, 20)
					if node.Pending.MutationCount() != before {
						t.Fatal("Ref ran before the fluent declarations were applied")
					}
				})

		})
		input.Listeners[0].Listener(&native.Event{Value: "typed"})
		if reads != 2 || refs != 1 || label.Node.Children[0].Text != "typed" {
			t.Fatal("input callback lost text or failed to batch writes")
		}
		input.OnInput(nil)
		if len(input.Listeners) != 0 {
			t.Fatal("nil fluent handler did not remove the listener")
		}
		return struct{}{}
	})
}

func TestRustLayoutPresetsPreserveValuesAndOverrideSpecificProperties(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		view := View().Flex().FlexColReverse().Flex1().ItemsEnd().JustifyBetween().P4().Px2().M4().MlAuto().
			GridColsMinContent(3).ColSpanFull().RoundedTl(2).RoundedLg().TextLg()
		before := view.Pending.MutationCount()
		for code, value := range map[uint16]string{
			protocol.Display: "flex", protocol.FlexDirection: "column-reverse", protocol.AlignItems: "flex-end",
			protocol.JustifyContent: "space-between", protocol.MarginLeft: "auto",
			protocol.GridTemplateColumns: "repeat(3, minmax(min-content, 1fr))",
		} {
			native.SetString(view.Node, code, value)
		}
		for code, value := range map[uint16]float32{
			protocol.FlexGrow: 1, protocol.FlexShrink: 1, protocol.FlexBasis: 0,
			protocol.PaddingTop: 16, protocol.PaddingRight: 8, protocol.PaddingLeft: 8, protocol.PaddingBottom: 16,
			protocol.MarginTop: 16, protocol.GridColumnStart: 1, protocol.GridColumnEnd: -1,
			protocol.BorderRadius: 8, protocol.FontSize: 18, protocol.LineHeight: 26,
		} {
			native.SetNumber(view.Node, code, value)
		}
		native.ClearProperty(view.Node, protocol.BorderTopLeftRadius)
		if view.Pending.MutationCount() != before {
			t.Fatal("Rust layout presets diverged in value, spacing, or override behavior")
		}
		view.ColStart(5).ColEnd(8).ColSpan(2)
		before = view.Pending.MutationCount()
		native.ClearProperty(view.Node, protocol.GridColumnStart)
		native.ClearProperty(view.Node, protocol.GridColumnEnd)
		native.SetNumber(view.Node, protocol.GridColumnSpan, 2)
		if view.Pending.MutationCount() != before {
			t.Fatal("column span kept explicit lines that override native span placement")
		}
		view.GridCols(0).WFraction(-1)
		before = view.Pending.MutationCount()
		native.SetString(view.Node, protocol.GridTemplateColumns, "none")
		native.SetString(view.Node, protocol.Width, "0%")
		if view.Pending.MutationCount() != before {
			t.Fatal("zero grids or negative fractions do not match Rust")
		}
		shared := composeStyles(Style().RoundedLg(), styleHover(styleTextColor("white")))
		reused := View().
			RoundedTl(3).
			Style(StyleBuilder{style: shared}).
			Hover(func(s StyleBuilder) StyleBuilder { return s.Bg("#112233") })
		before = reused.Pending.MutationCount()
		native.ClearProperty(reused.Node, protocol.BorderTopLeftRadius)
		if reused.Pending.MutationCount() != before || shared.Hover.BackgroundColor != nil {
			t.Fatal("reusable presets lost resets or mutated shared interaction styles")
		}
		return struct{}{}
	})
}

func BenchmarkFluentDeclarations(b *testing.B) {
	for _, count := range []int{8, 32, 128} {
		b.Run(fmt.Sprint(count), func(b *testing.B) {
			b.ReportAllocs()
			for range b.N {
				reactive.CreateRoot(func(dispose func()) struct{} {
					defer dispose()
					view := View().Width(func() int { return 100 })
					for i := range count {
						view.PaddingLeft(i)
					}
					return struct{}{}
				})
			}
		})
	}
}

func TestFluentModifiersDoNotReplayUnrelatedDeclarations(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		width := reactive.NewSignal(100)
		reads := 0
		view := View().Width(func() int { reads++; return width.Read() }).OnClick(func() {})
		listener := view.Listeners[0]
		for i := 0; i < 64; i++ {
			view.PaddingLeft(i)
		}
		view.RoundedLg().TextLg().Style(Style().Height(40).Bg("#112233"))
		if reads != 1 || len(width.Observers) != 1 || view.Listeners[0] != listener {
			t.Fatal("ordinary modifiers reread or replaced an unrelated binding or handler")
		}
		before := view.Pending.MutationCount()
		width.Write(120)
		if reads != 2 || view.Pending.MutationCount() != before+1 {
			t.Fatal("retained width binding did not produce exactly one update")
		}
		return struct{}{}
	})
}
