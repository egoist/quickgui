package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
)

func main() {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:     "{{NAME}} demo",
				Width:     540,
				Height:    320,
				Component: App,
			})
		}
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				open()
			}
		})
		open()
	}); err != nil {
		log.Fatal(err)
	}
}
