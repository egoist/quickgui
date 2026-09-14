package main

import (
	"log"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

const identifier = "dev.quickgui.system-api-example"
const appName = "QuickGUI System APIs"

func main() {
	if err := native.Run(func() {
		native.App.RequestSingleInstanceLock(
			identifier,
			func(primary bool, err error) {
				if err != nil {
					log.Print(err)
					native.App.Exit(1, nil)
					return
				}
				if !primary {
					native.App.Exit(0, nil)
					return
				}
				open := func() {
					native.NewWindow(native.WindowOptions{
						Title:                appName,
						Width:                760,
						Height:               640,
						MinimumWidth:         620,
						MinimumHeight:        520,
						Background:           "#0b0e14",
						TitleBarStyle:        "hiddenInset",
						TrafficLightPosition: &native.Point{X: 16, Y: 14},
						Component:            SystemAPIs,
					})
				}
				native.App.OnReopen(func(event native.ReopenEvent) {
					if !event.HasVisibleWindows {
						open()
					}
				})
				open()
			},
		)
	}); err != nil {
		log.Fatal(err)
	}
}

func SystemAPIs() *ui.Element {
	window := native.CurrentWindow()
	status, setStatus := ui.CreateSignal("Primary instance")
	busy, setBusy := ui.CreateSignal(false)
	state := &systemState{window: window, status: setStatus, busy: busy, setBusy: setBusy}
	ui.OnCleanup(state.dispose)
	native.App.OnSecondInstance(func(event native.SecondInstanceEvent) {
		state.showWindow()
		setStatus("A second launch forwarded " + strconv.Itoa(len(event.Argv)) + " argument(s).")
	})
	native.App.OnOpenURLs(func(event native.OpenURLsEvent) {
		setStatus("Deep link: " + strings.Join(event.URLs, ", "))
	})
	native.PowerMonitor.OnEvent(func(event native.PowerEvent) { setStatus("Power event: " + event.Type) })
	native.SystemPreferences.OnChange(func(value native.SystemPreferencesSnapshot) {
		setStatus("System preferences changed to " + value.ColorScheme + " appearance.")
	})
	native.App.OnNotificationResponse(func(event native.NotificationResponseEvent) {
		message := "Notification " + event.Tag + " activated"
		if event.Reply != nil {
			message += ": " + *event.Reply
		} else if event.ActionID != nil {
			message += " via " + *event.ActionID
		}
		setStatus(message)
	})
	native.SetApplicationMenu([]native.MenuDefinition{
		{Label: "App", Items: []native.MenuItem{
			{Label: "Show window", Click: state.showWindow},
			{Type: "separator"},
			{Label: "Hide", Role: "hide-application"},
			{Label: "Quit", Role: "quit"},
		}},
		{Label: "Edit", Items: []native.MenuItem{
			{Label: "Copy", Role: "copy"},
			{Label: "Paste", Role: "paste"},
			{Label: "Select All", Role: "select-all"},
		}},
		{Label: "Window", Items: []native.MenuItem{
			{Label: "Minimize", Role: "minimize-window"},
			{Label: "Close", Role: "close-window"},
		}},
	})
	return ui.View().
		Children(

			ui.View().
				Child(
					"Native system APIs",
				).
				Display("flex").
				Height(52).
				FlexShrink(0).
				AlignItems("center").
				JustifyContent("center").
				FontSize(14).
				FontWeight(600).
				AppRegion("drag").
				BorderColor("#1f2530").
				BorderBottomWidth(1),
			ui.View().
				Children(

					ui.Text(
						"Native integrations",
					).
						FontSize(26).
						LineHeight(32).
						FontWeight(700),
					ui.Text(
						"Typed Go APIs for application state, desktop services, notifications, and native resources.",
					).
						TextColor("#9aa6b7").
						FontSize(14).
						LineHeight(21),
					ui.View().
						Child(
							func() *native.Node {
								var children []*native.Node
								for _, item := range state.actions() {
									children = append(children, ui.Button().
										Style(buttonStyle).
										Child(item.label).
										Disabled(busy()).
										OnClick(func() { state.run(item) }).Node)

								}
								return ui.Fragment(children)
							},
						).
						Display("flex").
						FlexWrap("wrap").
						Gap(10),
					ui.View().
						Child(

							ui.Text(
								status(),
							).
								FontSize(13).
								LineHeight(19).
								UserSelect("text").
								FontFamily("monospace").
								TextColor(func() string {
									if busy() {
										return "#c7d2fe"
									}
									return "#aeb9c9"
								}),
						).
						MinHeight(68).
						Padding(16).
						BackgroundColor("#111620").
						BorderColor("#293242").
						BorderWidth(1).
						BorderRadius(10),
				).
				Display("flex").
				FlexDirection("column").
				Flex(1).
				MinHeight(0).
				Gap(18).
				Padding(28).
				OverflowY("auto"),
		).
		Display("flex").
		FlexDirection("column").
		Width("100%").
		Height("100%").
		BackgroundColor("#0b0e14").
		TextColor("#f4f7fb")
}
