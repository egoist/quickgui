package main

import (
	"fmt"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

type palette struct {
	Window, Sidebar, Panel, PanelAlt, Control, ControlHover, ControlActive string
	Ink, Muted, Faint, Border, Accent, AccentHover, OnAccent, Selection    string
	Popup, Backdrop, Track, Danger, DangerHover                            string
}

var lightPalette = palette{"#f4f5f7", "#ebedf1", "#ffffff", "#f8f9fb", "#ffffff", "#eef1f6", "#e2e8f4", "#131820", "#5a6472", "#8b94a3", "#d9dde4", "#2563eb", "#1d4fd7", "#ffffff", "#dbe6fd", "#ffffff", "#1e293b66", "#e4e7ec", "#b42318", "#9a1d14"}
var darkPalette = palette{"#0e1117", "#131924", "#171e2a", "#1b2331", "#1c2432", "#263042", "#2f3a50", "#e7edf7", "#98a4b6", "#6e7a8c", "#2a3446", "#5b93f7", "#7aa7ff", "#08111f", "#1f2d47", "#171e2a", "#010409aa", "#252f41", "#f0736a", "#f58c84"}

type galleryTheme struct {
	Appearance func() string
	Size       func() native.Point
}

var galleryContext = reactive.CreateContext(galleryTheme{Appearance: func() string {
	return "light"
}, Size: func() native.Point {
	return native.Point{X: 1080, Y: 780}
}})

func p() palette {
	return choose(galleryContext.Use().Appearance() == "dark", darkPalette, lightPalette)
}
func color(read func(palette) string) func() string {
	return func() string {
		return read(p())
	}
}
func ptr[T any](value T) *T {
	return &value
}
func choose[T any](condition bool, yes, no T) T {
	if condition {
		return yes
	}
	return no
}
func change[T any](write func(T)) func(T, *native.Event) {
	return func(value T, _ *native.Event) {
		write(value)
	}
}
func textValue[T any](value *T) string {
	if value == nil {
		return "—"
	}
	return fmt.Sprint(*value)
}
func controlStyle() ui.StyleBuilder {
	return ui.Style().
		Display("flex").
		FlexDirection("row").
		AlignItems("center").
		JustifyContent("center").
		Gap(6).
		Height(30).
		FlexShrink(0).
		PaddingLeft(12).
		PaddingRight(12).
		BorderRadius(8).
		BackgroundColor(color(func(p palette) string {
			return p.Control
		})).
		BorderColor(color(func(p palette) string {
			return p.Border
		})).
		BorderWidth(1).
		TextColor(color(func(p palette) string {
			return p.Ink
		})).
		FontSize(13).
		Cursor("default").
		UserSelect("none").
		AppRegion("no-drag").
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.BackgroundColor(color(func(p palette) string {
				return p.ControlHover
			}))
		}).
		FocusStyle(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.OutlineWidth(2).OutlineColor(color(func(p palette) string {
				return p.Accent
			}))
		}).
		OutlineOffset(2).
		DisabledStyle(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.Opacity(0.45)
		})
}
func inputStyle() ui.StyleBuilder {
	return ui.Style().
		Height(30).
		PaddingLeft(10).
		PaddingRight(10).
		BorderRadius(8).
		BackgroundColor(color(func(p palette) string {
			return p.PanelAlt
		})).
		BorderColor(color(func(p palette) string {
			return p.Border
		})).
		BorderWidth(1).
		TextColor(color(func(p palette) string {
			return p.Ink
		})).
		FontSize(13).
		FocusStyle(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.OutlineWidth(2).OutlineColor(color(func(p palette) string {
				return p.Accent
			}))
		}).
		OutlineOffset(2)
}
func popupStyle() ui.StyleBuilder {
	return ui.Style().
		Display("flex").
		FlexDirection("column").
		Gap(8).
		Padding(14).
		BorderRadius(12).
		BackgroundColor(color(func(p palette) string {
			return p.Popup
		})).
		BorderColor(color(func(p palette) string {
			return p.Border
		})).
		BorderWidth(1).
		TextColor(color(func(p palette) string {
			return p.Ink
		})).
		BoxShadow("0 18px 40px #00000033")
}
func fillStyle() ui.StyleBuilder {
	return ui.Style().Position("absolute").Top(0).Right(0).Bottom(0).Left(0)
}
func control() ui.PartProps {
	return ui.PartProps{Style: controlStyle()}
}
func columnPart() ui.PartProps {
	return ui.PartProps{Style: ui.Style().Display("flex").FlexDirection("column").Gap(8)}
}
func rowPart() ui.PartProps {
	return ui.PartProps{Style: ui.Style().Display("flex").FlexDirection("row").AlignItems("center").Gap(10)}
}
func popup(width float64) ui.PartProps {
	s := popupStyle()
	s = s.Width(width)
	return ui.PartProps{Style: s}
}
func backdrop() ui.PartProps {
	s := fillStyle()
	s = s.BackgroundColor(color(func(p palette) string {
		return p.Backdrop
	}))
	return ui.PartProps{Style: s}
}
func overlay() ui.PartProps {
	s := fillStyle()
	s = s.Display("flex")
	s = s.AlignItems("center")
	s = s.JustifyContent("center")
	return ui.PartProps{Style: s}
}
func panel(title, hint string, children ui.Component) *ui.Element {
	return ui.View().
		Children(
			ui.Text(title).FontSize(17).FontWeight(700),
			ui.Text(hint).
				FontSize(12).
				LineHeight(18).
				TextColor(color(func(p palette) string {
					return p.Muted
				})),
			children,
		).
		Display("flex").
		FlexDirection("column").
		Gap(14).
		Padding(20).
		BorderRadius(12).
		BorderWidth(1).
		BorderColor(color(func(p palette) string {
			return p.Border
		})).
		BackgroundColor(color(func(p palette) string {
			return p.Panel
		})).
		FlexShrink(0)
}
func row(children ui.Component) *ui.Element {
	return ui.View().
		Child(children).
		Display("flex").
		FlexDirection("row").
		AlignItems("center").
		FlexWrap("wrap").
		Gap(10)
}
func col(children ui.Component) *ui.Element {
	return ui.View().Child(children).Display("flex").FlexDirection("column").Gap(8)
}
func note(value any) *ui.Element {
	return ui.Text(value).
		FontSize(12).
		LineHeight(18).
		FontFamily("monospace").
		TextColor(color(func(p palette) string {
			return p.Muted
		}))
}
func label(value any) *ui.Element {
	return ui.Text(value).FontSize(12)
}
func muted(value any) *ui.Element {
	return ui.Text(value).
		FontSize(12).
		LineHeight(17).
		TextColor(color(func(p palette) string {
			return p.Muted
		}))
}
func button(label any, click func()) *ui.Element {
	return ui.Button().Style(controlStyle()).OnClick(click).Child(label)
}
func primary(label any, click func()) *ui.Element {
	return button(label, click).
		Style(ui.Style().
			BackgroundColor(color(func(p palette) string {
				return p.Accent
			})).
			TextColor(color(func(p palette) string {
				return p.OnAccent
			})).
			Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
				return s.BackgroundColor(color(func(p palette) string {
					return p.AccentHover
				}))
			}))
}
func input(value any, set func(string), placeholder string) *ui.Element {
	return ui.Input().Style(inputStyle()).Value(value).Placeholder(placeholder).OnInput(set)
}
func checkboxStyle() ui.StyleBuilder {
	s := controlStyle()
	s = s.Height(28)
	s = s.BorderWidth(0)
	s = s.BackgroundColor("transparent")
	s = s.JustifyContent("flex-start")
	s = s.PaddingLeft(8)
	return s
}
func checkbox(props ui.CheckboxProps, caption any) *native.Node {
	if props.Style == nil {
		props.Style = checkboxStyle()
	}
	if props.Checked == nil {
		initial := props.DefaultChecked
		if initial == nil {
			initial = false
		}
		checked, setChecked := ui.CreateSignal(initial)
		onChange := props.OnCheckedChange
		props.Checked = checked
		props.OnCheckedChange = func(next bool, event *native.Event) {
			setChecked(next)
			if onChange != nil {
				onChange(next, event)
			}
		}
	}
	checkbox34 := ui.NewCheckbox(props)
	return checkbox34.Root().
		Children(func() *native.Node {
			return ui.Fragment([]*native.Node{
				checkbox34.Indicator(ui.PartProps{Style: func() ui.StyleBuilder {
					border, background := p().Border, p().Control
					if props.Checked() != false {
						border, background = p().Accent, p().Accent
					}
					return ui.Style().
						Width(16).
						Height(16).
						BorderRadius(5).
						BorderWidth(1).
						BorderColor(border).
						BackgroundColor(background).
						TextColor(p().OnAccent).
						Display("flex").
						AlignItems("center").
						JustifyContent("center")
				}}).
					Children(func() *native.Node {
						return ui.Show(
							props.Checked() != false,
							func() *ui.Element {
								return ui.SVG().
									Value(func() string {
										path := "M3 6l2 2 4-4"
										if props.Checked() == ui.CheckedIndeterminate {
											path = "M3 6h6"
										}
										return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="` + path + `" fill="none" stroke="` + p().OnAccent + `" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>`
									}).
									Width(12).
									Height(12).
									FlexShrink(0)
							},
						)
					}).
					NativeNode(),
				label(caption).Node,
			})
		}).
		NativeNode()
}
func checkboxSelectionState(checked, total int) ui.CheckedState {
	if checked == 0 {
		return false
	}
	if checked == total {
		return true
	}
	return ui.CheckedIndeterminate
}
