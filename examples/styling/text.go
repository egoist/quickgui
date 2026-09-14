package main

import "github.com/egoist/quickgui/go/native"

import "github.com/egoist/quickgui/go/ui"

// The core resolves start and end against the inherited text direction.
func TextAlignment() *ui.Element {
	return Panel("Text alignment", func() *native.Node {
		var children []*native.Node
		for _, align := range []string{"left", "center", "right", "justify", "start", "end"} {
			children = append(children, ui.View().
				Child(

					ui.Text(
						"textAlign: \""+align+"\" — the core resolves start and end against the inherited direction.",
					).
						Width("100%").
						TextAlign(align).
						FontSize(13).
						TextColor(ink),
				).
				Width("100%").
				Padding(8).
				BorderRadius(8).
				BackgroundColor("#1b2434").Node)
		}
		return ui.Fragment(children)
	})
}

func TextStyling() *ui.Element {
	return Panel("Extended text styling", func() *native.Node {
		return ui.Fragment([]*native.Node{
			ui.Text(
				"letterSpacing 2",
			).
				FontSize(20).
				FontWeight(700).
				LetterSpacing(2).
				TextColor(ink).Node,
			ui.Text(
				"wordSpacing 8 pushes every space apart",
			).
				FontSize(14).
				WordSpacing(8).
				TextColor(ink).Node,
			ui.Text(
				"textTransform capitalize keeps selection on the original text",
			).
				FontSize(14).
				TextTransform("capitalize").
				TextColor(ink).Node,
			ui.Text(
				"textShadow",
			).
				FontSize(24).
				FontWeight(700).
				TextColor("#f8fafc").
				TextShadow("0 3px 10px #38bdf8aa").Node,
			ui.Text(
				"wavy underline in its own color",
			).
				FontSize(14).
				TextColor(ink).
				TextDecoration("underline").
				TextDecorationColor("#f97316").
				TextDecorationStyle("wavy").
				TextDecorationThickness(2).Node,
			ui.Text(
				"line-through and overline together",
			).
				FontSize(14).
				TextColor(ink).
				TextDecoration("line-through overline").
				TextDecorationColor("#f43f5e").Node,
			ui.View().
				Child(

					ui.Text(
						"wordBreak break-all with overflowWrap anywhere: supercalifragilisticexpialidocious",
					).
						Width("100%").
						FontSize(13).
						TextColor(muted).
						WordBreak("break-all").
						OverflowWrap("anywhere").
						Hyphens("manual"),
				).
				Width(200).
				Padding(8).
				BorderRadius(8).
				BackgroundColor("#1b2434").Node,
		})
	})
}

func Direction() *ui.Element {
	return Panel("Right to left", func() *native.Node {
		return ui.Fragment([]*native.Node{
			ui.View().
				Children(

					ui.View().Width(28).Height(20).BorderRadius(6).BackgroundColor("#38bdf8"),
					ui.View().Width(20).Height(20).BorderRadius(6).BackgroundColor("#334155"),
					ui.Text(
						"مرحبا بالعالم — hello",
					).
						FontSize(13).
						TextColor(ink).
						TextAlign("start").
						TextDirection("rtl"),
				).
				Direction("rtl").
				Display("flex").
				Gap(8).
				AlignItems("center").
				Padding(10).
				PaddingStart(20).
				BorderRadius(10).
				BorderStartWidth(3).
				BorderColor("#38bdf8").
				BackgroundColor("#1b2434").Node,
			ui.Text(
				"paddingStart and borderStartWidth resolve to the right edge inside this subtree.",
			).
				FontSize(12).
				TextColor(muted).Node,
		})
	})
}
