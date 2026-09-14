package ui

import (
	"bytes"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestFluentStyleReusePreservesOverridesAndLayoutResets(t *testing.T) {
	base := Style().RoundedLg().PaddingStart(12).GridCols(3)
	variant := Style().
		Merge(base).
		PaddingInlineStart(0).
		ColSpan(2).
		When(
			false,
			func(s StyleBuilder) StyleBuilder {
				t.Fatal("an inactive style callback ran")
				return s
			},
		)
	view := View().RoundedTl(3).ColStart(5).ColEnd(8).Style(variant)
	before := view.Pending.MutationCount()
	native.SetNumber(view.Node, protocol.BorderRadius, 8)
	native.ClearProperty(view.Node, protocol.BorderTopLeftRadius)
	native.SetNumber(view.Node, protocol.PaddingStart, 0)
	native.SetNumber(view.Node, protocol.GridColumnSpan, 2)
	native.ClearProperty(view.Node, protocol.GridColumnStart)
	native.ClearProperty(view.Node, protocol.GridColumnEnd)
	if view.Pending.MutationCount() != before {
		t.Fatal("reusable layout helpers lost alias precedence, explicit zero, or resets")
	}
	other := View().Style(base)
	before = other.Pending.MutationCount()
	native.SetNumber(other.Node, protocol.PaddingStart, 12)
	native.ClearProperty(other.Node, protocol.GridColumnSpan)
	if other.Pending.MutationCount() != before {
		t.Fatal("deriving a style mutated the shared base")
	}
}

func TestFluentStyleInteractionCallbacksMergeWithoutMutatingSharedValues(t *testing.T) {
	for name, state := range map[string]func(StyleBuilder, func(StyleBuilder) StyleBuilder) StyleBuilder{
		"Hover": StyleBuilder.Hover, "Active": StyleBuilder.Active, "Focus": StyleBuilder.FocusStyle,
		"Disabled": StyleBuilder.DisabledStyle, "Selected": StyleBuilder.SelectedStyle,
		"Invalid": StyleBuilder.InvalidStyle, "Dragging": StyleBuilder.Dragging,
		"DragOver": StyleBuilder.DragOver, "FocusWithin": StyleBuilder.FocusWithin,
	} {
		t.Run(name, func(t *testing.T) {
			base := state(Style(), func(s StyleBuilder) StyleBuilder { return s.Bg("#112233").TextColor("white") })
			override := state(Style(), func(s StyleBuilder) StyleBuilder { return s.Bg("#445566").Opacity(0) })
			merged := Style().Merge(base, override)
			read := func(style StyleBuilder) *styleData {
				return reflect.ValueOf(style.style).FieldByName(name).Interface().(*styleData)
			}
			if got := read(merged); got.BackgroundColor != "#445566" || got.TextColor != "white" || got.Opacity != 0 {
				t.Fatalf("interaction style did not merge declarations: %+v", got)
			}
			if got := read(base); got.BackgroundColor != "#112233" || got.Opacity != nil {
				t.Fatal("merging an interaction style mutated its base")
			}

		})
	}
	base := Style().
		GroupHoverNamed(
			"card",
			func(s StyleBuilder) StyleBuilder { return s.Opacity(.5) },
		)
	variant := base.GroupHoverNamed(
		"toolbar",
		func(s StyleBuilder) StyleBuilder { return s.Opacity(1) },
	)
	if got := groupHoverRules(variant.style); len(got) != 2 || got[0].name != "card" || got[1].name != "toolbar" {
		t.Fatal("fluent group rules lost their names or declaration order")
	}
	if len(groupHoverRules(base.style)) != 1 {
		t.Fatal("extending group rules changed a shared style")
	}
}

func TestFluentStyleSharedAccessorsUpdateIndependentlyAndDispose(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		width, color := reactive.NewSignal(20), reactive.NewSignal("#112233")
		shared := Style().Width(width.Read).Bg(color.Read)
		if len(width.Observers) != 0 || len(color.Observers) != 0 {
			t.Fatal("building a reusable style evaluated its accessors")
		}
		parent := View()
		first := View().Child(Text("kept")).Style(shared)
		second := View().Style(shared)
		part := View()
		applyPart(part.Node, PartProps{Style: shared})
		for _, node := range []*Element{first, second, part} {
			native.InsertNode(parent.Node, node.Node, nil)
		}
		child := first.Node.Children[0]
		shared = shared.Width(999)
		offset := len(parent.Pending.Body())
		width.Write(30)
		expected := protocol.NewBatch()
		for _, node := range []*Element{first, second, part} {
			expected.SetNumber(node.ID, protocol.Width, 30)
		}
		if !bytes.Equal(parent.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("a shared accessor changed unrelated properties or lost its applied snapshot")
		}
		if first.Node.Children[0] != child || len(width.Observers) != 3 || len(color.Observers) != 3 {
			t.Fatal("reusing a style rebuilt children or shared a node's subscriptions")
		}
		first.Style(Style().Width(0))
		if len(width.Observers) != 2 || len(color.Observers) != 3 {
			t.Fatal("a later fluent style failed to replace only its width binding")
		}
		for _, node := range []*Element{first, second, part} {
			native.RemoveNode(parent.Node, node.Node)
		}
		if len(width.Observers) != 0 || len(color.Observers) != 0 {
			t.Fatal("removing styled elements retained their subscriptions")
		}
		return struct{}{}
	})
}

func TestFluentStylePartAccessorRestoresConditionalFallback(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		selected, setSelected := CreateSignal(false)
		color := reactive.NewSignal("#112233")
		base := Style().Bg(color.Read).Padding(12)
		parent := View()
		part := createViewPart(PartProps{Style: func() StyleBuilder {
			return base.When(
				selected(),
				func(s StyleBuilder) StyleBuilder { return s.Bg("#445566").Opacity(.5) },
			)
		}})
		native.InsertNode(parent.Node, part, nil)
		setSelected(true)
		if len(color.Observers) != 0 {
			t.Fatal("an overridden style accessor remained subscribed")
		}
		color.Write("#abcdef")
		offset := len(parent.Pending.Body())
		setSelected(false)
		expected := protocol.NewBatch()
		expected.SetColor(part.ID, protocol.BackgroundColor, native.ParseColor("#abcdef"))
		expected.ClearProperty(part.ID, protocol.Opacity)
		if !bytes.Equal(parent.Pending.Body()[offset:], expected.Body()) {
			t.Fatal("a conditional builder did not restore its latest fallback and clear omitted properties")
		}
		native.RemoveNode(parent.Node, part)
		if len(color.Observers) != 0 {
			t.Fatal("a removed conditional style retained its accessor")
		}
		return struct{}{}
	})
}
