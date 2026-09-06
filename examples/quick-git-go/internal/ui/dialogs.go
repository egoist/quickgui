package ui

import (
	"github.com/egoist/quickgui/examples/quick-git-go/internal/git"
	"github.com/egoist/quickgui/packages/go/native"
	gui "github.com/egoist/quickgui/packages/go/ui"
)

func Dialogs() *native.Node {
	app := UseApp()
	return gui.Show(func() bool { return app.Dialog().Kind != DialogNone }, func() *native.Node {
		switch app.Dialog().Kind {
		case DialogNewBranch:
			return newBranchDialog()
		case DialogNewWorktree:
			return newWorktreeDialog()
		case DialogStash:
			return stashDialog()
		default:
			return nil
		}
	})
}

func dialogFrame(title, description string, body *native.Node, actions *native.Node) *native.Node {
	app := UseApp()
	return gui.View(gui.Props{
		Style: gui.Style{Position: "absolute", Top: 0, Right: 0, Bottom: 0, Left: 0, Display: "flex", AlignItems: "center", JustifyContent: "center"},
		Children: []any{
			gui.Button(gui.Props{
				OnClick: func(*native.Event) { app.CloseDialog() },
				Style:   gui.Style{Position: "absolute", Top: 0, Right: 0, Bottom: 0, Left: 0, BackgroundColor: app.Theme().Scrim},
			}),
			gui.View(gui.Props{
				Style: gui.Style{
					Display: "flex", FlexDirection: "column", Width: 440, Gap: 14, Padding: 20,
					BackgroundColor: app.Theme().Raised, BorderWidth: 1, BorderColor: app.Theme().BorderStrong, BorderRadius: 10,
				},
				Children: []any{
					gui.Text(gui.Props{Style: gui.Style{FontSize: 15, FontWeight: 700, Color: app.Theme().Text}, Children: title}),
					gui.Show(func() bool { return description != "" }, func() *native.Node {
						return gui.Text(gui.Props{Style: gui.Style{FontSize: 12.5, LineHeight: 18, Color: app.Theme().TextSecondary}, Children: description})
					}),
					body,
					gui.View(gui.Props{
						Style:    gui.Style{Display: "flex", FlexDirection: "row", JustifyContent: "flex-end", Gap: 8, MarginTop: 4},
						Children: []any{gui.Button(gui.Props{OnClick: func(*native.Event) { app.CloseDialog() }, Style: app.Theme().Button("secondary"), Children: "Cancel"}), actions},
					}),
				},
			}),
		},
	})
}

func newBranchDialog() *native.Node {
	app := UseApp()
	store := app.Store
	name, setName := gui.CreateSignal("")
	checkout, setCheckout := gui.CreateSignal(true)
	base := app.Dialog().From
	if base == "" {
		if status := store.Status(); status != nil && status.Branch != "" {
			base = status.Branch
		} else {
			base = "HEAD"
		}
	}
	problem := func() string { return git.BranchNameProblem(name()) }
	exists := func() bool {
		for _, branch := range store.Refs().Local {
			if branch.Name == name() {
				return true
			}
		}
		return false
	}
	valid := func() bool { return name() != "" && problem() == "" && !exists() }
	submit := func() {
		if !valid() {
			return
		}
		app.CloseDialog()
		store.CreateBranch(name(), base, checkout())
	}
	return dialogFrame("New Branch", "", gui.View(gui.Props{
		Style: gui.Style{Display: "flex", FlexDirection: "column", Gap: 10},
		Children: []any{
			gui.Text(gui.Props{Style: gui.Style{FontSize: 12, FontWeight: 600, Color: app.Theme().TextSecondary}, Children: "Name"}),
			gui.Input(gui.Props{Placeholder: "feature/great-idea", Value: func() string { return name() }, OnInput: func(event *native.Event) { setName(event.Value) }, OnSubmit: func(*native.Event) { submit() }, Style: app.Theme().InputStyle()}),
			gui.Show(func() bool { return problem() != "" || exists() }, func() *native.Node {
				text := problem()
				if text == "" {
					text = "A branch with this name already exists."
				}
				return gui.Text(gui.Props{Style: gui.Style{FontSize: 11.5, Color: app.Theme().Danger}, Children: text})
			}),
			gui.Text(gui.Props{Style: gui.Style{FontSize: 12, Color: app.Theme().TextTertiary}, Children: "Starting from " + base}),
			gui.Button(gui.Props{
				OnClick: func(*native.Event) { setCheckout(!checkout()) },
				Style:   app.Theme().Button("secondary"),
				Children: func() string {
					if checkout() {
						return "☑ Check out after creating"
					}
					return "☐ Check out after creating"
				},
			}),
		},
	}), gui.Button(gui.Props{
		Disabled: !valid(),
		OnClick:  func(*native.Event) { submit() },
		Style:    app.Theme().Button("primary"),
		Children: func() string {
			if checkout() {
				return "Create and Switch"
			}
			return "Create"
		},
	}))
}

