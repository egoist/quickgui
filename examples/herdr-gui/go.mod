module quickgui.example/herdr-gui

go 1.23.0

require github.com/egoist/quickgui/go v0.1.6
require github.com/egoist/quickgui/extensions/terminal v0.1.6
replace github.com/egoist/quickgui/extensions/terminal => ../../extensions/terminal

require github.com/ebitengine/purego v0.10.1 // indirect

replace github.com/egoist/quickgui/go => ../../go
