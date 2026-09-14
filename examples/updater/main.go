package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
	"github.com/egoist/quickgui/go/updater"
)

func main() {
	if err := native.Run(func() {
		status, setStatus := ui.CreateSignal("Starting updater…")
		state, setState := ui.CreateSignal(updater.Event{})
		report := func(err error) {
			if err != nil {
				setStatus(err.Error())
			}
		}
		updates := updater.Start(updater.Options{}, func(event updater.Event) {
			setState(event)
			switch {
			case event.Error != "":
				setStatus(event.Error)
			case event.Status == updater.Disabled:
				setStatus("Updates are disabled in development builds.")
			case event.QuitRequired:
				native.App.Quit(false, nil)
			case event.Kind == "up-to-date":
				setStatus("You’re up to date.")
			case event.Status == updater.Available:
				setStatus("Version " + event.Version + " is available.")
			default:
				setStatus(string(event.Status))
			}
		}, report)
		native.NewWindow(native.WindowOptions{
			Title:  "Updater Example",
			Width:  520,
			Height: 340,
			Component: func() *ui.Element {
				return ui.View().
					Children(

						ui.Text(
							"Application updates",
						).
							FontSize(24).
							FontWeight(700),
						ui.Text(status()).LineHeight(22),
						ui.Text(
							"macOS uses Sparkle. Windows and Linux share its signed appcast format.",
						).
							TextColor("#64748b").
							LineHeight(22),
						ui.Button().
							Child("Check for Updates…").
							OnClick(func() { updates.Check(report) }).
							Padding(10).
							BackgroundColor("#2563eb").
							TextColor("white").
							BorderRadius(8),
						ui.Show(
							state().Status == updater.Available,
							func() *ui.Element {
								return ui.Button().
									Child("Install update").
									OnClick(func() { updates.Install(report) }).
									Padding(10).
									BackgroundColor("#16a34a").
									TextColor("white").
									BorderRadius(8)
							},
						),
						ui.Button().
							Child(func() string {
								if state().AutomaticChecks {
									return "Disable automatic checks"
								}
								return "Enable automatic checks"
							}).
							OnClick(func() {
								updates.SetAutomaticChecks(!state().AutomaticChecks, report)
							}).
							Padding(10),
					).
					Display("flex").
					FlexDirection("column").
					Gap(16).
					Padding(28).
					Width("100%").
					Height("100%")
			},
		})
	}); err != nil {
		log.Fatal(err)
	}
}
