package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:                "QuickGUI Sidebar Vibrancy",
				Width:                860,
				Height:               620,
				MinimumWidth:         700,
				MinimumHeight:        480,
				Background:           "transparent",
				Vibrancy:             "sidebar",
				VisualEffectState:    "followWindow",
				Appearance:           "light",
				TitleBarStyle:        "hiddenInset",
				TrafficLightPosition: &native.Point{X: 16, Y: 19},
				Component:            Vibrancy,
			})
		}
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				open()
			}
		})
		open()
	}); err != nil {
		log.Fatal(err)
	}
}

func Vibrancy() *ui.Element {
	window := native.CurrentWindow()
	material, setMaterial := ui.CreateSignal("sidebar")
	state, setState := ui.CreateSignal("followWindow")
	return ui.View().
		Children(

			ui.View().
				Children(

					ui.View().
						Child(

							ui.Text("Vibrancy").FontSize(13).FontWeight(700),
						).
						Display("flex").
						Height(52).
						FlexShrink(0).
						AlignItems("center").
						PaddingLeft(90).
						PaddingRight(14).
						AppRegion("drag"),
					ui.View().
						Child(
							func() *native.Node {
								var children []*native.Node
								for _, option := range materials {
									selected := func() bool { return material() == option.value }
									children = append(children, ui.Button().
										Children(
											ui.Text(
												option.label,
											).
												FontSize(12).
												FontWeight(600),
											ui.Show(selected, checkmark),
										).
										Display("flex").
										Width("100%").
										Height(31).
										FlexShrink(0).
										AlignItems("center").
										JustifyContent("space-between").
										PaddingLeft(11).
										PaddingRight(11).
										BackgroundColor("transparent").
										BorderColor("transparent").
										BorderWidth(1).
										BorderRadius(7).
										TextColor("#263247").
										Cursor("default").
										AppRegion("no-drag").
										UserSelect("none").
										When(
											selected,
											ui.Style().BackgroundColor("#ffffff52"),
											ui.Style().BorderColor("#ffffff70"),
										).
										Selected(selected).
										OnClick(func() {
											setMaterial(option.value)
											window.SetVibrancy(option.value)
										}).Node)

								}
								return ui.Fragment(children)
							},
						).
						Display("flex").
						FlexDirection("column").
						Flex(1).
						MinHeight(0).
						Gap(3).
						PaddingLeft(10).
						PaddingRight(10).
						PaddingBottom(10).
						OverflowY("auto"),
					ui.View().
						Children(

							ui.Text(
								"EFFECT STATE",
							).
								TextColor("#59667b").
								FontSize(11).
								FontWeight(700),
							ui.View().
								Child(
									func() *native.Node {
										var children []*native.Node
										for _, option := range effectStates {
											selected := func() bool { return state() == option.value }
											children = append(children, ui.Button().
												Child(option.label).
												Display("flex").
												Flex(1).
												Height(27).
												MinWidth(0).
												AlignItems("center").
												JustifyContent("center").
												BackgroundColor("#ffffff24").
												BorderColor("#ffffff3d").
												BorderWidth(1).
												BorderRadius(6).
												TextColor("#445168").
												FontSize(10).
												FontWeight(600).
												Cursor("default").
												AppRegion("no-drag").
												UserSelect("none").
												When(
													selected,
													ui.Style().BackgroundColor("#ffffff5c"),
													ui.Style().BorderColor("#ffffff7a"),
												).
												Selected(selected).
												OnClick(func() {
													setState(option.value)
													window.SetVisualEffectState(option.value)
												}).Node)

										}
										return ui.Fragment(children)
									},
								).
								Display("flex").
								Gap(5),
						).
						Display("flex").
						FlexDirection("column").
						FlexShrink(0).
						Gap(7).
						Padding(12).
						BorderColor("#c1c1c2").
						BorderTopWidth(1),
				).
				Display("flex").
				FlexDirection("column").
				Width(254).
				Height("100%").
				FlexShrink(0).
				BackgroundColor("transparent").
				BorderColor("#cccccc").
				BorderRightWidth(1),
			ui.View().
				Children(

					ui.View().
						Child(

							ui.Text(material()).FontSize(13).FontWeight(700),
						).
						Display("flex").
						Height(52).
						FlexShrink(0).
						AlignItems("center").
						JustifyContent("center").
						BorderColor("#e2e8f0").
						BorderBottomWidth(1).
						AppRegion("drag"),
					ui.View().
						Child(

							ui.View().
								Children(

									ui.Text(
										"GO + QUICKGUI",
									).
										TextColor("#2563eb").
										FontSize(12).
										FontWeight(700),
									ui.Text(
										"Every macOS vibrancy type",
									).
										FontSize(26).
										LineHeight(33).
										FontWeight(700),
									ui.Text(
										"Select any Electron-compatible semantic material. QuickGUI updates one native "+
											"NSVisualEffectView while the retained QuickGUI tree and Metal surface stay mounted. "+
											"This pane is opaque, so the selected material remains visually confined to the translucent sidebar.",
									).
										TextColor("#667085").
										FontSize(14).
										LineHeight(21),
									ui.View().
										Children(

											readout("Material", material),
											readout("Effect state", state),
											readout("Content", "Opaque"),
										).
										Display("flex").
										Gap(10).
										PaddingTop(4),
								).
								Display("flex").
								FlexDirection("column").
								Width("100%").
								MaxWidth(480).
								Gap(16).
								Padding(28).
								BackgroundColor("#ffffff").
								BorderColor("#dfe5ed").
								BorderWidth(1).
								BorderRadius(14).
								BoxShadow("0 18px 45px -24px rgba(15, 23, 42, 0.35)"),
						).
						Display("flex").
						Flex(1).
						MinHeight(0).
						AlignItems("center").
						JustifyContent("center").
						Padding(36),
				).
				Display("flex").
				FlexDirection("column").
				Flex(1).
				MinWidth(0).
				Height("100%").
				BackgroundColor("#ffffff"),
		).
		Display("flex").
		Width("100%").
		Height("100%").
		BackgroundColor("transparent").
		TextColor("#172033")
}