func newWorktreeDialog() *native.Node {
	app := UseApp()
	store := app.Store
	initialBranch := app.Dialog().Branch
	branch, setBranch := gui.CreateSignal(initialBranch)
	createNew, setCreateNew := gui.CreateSignal(initialBranch == "")
	path, setPath := gui.CreateSignal(store.SuggestWorktreePath(initialBranch))
	problem := func() string {
		if !createNew() {
			return ""
		}
		return git.BranchNameProblem(branch())
	}
	submit := func() {
		if createNew() && problem() != "" {
			return
		}
		app.CloseDialog()
		if createNew() {
			store.AddWorktree(path(), branch(), "", "")
			return
		}
		store.AddWorktree(path(), "", branch(), "")
	}
	return dialogFrame("New Worktree", "Adds a linked working tree beside this repository.", gui.View(gui.Props{
		Style: gui.Style{Display: "flex", FlexDirection: "column", Gap: 10},
		Children: []any{
			gui.Button(gui.Props{
				OnClick: func(*native.Event) { setCreateNew(!createNew()) },
				Style:   app.Theme().Button("secondary"),
				Children: func() string {
					if createNew() {
						return "☑ Create a new branch"
					}
					return "☐ Create a new branch"
				},
			}),
			gui.Text(gui.Props{Style: gui.Style{FontSize: 12, FontWeight: 600, Color: app.Theme().TextSecondary}, Children: "Branch"}),
			gui.Input(gui.Props{
				Value: func() string { return branch() },
				OnInput: func(event *native.Event) {
					setBranch(event.Value)
					setPath(store.SuggestWorktreePath(event.Value))
				},
				Style: app.Theme().InputStyle(),
			}),
			gui.Show(func() bool { return problem() != "" }, func() *native.Node {
				return gui.Text(gui.Props{Style: gui.Style{FontSize: 11.5, Color: app.Theme().Danger}, Children: problem()})
			}),
			gui.Text(gui.Props{Style: gui.Style{FontSize: 12, FontWeight: 600, Color: app.Theme().TextSecondary}, Children: "Path"}),
			gui.Input(gui.Props{Value: func() string { return path() }, OnInput: func(event *native.Event) { setPath(event.Value) }, Style: app.Theme().InputStyle()}),
		},
	}), gui.Button(gui.Props{OnClick: func(*native.Event) { submit() }, Style: app.Theme().Button("primary"), Children: "Add Worktree"}))
}

func stashDialog() *native.Node {
	app := UseApp()
	store := app.Store
	message, setMessage := gui.CreateSignal("")
	include, setInclude := gui.CreateSignal(true)
	submit := func() {
		app.CloseDialog()
		store.StashPush(message(), include())
	}
	return dialogFrame("Stash Changes", "Saves local changes and returns the working tree to HEAD.", gui.View(gui.Props{
		Style: gui.Style{Display: "flex", FlexDirection: "column", Gap: 10},
		Children: []any{
			gui.Input(gui.Props{Placeholder: "Optional message", Value: func() string { return message() }, OnInput: func(event *native.Event) { setMessage(event.Value) }, OnSubmit: func(*native.Event) { submit() }, Style: app.Theme().InputStyle()}),
			gui.Button(gui.Props{
				OnClick: func(*native.Event) { setInclude(!include()) },
				Style:   app.Theme().Button("secondary"),
				Children: func() string {
					if include() {
						return "☑ Include untracked files"
					}
					return "☐ Include untracked files"
				},
			}),
		},
	}), gui.Button(gui.Props{OnClick: func(*native.Event) { submit() }, Style: app.Theme().Button("primary"), Children: "Stash"}))
}
