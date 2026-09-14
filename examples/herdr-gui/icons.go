package main

import (
	"github.com/egoist/quickgui/go/ui"
	"strings"
)

const svgFrame = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">`

var icons = map[string]string{
	"plus":           `<path d="M12 5v14M5 12h14"/>`,
	"close":          `<path d="m7 7 10 10M17 7 7 17"/>`,
	"sun":            `<circle cx="12" cy="12" r="3.5"/><path d="M12 2.5v2M12 19.5v2M4.5 4.5l1.4 1.4M18.1 18.1l1.4 1.4M2.5 12h2M19.5 12h2M4.5 19.5l1.4-1.4M18.1 5.9l1.4-1.4"/>`,
	"moon":           `<path d="M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401"/>`,
	"columns":        `<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M12 4v16"/>`,
	"rows":           `<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M3 12h18"/>`,
	"terminal":       `<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="m7 9 3 3-3 3M13 15h4"/>`,
	"circle":         `<circle cx="12" cy="12" r="7"/>`,
	"loader":         `<path d="M20 12a8 8 0 1 1-2.35-5.65"/><path d="M17.65 3.5v3h-3"/>`,
	"check-circle":   `<circle cx="12" cy="12" r="8"/><path d="m8.5 12 2.2 2.2 4.8-5"/>`,
	"warning-circle": `<circle cx="12" cy="12" r="8"/><path d="M12 8v4.5M12 16h.01"/>`,
}

func icon(name string, size float64, color func() string) *ui.Element {
	return dynamicIcon(func() string { return name }, size, color)
}
func dynamicIcon(name func() string, size float64, color func() string) *ui.Element {
	return ui.SVG().
		Width(size).
		Height(size).
		FlexShrink(0).
		Value(func() string {
			return strings.ReplaceAll(svgFrame+icons[name()]+"</svg>", "currentColor", color())
		})

}
func statusGlyph(m *model, status func() string, compact bool) *ui.Element {
	size := choose(compact, 12.0, 14.0)
	return ui.View().
		Child(

			dynamicIcon(func() string {
				switch status() {
				case "blocked":
					return "warning-circle"
				case "working":
					return "loader"
				case "idle":
					return "check-circle"
				}
				return "circle"
			}, size-1, func() string {
				switch status() {
				case "blocked":
					return m.theme().Danger
				case "working":
					return m.theme().Working
				case "idle":
					return m.theme().Success
				}
				return m.theme().TextGhost
			}),
		).
		Display("flex").
		Width(size).
		Height(size).
		FlexShrink(0).
		AlignItems("center").
		JustifyContent("center")
}
func iconButton(m *model, label, name string, size float64, click func()) *ui.Element {
	return ui.Button().
		AriaLabel(label).
		FocusOnPointer(false).
		Style(m.iconStyle(size)).
		OnClick(click).
		Child(icon(name, 14, m.color(func(t theme) string { return t.TextTertiary })))
}
