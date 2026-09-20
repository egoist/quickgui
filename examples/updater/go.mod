module quickgui.example/updater

go 1.23.0

require github.com/egoist/quickgui/go v0.1.6
require github.com/egoist/quickgui/extensions/updater v0.1.6
replace github.com/egoist/quickgui/extensions/updater => ../../extensions/updater
require github.com/ebitengine/purego v0.10.1 // indirect
replace github.com/egoist/quickgui/go => ../../go
