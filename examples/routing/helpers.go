package main

import "github.com/egoist/quickgui/go/native"

import "github.com/egoist/quickgui/go/ui"

const (
	background  = "#0b1020"
	panel       = "#11182b"
	panelRaised = "#18223a"
	border      = "#26344f"
	textColor   = "#e8edf7"
	muted       = "#8e9bb4"
	blue        = "#6ea8fe"
	blueSurface = "#1d3b68"
)

var linkStyle = ui.Style().
	Display("flex").
	Height(34).
	AlignItems("center").
	PaddingLeft(12).
	PaddingRight(12).
	BorderRadius(8).
	TextColor(muted).
	Cursor("default").
	UserSelect("none").
	Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor(panelRaised) })
var activeLinkStyle = ui.Style().BackgroundColor(blueSurface).TextColor("#dceaff")
var buttonStyle = ui.Style().
	Display("flex").
	Height(34).
	AlignItems("center").
	JustifyContent("center").
	PaddingLeft(12).
	PaddingRight(12).
	BackgroundColor(panelRaised).
	TextColor(textColor).
	BorderRadius(8).
	Cursor("default").
	UserSelect("none").
	AppRegion("no-drag").
	Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor(blueSurface) }).
	DisabledStyle(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(0.35) })

func link(label, href string, end bool) *native.Node {
	return ui.Link(
		ui.LinkProps{
			Href:        href,
			End:         end,
			PartProps:   ui.PartProps{Style: linkStyle},
			ActiveStyle: &activeLinkStyle,
		},
		label,
	)
}

func button(label string, click func()) *ui.Element {
	return ui.Button().Style(buttonStyle).OnClick(click).Child(label)
}

func historyButton(label, path string, click func(), disabled func() bool) *ui.Element {
	return ui.Button().
		Child(ui.SVG().
			Value(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#e8edf7" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="` + path + `"/></svg>`).
			Width(17).
			Height(17)).
		Style(buttonStyle).
		Width(34).
		Height(30).
		PaddingLeft(0).
		PaddingRight(0).
		BorderRadius(7).
		AriaLabel(label).
		Disabled(disabled()).
		OnClick(click)

}

func page(title, description any, children ...any) *ui.Element {
	return ui.View().
		Children(

			ui.Text(
				title,
			).
				FontSize(28).
				LineHeight(36).
				FontWeight(750),
			ui.Text(
				description,
			).
				MaxWidth(620).
				TextColor(muted).
				LineHeight(21),
			children,
		).
		Display("flex").
		FlexDirection("column").
		Width("100%").
		Height("100%").
		Padding(28).
		Gap(16).
		OverflowY("auto")
}

func card(title, detail, href string) *native.Node {
	active := ui.Style().BorderColor(blue)
	return ui.Link(
		ui.LinkProps{
			Href:        href,
			ActiveStyle: &active,
			PartProps: ui.PartProps{Style: ui.Style().
				Display("flex").
				FlexDirection("column").
				Width(260).
				MinHeight(112).
				Padding(18).
				Gap(8).
				BackgroundColor(panel).
				BorderWidth(1).
				BorderColor(border).
				BorderRadius(12).
				TextColor(textColor).
				Cursor("default").
				Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
					return s.BackgroundColor(panelRaised)
				})},
		},
		func() *native.Node {
			return ui.Fragment([]*native.Node{
				ui.Text(title).FontWeight(700).Node,
				ui.Text(detail).TextColor(muted).LineHeight(19).Node,
			})
		},
	)
}
