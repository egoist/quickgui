package ui

import "github.com/egoist/quickgui/go/native"

import (
	_ "embed"
	"strings"

	gui "github.com/egoist/quickgui/go/ui"
)

const (
	iconHeader      = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">`
	branchIcon      = iconHeader + `<path d="M6 6v12m12-12v2a6 6 0 0 1-6 6H6"/><circle cx="6" cy="3" r="2"/><circle cx="6" cy="20" r="2"/><circle cx="18" cy="3" r="2"/></svg>`
	fetchIcon       = iconHeader + `<path d="M12 3v12m-4-4 4 4 4-4M4 16v4h16v-4"/></svg>`
	pullIcon        = iconHeader + `<path d="M12 4v16m-6-6 6 6 6-6"/></svg>`
	pushIcon        = iconHeader + `<path d="M12 20V4m-6 6 6-6 6 6"/></svg>`
	checkIcon       = iconHeader + `<path d="m5 12 4 4L19 6"/></svg>`
	closeIcon       = iconHeader + `<path d="m6 6 12 12M6 18 18 6"/></svg>`
	chevronDownIcon = iconHeader + `<path d="m6 9 6 6 6-6"/></svg>`
	plusIcon        = iconHeader + `<path d="M12 5v14M5 12h14"/></svg>`
	minusIcon       = iconHeader + `<path d="M5 12h14"/></svg>`
)

// Lucide refresh-cw: https://lucide.dev/icons/refresh-cw. License: icons/LICENSE.
//
//go:embed icons/refresh-cw.svg
var refreshIcon string

func icon(svg string, size float64, color func() string) *gui.Element {
	return gui.SVG().
		Width(size).
		Height(size).
		FlexShrink(0).
		Value(func() string { return strings.ReplaceAll(svg, "currentColor", color()) })

}

func toolbarIcon(svg string) *gui.Element {
	app := UseApp()
	return icon(svg, 14, func() string { return app.Theme().TextSecondary })
}

func currentIndicator(current func() bool) *native.Node {
	return gui.Show(
		current,
		func() *gui.Element { return toolbarIcon(checkIcon) },
	)
}
