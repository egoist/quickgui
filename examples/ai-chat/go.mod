module quickgui.example/ai-chat

go 1.23.0

require github.com/egoist/quickgui/go v0.1.6

require github.com/egoist/quickgui/extensions/markdown v0.1.6

require github.com/egoist/quickgui/extensions/editor v0.1.6

replace github.com/egoist/quickgui/extensions/editor => ../../extensions/editor

replace github.com/egoist/quickgui/extensions/markdown => ../../extensions/markdown

require github.com/ebitengine/purego v0.10.1 // indirect

replace github.com/egoist/quickgui/go => ../../go
