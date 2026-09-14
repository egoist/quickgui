package main

import "github.com/egoist/quickgui/go/native"

import (
	_ "embed"
	"encoding/base64"

	"github.com/egoist/quickgui/go/ui"
)

// Color filters have a direct raster path; blur and backdrop effects composite
// subtrees. Embed the raster swatch so the example needs no external assets.
//
//go:embed assets/filter-swatch.png
var filterSwatchPNG []byte

var filterSwatch = "data:image/png;base64," + base64.StdEncoding.EncodeToString(filterSwatchPNG)

func Gradients() *ui.Element {
	tile := ui.Style().Height(64).BorderRadius(12).FontWeight(700)
	return Panel("Gradients", func() *native.Node {
		return ui.Fragment([]*native.Node{
			swatch("linear-gradient", tile, ui.Style().Background("linear-gradient(135deg, #1d4ed8, #38bdf8 60%, #a855f7)")).Node,
			swatch("radial-gradient", tile, ui.Style().
				Background("radial-gradient(circle closest-side at 30% 40%, #f8fafc, #0f172a)")).Node,
			swatch("conic-gradient", tile, ui.Style().Background("conic-gradient(from 200deg, #f97316, #38bdf8, #f97316)")).Node,
			swatch("object form, oklab", tile, ui.Style().
				Background(ui.GradientDeclaration{
					Type:          "linear",
					Angle:         ptr(90.0),
					Interpolation: "oklab",
					Stops: []ui.GradientStop{
						{Color: "#0ea5e9", Position: ptr(0.0)},
						{Color: "#e879f9", Position: ptr(1.0)},
					},
				})).Node,
		})
	})
}

func BordersAndOutlines() *ui.Element {
	return Panel("Corners, borders, and outlines", func() *native.Node {
		return ui.Fragment([]*native.Node{
			ui.View().
				Height(56).
				BorderRadius("22px 6px 22px 6px").
				BackgroundColor("#1b2434").
				BorderWidth(1).
				BorderColor("#38bdf8").Node,
			ui.View().
				Height(56).
				BorderRadius(12).
				BorderWidth(2).
				BorderColor("#f97316").
				BorderStyle("dashed").Node,
			ui.View().
				Height(56).
				BorderRadius(12).
				BorderWidth(2).
				BorderColor("#22c55e").
				BorderStyle("dotted").Node,
			ui.View().
				Height(56).
				Margin(6).
				BorderRadius(12).
				BackgroundColor("#1b2434").
				Outline("2px solid #a855f7").
				OutlineOffset(4).Node,
			ui.View().
				Height(56).
				Margin(6).
				BorderTopLeftRadius(28).
				BorderBottomRightRadius(28).
				BackgroundColor("#1b2434").
				Outline("2px dashed #38bdf8").
				OutlineOffset(3).Node,
		})
	})
}

func Filters() *ui.Element {
	tile := ui.Style().
		Height(52).
		BorderRadius(10).
		Background("linear-gradient(90deg, #f97316, #38bdf8)").
		FontWeight(700)
	return Panel("Filters and backdrop", func() *native.Node {
		var children []*native.Node
		raster := ui.Style().
			BackgroundImage(filterSwatch).
			BackgroundSize("cover").
			BackgroundRepeat("no-repeat")
		children = append(children, swatch("saturate + contrast", tile, raster, ui.Style().Filter("saturate(1.8) contrast(1.15)")).Node)
		children = append(children, swatch("grayscale", tile, raster, ui.Style().Filter("grayscale(1) brightness(1.2)")).Node)
		children = append(children, swatch("hue-rotate", tile, raster, ui.Style().Filter("hue-rotate(140deg)")).Node)
		children = append(children, swatch("subtree blur", tile, ui.Style().Filter("blur(2px)")).Node)
		children = append(children, swatch("drop-shadow", tile, ui.Style().Filter("drop-shadow(0 6px 12px #0b1220)")).Node)
		children = append(children, ui.View().
			Child(

				swatch("backdropFilter", ui.Style().
					Width("100%").
					Height("100%").
					BorderRadius(10).
					BackgroundColor("#0f172a80").
					BackdropFilter("blur(14px) brightness(1.1)").
					FontWeight(700)),
			).
			Position("relative").
			Height(84).
			BorderRadius(12).
			Padding(12).
			Background("conic-gradient(from 30deg, #1d4ed8, #f97316, #1d4ed8)").Node)
		return ui.Fragment(children)
	})
}

func Transforms() *ui.Element {
	card := ui.Style().
		Height(54).
		BorderRadius(12).
		BackgroundColor("#1b2434").
		BorderColor(panelBorder).
		BorderWidth(1)
	return Panel("Transforms and blending", func() *native.Node {
		return ui.Fragment([]*native.Node{
			swatch("rotate(-3deg)", card, ui.Style().Transform("rotate(-3deg)")).Node,
			swatch("skew from the left edge", card, ui.Style().
				Transform("skew(8deg, 0)").
				TransformOrigin("left center")).Node,
			swatch("hover to lift, glow, and ring", card, ui.Style().
				Transform("scale(0.96)").
				OutlineOffset(3).
				Cursor("pointer").
				Transition(ui.TransitionDeclaration{
					Properties: []string{"background-color"},
					Duration:   "120ms",
					Easing:     "ease-out",
				}).
				Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
					return s.
						Transform("scale(1.03) translate(0, -2px)").
						Background("linear-gradient(90deg, #1d4ed8, #38bdf8)").
						Outline("2px solid #93c5fd")
				})).Node,
			ui.View().
				Child(
					func() *native.Node {
						var children []*native.Node
						for _, tile := range []struct{ color, blend string }{
							{"#f8fafc", "multiply"}, {"#334155", "screen"}, {"#94a3b8", "overlay"},
						} {
							children = append(children, ui.View().
								Flex(1).
								BorderRadius(10).
								BackgroundColor(tile.color).
								MixBlendMode(tile.blend).Node)
						}
						return ui.Fragment(children)
					},
				).
				Display("flex").
				Height(84).
				Gap(10).
				Padding(10).
				BorderRadius(12).
				Background("linear-gradient(90deg, #f97316, #38bdf8)").Node,
		})
	})
}
