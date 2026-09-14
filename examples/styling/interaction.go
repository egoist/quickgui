package main

import (
	"strconv"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

// Row members follow the nearest group; hints explicitly follow the outer list.
func InteractionStates() *ui.Element {
	dropStatus, setDropStatus := ui.CreateSignal("Drop the swatch or a file here.")
	rowStyle := ui.Style().
		Display("flex").
		AlignItems("center").
		Gap(10).
		Height(40).
		PaddingLeft(12).
		PaddingRight(8).
		BorderRadius(10).
		BackgroundColor("#1b2434").
		Transition("background-color 120ms").
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor("#243047") }).
		FocusWithin(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Outline("1px solid #93c5fd") })
	actionStyle := ui.Style().
		Display("flex").
		AlignItems("center").
		JustifyContent("center").
		Height(26).
		PaddingLeft(10).
		PaddingRight(10).
		BorderRadius(7).
		BackgroundColor("#2b3a5c").
		Opacity(0).
		Transition("opacity 120ms, background-color 120ms").
		UserSelect("none").
		GroupHover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(1) }).
		GroupActive(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(0.7) }).
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.BackgroundColor("#3b82f6").Transform("translate(0, -1px)")
		}).
		Active(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.BackgroundColor("#1d4ed8").Transform("scale(0.97)")
		}).
		FocusStyle(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Outline("2px solid #93c5fd") }).
		DisabledStyle(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(0.35).Cursor("not-allowed") })
	return Panel("Interaction states", func() *native.Node {
		return ui.Fragment([]*native.Node{
			ui.View().
				Child(
					func() *native.Node {
						var children []*native.Node
						for index, title := range []string{"Quarterly report", "Roadmap draft"} {
							children = append(children, ui.View().
								Children(

									ui.Text(
										title,
									).
										Flex(1).
										FontSize(13).
										TextColor(ink),
									ui.Text(
										"Cmd ",
										index+1,
									).
										FontSize(11).
										TextColor(muted).
										Opacity(0).
										Transition("opacity 120ms").
										GroupHoverNamed(
											"list",
											func(s ui.StyleBuilder) ui.StyleBuilder {
												return s.Opacity(1)
											},
										),
									ui.Button().
										Style(actionStyle).
										Child(ui.Text(
											"Rename",
										).
											FontSize(12).
											TextColor(ink)).
										AriaLabel(title+" Rename"),

									ui.Button().
										Style(actionStyle).
										Child(ui.Text(
											"Share",
										).
											FontSize(12).
											TextColor(ink)).
										Disabled(true).
										AriaLabel(title+" Share"),

									rowStyle,
								).
								Group(true).Node)

						}
						return ui.Fragment(children)
					},
				).
				Display("flex").
				FlexDirection("column").
				Gap(12).
				Group("list").Node,

			ui.View().
				Children(
					"Drag swatch",
					centered,
				).
				Height(30).
				BorderRadius(8).
				BackgroundColor("#1d4ed8").
				TextColor(ink).
				FontSize(12).
				UserSelect("none").
				Dragging(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(0.45) }).
				Draggable(ui.DragSource{
					ID:   "swatch",
					Text: "swatch",
				}).Node,

			ui.View().
				Children(

					ui.Text(
						dropStatus(),
					).
						FontSize(12).
						TextColor(muted),

					centered,
				).
				Height(54).
				PaddingLeft(12).
				PaddingRight(12).
				BorderRadius(12).
				BorderWidth(1).
				BorderStyle("dashed").
				BorderColor(panelBorder).
				Transition("background-color 120ms, border-color 120ms").
				Dragging(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(0.6) }).
				DragOver(func(s ui.StyleBuilder) ui.StyleBuilder {
					return s.BorderColor("#38bdf8").Background("#38bdf826")
				}).
				DropKinds([]string{"files", "local"}).
				OnDrop(func(event *native.Event) {
					if drop := ui.DropFromEvent(event); drop != nil {
						setDropStatus("Dropped " + drop.ID)
					}
				}).
				OnFilesDropped(func(event *native.Event) {
					if drop := ui.DropFromEvent(event); drop != nil {
						setDropStatus("Dropped " + strconv.Itoa(len(drop.Paths)) + " file(s)")
					}
				}).
				Draggable(ui.DragSource{
					ID:   "drop-zone",
					Text: "drop-zone",
				}).Node,
		})

	})
}

func StickyHeaders() *ui.Element {
	return Panel("Sticky headers", func() *ui.Element {
		return ui.View().
			Child(
				func() *native.Node {
					var children []*native.Node
					for index, section := range []string{"Inbox", "Archive", "Trash"} {
						color := "#1e3a5f"
						if index%2 != 0 {
							color = "#3c2858"
						}
						children = append(children, ui.View().
							Child(
								func() *native.Node {
									var children []*native.Node
									children = append(children, ui.View().
										Child(

											ui.Text(
												section,
											).
												FontSize(12).
												FontWeight(700).
												TextColor(ink),
										).
										Display("flex").
										AlignItems("center").
										Position("sticky").
										Top(0).
										Height(28).
										FlexShrink(0).
										PaddingLeft(12).
										BackgroundColor(color).Node)
									for row := range 6 {
										children = append(children, ui.View().
											Child(

												ui.Text(
													section,
													" row ",
													row,
												).
													FontSize(12).
													TextColor(muted),
											).
											Display("flex").
											AlignItems("center").
											Height(30).
											FlexShrink(0).
											PaddingLeft(12).Node)
									}
									return ui.Fragment(children)
								},
							).
							Display("flex").
							FlexDirection("column").
							FlexShrink(0).Node)
					}
					return ui.Fragment(children)
				},
			).
			Height(200).
			OverflowY("scroll").
			Display("flex").
			FlexDirection("column").
			BorderRadius(10).
			BorderWidth(1).
			BorderColor(panelBorder)
	})
}

func ScrollSnap() *ui.Element {
	tints := []string{"#1d4ed8", "#7c3aed", "#0f766e", "#b45309", "#be123c"}
	return Panel("Mandatory scroll snap", func() *native.Node {
		return ui.Fragment([]*native.Node{
			ui.View().
				Child(
					func() *native.Node {
						var children []*native.Node
						for index, tint := range tints {
							children = append(children, ui.View().
								Children(

									ui.Text(
										"page ",
										index,
									).
										FontSize(14).
										FontWeight(700).
										TextColor("#f8fafc"),

									centered,
								).
								Width(200).
								Height(110).
								FlexShrink(0).
								ScrollSnapAlign("start").
								ScrollSnapStop("always").
								BorderRadius(12).
								BackgroundColor(tint).Node)
						}
						return ui.Fragment(children)
					},
				).
				Height(130).
				OverflowX("scroll").
				ScrollSnapType("x mandatory").
				Display("flex").
				Gap(12).
				Padding(6).
				BorderRadius(10).
				BorderWidth(1).
				BorderColor(panelBorder).Node,
			ui.Text(
				"The core resolves the snap target at the momentum end phase and animates to it on exact deadlines, leaving the window settled.",
			).
				FontSize(12).
				TextColor(muted).Node,
		})
	})
}
