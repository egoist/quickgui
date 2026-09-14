package main

import (
	"log"
	"quickgui.example/native-extension/echo"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		native.NewWindow(native.WindowOptions{
			Title:     "Native extension",
			Width:     480,
			Height:    260,
			Component: App,
		})
	}); err != nil {
		log.Fatal(err)
	}
}

func App() *ui.Element {
	message, setMessage := ui.CreateSignal("Hello from an independent native library")
	return ui.View().
		Children(

			ui.Text(message()),
			ui.Button().
				Child("Call extension").
				OnClick(func() {
					echo.Send("The stock core loaded @acme/extension-echo 1.0.0", func(reply string, err error) {
						if err != nil {
							setMessage(err.Error())
							return
						}
						setMessage(reply)
					})
				}).
				Padding(12).
				BackgroundColor("#2563eb").
				TextColor("white").
				BorderRadius(8),
		).
		Display("flex").
		FlexDirection("column").
		Padding(24).
		Gap(16)
}
