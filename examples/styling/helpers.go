package main

import (
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

const (
	ink             = "#e6ecf7"
	muted           = "#94a3b8"
	panelBackground = "#141a26"
	panelBorder     = "#27324a"
)

var warmPalette = reactive.CreateContext[func() bool](func() bool { return false })

var panelStyle = ui.Style().
	Display("flex").
	FlexDirection("column").
	MinWidth(0).
	Gap(12).
	Padding(18).
	BackgroundColor(panelBackground).
	BorderColor(panelBorder).
	BorderWidth(1).
	BorderRadius(14)
var captionStyle = ui.Style().
	TextColor(muted).
	FontSize(11).
	LetterSpacing(0.8).
	TextTransform("uppercase").
	FontWeight(700)
var centered = ui.Style().
	Display("flex").
	AlignItems("center").
	JustifyContent("center")

func Panel(title string, children ui.Component) *ui.Element {
	warm := warmPalette.Use()
	return ui.View().
		Children(

			ui.Text(title).Style(captionStyle),
			children,
		).
		Style(panelStyle).
		When(
			warm,
			ui.Style().BackgroundColor("#30231c"),
			ui.Style().BorderColor("#594338"),
		)

}

func swatch(label string, styles ...ui.StyleBuilder) *ui.Element {
	style := centered
	for _, next := range styles {
		style = style.Merge(next)
	}
	return ui.View().Style(style).Child(ui.Text(label).FontSize(12).TextColor(ink))
}

func ptr[T any](value T) *T { return &value }
