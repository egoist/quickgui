package ui

import (
	"github.com/egoist/quickgui/examples/quick-git-go/internal/model"
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/reactive"
	gui "github.com/egoist/quickgui/packages/go/ui"
)

type DialogKind string

const (
	DialogNone        DialogKind = ""
	DialogNewBranch   DialogKind = "new-branch"
	DialogNewWorktree DialogKind = "new-worktree"
	DialogStash       DialogKind = "stash"
)

type DialogRequest struct {
	Kind   DialogKind
	From   string
	Branch string
}

type AppContext struct {
	Store              *model.Store
	Window             *native.Window
	Theme              reactive.Accessor[Theme]
	Dialog             reactive.Accessor[DialogRequest]
	OpenDialog         func(DialogRequest)
	CloseDialog        func()
	OpenRepository     func()
	OpenRepositoryPath func(string)
}

var appContext = reactive.CreateContext[AppContext](AppContext{})

func ProvideApp(value AppContext, children func() *native.Node) *native.Node {
	return reactive.Provide(appContext, value, children)
}

func UseApp() AppContext {
	value := reactive.UseContext(appContext)
	if value.Store == nil {
		panic("useApp must run inside AppProvider")
	}
	return value
}

func boolPtr(value bool) *bool { return &value }

func rowStyle(theme Theme, selected bool) gui.Style {
	background := "transparent"
	if selected {
		background = theme.Selection
	}
	return gui.Style{
		Display: "flex", FlexDirection: "row", Width: "100%", MinWidth: 0, Height: 28, FlexShrink: 0,
		AlignItems: "center", Gap: 8, PaddingLeft: 10, PaddingRight: 8, BorderRadius: 6,
		BackgroundColor: background, Color: theme.Text, Cursor: "default", UserSelect: "none",
		Hover: &gui.Style{BackgroundColor: theme.Hover}, Active: &gui.Style{BackgroundColor: theme.Active},
	}
}

func RepositoryLabels(paths []string) map[string]string {
	counts := map[string]int{}
	for _, path := range paths {
		counts[model.Basename(path)]++
	}
	labels := map[string]string{}
	for _, path := range paths {
		name := model.Basename(path)
		if counts[name] > 1 {
			labels[path] = model.Basename(model.Dirname(path)) + "/" + name
		} else {
			labels[path] = name
		}
	}
	return labels
}
