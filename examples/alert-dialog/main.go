package main

import (
	"strconv"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	run(native.WindowOptions{
		Title:         "QuickGUI Alert Dialogs",
		Width:         680,
		Height:        500,
		MinimumWidth:  540,
		MinimumHeight: 420,
	}, Alerts)
}

func Alerts() *ui.Element {
	window := native.CurrentWindow()
	status, setStatus := ui.CreateSignal("Choose a dialog to present.")
	pending, setPending := ui.CreateSignal(false)
	show := func(options native.AlertDialogOptions, message func(int) string) {
		if pending() {
			return
		}
		setPending(true)
		setStatus("Waiting for the alert dialog…")
		native.ShowAlertDialog(
			options,
			func(index int, err error) {
				if window.Closed {
					return
				}
				setPending(false)
				if err != nil {
					setStatus("Dialog failed: " + err.Error())
				} else {
					setStatus(message(index))
				}
			},
		)
	}
	return ui.View().
		Children(

			ui.Text(
				"System-owned UI",
			).
				FontSize(26).
				LineHeight(32).
				FontWeight(700),
			ui.Text(
				"Present a native alert from an event handler and receive the selected button in its completion callback.",
			).
				TextColor("#9aa6b7").
				FontSize(14).
				LineHeight(21),
			ui.View().
				Children(

					button("Information", pending, func() {
						show(native.AlertDialogOptions{
							Window:  window,
							Message: "QuickGUI uses a native alert dialog.",
							Detail:  "This alert is attached to the current window as a native sheet.",
						}, func(int) string { return "Information dialog dismissed." })
					}),
					button("Save warning", pending, func() {
						show(native.AlertDialogOptions{
							Window:  window,
							Level:   "warning",
							Message: "Save changes before closing?",
							Detail:  "Your edits will be lost if you close this document without saving.",
							Buttons: []native.AlertDialogButton{
								{Label: "Save", Role: "default"},
								{Label: "Don't Save"},
								{Label: "Cancel", Role: "cancel"},
							},
						}, func(index int) string {
							labels := []string{"Save", "Don't Save", "Cancel"}
							if index >= 0 && index < len(labels) {
								return "Save dialog result: " + labels[index]
							}
							return "Save dialog result: " + strconv.Itoa(index)
						})
					}),
					button("Critical", pending, func() {
						show(native.AlertDialogOptions{
							Level:   "critical",
							Message: "Delete this workspace?",
							Detail:  "This example does not delete anything; it only demonstrates an application-modal critical alert.",
							Buttons: []native.AlertDialogButton{
								{Label: "Delete", Role: "default"},
								{Label: "Cancel", Role: "cancel"},
							},
						}, func(index int) string {
							if index == 0 {
								return "Delete selected (no action taken)."
							}
							return "Delete cancelled."
						})
					}),
				).
				Display("flex").
				FlexWrap("wrap").
				Gap(10),
			dialogStatus(status, pending),

			panelStyle,
		)
}
