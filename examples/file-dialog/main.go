package main

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	run(native.WindowOptions{
		Title:         "QuickGUI File Dialogs",
		Width:         720,
		Height:        520,
		MinimumWidth:  560,
		MinimumHeight: 440,
	}, FileDialogs)
}

func FileDialogs() *ui.Element {
	window := native.CurrentWindow()
	status, setStatus := ui.CreateSignal("Choose an open or save dialog.")
	pending, setPending := ui.CreateSignal(false)
	start := func() bool {
		if pending() {
			return false
		}
		setPending(true)
		setStatus("Waiting for the native file dialog…")
		return true
	}
	complete := func(message string, err error) {
		if window.Closed {
			return
		}
		setPending(false)
		if err != nil {
			setStatus("Dialog failed: " + err.Error())
		} else {
			setStatus(message)
		}
	}
	directory := func() string {
		path, err := os.Getwd()
		if err != nil {
			path, _ = os.UserHomeDir()
		}
		return path
	}
	return ui.View().
		Children(

			ui.Text(
				"Open and save",
			).
				FontSize(26).
				LineHeight(32).
				FontWeight(700),
			ui.Text(
				"Native file panels return selected paths and cancellation state. Choosing a save destination does not write a file.",
			).
				TextColor("#9aa6b7").
				FontSize(14).
				LineHeight(21),
			ui.View().
				Children(

					button("Open files", pending, func() {
						if !start() {
							return
						}
						native.ShowOpenDialog(
							native.OpenDialogOptions{
								Window:      window,
								Title:       "Open text files",
								DefaultPath: directory(),
								Filters: []native.FileDialogFilter{
									{Name: "Text", Extensions: []string{"txt", "md"}},
									{Name: "All files", Extensions: []string{"*"}},
								},
								Properties: []string{"openFile", "multiSelections"},
							},
							func(result native.OpenDialogResult, err error) {
								message := "Open dialog canceled."
								if !result.Canceled && err == nil {
									message = "Selected " + strconv.Itoa(len(result.FilePaths)) + ": " + strings.Join(result.FilePaths, ", ")
								}
								complete(message, err)
							},
						)
					}),
					button("Open folder", pending, func() {
						if !start() {
							return
						}
						native.ShowOpenDialog(
							native.OpenDialogOptions{
								Title:       "Choose a folder",
								DefaultPath: directory(),
								Properties:  []string{"openDirectory"},
							},
							func(result native.OpenDialogResult, err error) {
								message := "Folder dialog canceled."
								if !result.Canceled && len(result.FilePaths) > 0 {
									message = "Selected folder: " + result.FilePaths[0]
								}
								complete(message, err)
							},
						)
					}),
					button("Save file", pending, func() {
						if !start() {
							return
						}
						native.ShowSaveDialog(
							native.SaveDialogOptions{
								Window:      window,
								Title:       "Choose a save destination",
								DefaultPath: filepath.Join(directory(), "quickgui-example.txt"),
								Filters:     []native.FileDialogFilter{{Name: "Text", Extensions: []string{"txt"}}},
							},
							func(result native.SaveDialogResult, err error) {
								message := "Save dialog canceled."
								if !result.Canceled && err == nil {
									message = "Save destination: " + result.FilePath + " (no file was written)"
								}
								complete(message, err)
							},
						)
					}),
				).
				Display("flex").
				FlexWrap("wrap").
				Gap(10),
			dialogStatus(status, pending),

			panelStyle,
		).
		MaxWidth(520)
}
