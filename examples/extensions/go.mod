module quickgui.example/extensions

go 1.23.0

require (
	github.com/egoist/quickgui/extensions/editor v0.1.6
	github.com/egoist/quickgui/extensions/markdown v0.1.6
	github.com/egoist/quickgui/extensions/terminal v0.1.6
	github.com/egoist/quickgui/go v0.1.6
)

require github.com/ebitengine/purego v0.10.1 // indirect

replace github.com/egoist/quickgui/go => ../../go

replace github.com/egoist/quickgui/extensions/editor => ../../extensions/editor

replace github.com/egoist/quickgui/extensions/markdown => ../../extensions/markdown

replace github.com/egoist/quickgui/extensions/terminal => ../../extensions/terminal
