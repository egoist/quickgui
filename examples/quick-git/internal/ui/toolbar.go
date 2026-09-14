package ui

import "github.com/egoist/quickgui/go/native"
import (
	"fmt"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/model"
	"strconv"
)

func Toolbar() *gui.Element {
	app := UseApp()
	store := app.Store
	busy := func() bool {
		return store.Busy() != nil
	}
	pullDisabled := func() bool {
		status := store.Status()
		return busy() || status == nil || status.Upstream == ""
	}
	pushDisabled := func() bool {
		status := store.Status()
		return busy() || status == nil || status.Branch == ""
	}
	return gui.View().
		Children(
			gui.Button().
				Child(toolbarIcon(branchIcon)).
				Style(app.Theme().Button("secondary")).
				Child(gui.Text(func() string {
					if status := store.Status(); status != nil {
						if status.Branch != "" {
							return status.Branch
						}
						if status.Detached && len(status.HeadSha) >= 7 {
							return status.HeadSha[:7] + " (detached)"
						}
					}
					return "…"
				}).
					MinWidth(0).
					LineClamp(1)).
				OnClick(func() {
					store.SetView(model.ViewBranches)
				}).
				FlexDirection("row").
				Height(28).
				MaxWidth(260).
				BackgroundColor("transparent").
				BorderWidth(0),
			gui.Show(
				func() bool {
					status := store.Status()
					return status != nil && status.HasUpstreamCounts && (status.Ahead > 0 || status.Behind > 0)
				},
				func() *gui.Element {
					return gui.View().
						Children(
							toolbarIcon(pushIcon),
							gui.Text(fmt.Sprint(store.Status().Ahead)),
							toolbarIcon(pullIcon),
							gui.Text(fmt.Sprint(store.Status().Behind)),
						).
						AriaLabel("Upstream commit counts").
						Display("flex").
						AlignItems("center").
						Gap(3).
						FontSize(11).
						TextColor(app.Theme().TextSecondary).
						AppRegion("no-drag")
				},
			),
			gui.Show(
				store.Conflicts() > 0,
				func() *gui.Element {
					return gui.Text(strconv.Itoa(store.Conflicts()) + " conflicted").
						FontSize(11).
						FontWeight(600).
						TextColor(app.Theme().Warning).
						AppRegion("no-drag")
				},
			),
			gui.View().Flex(1).AppRegion("drag"),
			gui.Show(busy, toolbarBusy),
			toolbarButton("Fetch", fetchIcon, store.Fetch, busy, false),
			toolbarButton("Pull", pullIcon, store.Pull, pullDisabled, false),
			toolbarButton("Push", pushIcon, store.Push, pushDisabled, false),
			toolbarButton("Refresh", refreshIcon, store.Refresh, busy, true),
		).
		Display("flex").
		Height(TitlebarHeight).
		FlexShrink(0).
		AlignItems("center").
		Gap(6).
		PaddingLeft(14).
		PaddingRight(12).
		BorderBottomWidth(1).
		BorderColor(app.Theme().Border).
		BackgroundColor(app.Theme().Content).
		AppRegion("drag")
}
func toolbarBusy() *gui.Element {
	app := UseApp()
	store := app.Store
	return gui.View().
		Children(
			func() *native.Node {
				progress66 := gui.NewProgress(gui.ProgressProps{
					GaugeFormatProps: gui.GaugeFormatProps{PartProps: gui.PartProps{
						AriaLabel: "Git operation in progress",
						Style:     gui.Style().Width(32).Height(4).FlexShrink(0),
					}},
					Indeterminate: func() bool {
						return true
					},
				})
				return progress66.Root().
					Children(func() *native.Node {
						return progress66.Track(gui.PartProps{Style: gui.Style().
							Width("100%").
							Height(4).
							BorderRadius(2).
							BackgroundColor(app.Theme().BorderStrong)}).
							Children(func() *native.Node {
								return progress66.Indicator(gui.PartProps{Style: gui.Style().
									Width(12).
									Height(4).
									BorderRadius(2).
									BackgroundColor(app.Theme().TextSecondary)}).
									NativeNode()
							}).
							NativeNode()
					}).
					NativeNode()
			}(),
			gui.Text(func() string {
				if busy := store.Busy(); busy != nil {
					return busy.Label + "…"
				}
				return ""
			}).
				FontSize(12).
				TextColor(app.Theme().TextSecondary),
			gui.Show(
				func() bool {
					busy := store.Busy()
					return busy != nil && busy.Cancel != nil
				},
				func() *gui.Element {
					return gui.Button().
						Style(app.Theme().Button("secondary")).
						Child("Cancel").
						OnClick(store.CancelBusy)
				},
			),
		).
		Display("flex").
		FlexDirection("row").
		AlignItems("center").
		Gap(8).
		MarginRight(8).
		AppRegion("no-drag")
}
func toolbarButton(label, icon string, click func(), disabled func() bool, iconOnly bool) *gui.Element {
	app := UseApp()
	return gui.Button().
		Style(app.Theme().Button("secondary")).
		Child(func() *native.Node {
			var children []*native.Node
			children = append(children, toolbarIcon(icon).Node)
			if !iconOnly {
				children = append(children, gui.Text(label).Node)
			}
			return gui.Fragment(children)
		}).
		AriaLabel(label).
		OnClick(click).
		Disabled(disabled).
		FlexDirection("row").
		Height(28).
		BorderRadius(7)
}
