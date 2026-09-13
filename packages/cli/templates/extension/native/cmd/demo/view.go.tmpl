package main

import (
	extension "{{GO_MODULE}}"

	"github.com/egoist/quickgui/go/ui"
)

func App() *ui.Element {
	message, setMessage := ui.CreateSignal("Ready to call the {{TYPE}} extension")
	return ui.View().Children(
		ui.Text("{{NAME}}").FontSize(24).FontWeight(700),
		ui.Text(message()).FontSize(16),
		ui.Button().Child("Call extension").OnClick(func() {
			extension.Echo("Hello from {{TYPE}} through purego", func(reply string, err error) {
				if err != nil {
					setMessage(err.Error())
					return
				}
				setMessage(reply)
			})
		}).Padding(12).BorderRadius(8).BackgroundColor("#2563eb").TextColor("white").
			Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor("#3b82f6") }),
	).Display("flex").FlexDirection("column").JustifyContent("center").Height("100%").Padding(24).Gap(16)
}
