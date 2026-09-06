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
	return gui.Dialog.Root(gui.DialogRootProps{
		Open: func() bool { return true },
		OnOpenChange: func(open bool, _ gui.DialogOpenChangeDetails) {
			if !open {
				app.CloseDialog()
			}
		},
		ExitDuration: 0,
		Children: func() *native.Node {
			return gui.Dialog.Portal(gui.PartProps{
				Style: gui.Style{Position: "absolute", Top: 0, Right: 0, Bottom: 0, Left: 0, Display: "flex", AlignItems: "center", JustifyContent: "center"},
				Children: func() *native.Node {
					return gui.Fragment([]*native.Node{
						gui.Dialog.Backdrop(gui.PartProps{
							Style: gui.Style{Position: "absolute", Top: 0, Right: 0, Bottom: 0, Left: 0, BackgroundColor: app.Theme().Scrim},
						}),
						gui.Dialog.Popup(gui.DialogPopupProps{
							PartProps: gui.PartProps{
								Style: gui.Style{
									Display: "flex", FlexDirection: "column", Width: 440, Gap: 14, Padding: 20,
									BackgroundColor: app.Theme().Raised, BorderWidth: 1, BorderColor: app.Theme().BorderStrong, BorderRadius: 10,
								},
								Children: func() *native.Node {
									return gui.Fragment([]*native.Node{
										gui.Dialog.Title(gui.PartProps{
											Children: func() *native.Node {
												return gui.Text(gui.Props{Style: gui.Style{FontSize: 15, FontWeight: 700, Color: app.Theme().Text}, Children: title})
											},
										}),
										gui.Show(func() bool { return description != "" }, func() *native.Node {
											return gui.Dialog.Description(gui.PartProps{
												Children: func() *native.Node {
													return gui.Text(gui.Props{Style: gui.Style{FontSize: 12.5, LineHeight: 18, Color: app.Theme().TextSecondary}, Children: description})
												},
											})
										}),
										gui.Dialog.Viewport(gui.PartProps{
											Style:    gui.Style{Display: "flex", FlexDirection: "column", Gap: 12, Padding: 2},
											Children: func() *native.Node { return body },
										}),
										gui.View(gui.Props{
											Style: gui.Style{Display: "flex", FlexDirection: "row", JustifyContent: "flex-end", Gap: 8, MarginTop: 4},
											Children: []any{
												gui.Dialog.Close(gui.PartProps{
													Style: app.Theme().Button("secondary"),
													Children: func() *native.Node {
														return gui.Text(gui.Props{Children: "Cancel"})
													},
												}),
												actions,
											},
										}),
									})
								},
							},
						}),
					})
				},
			})
		},
	})
}

func CheckRow(label string, checked func() bool, onChange func(bool)) *native.Node {
	app := UseApp()
	return gui.Checkbox.Root(gui.CheckboxProps{
		PartProps: gui.PartProps{
			Style: gui.Style{
				Display: "flex", FlexDirection: "row", AlignItems: "center", Gap: 8, Height: 24,
				Cursor: "default", UserSelect: "none", BorderRadius: 4,
				Focus:    &gui.Style{Outline: "2px solid " + app.Theme().FocusRing},
				Disabled: &gui.Style{Opacity: 0.5},
			},
			Children: func() *native.Node {
				return gui.Fragment([]*native.Node{
					gui.Checkbox.Indicator(gui.PartProps{
						Style: func() gui.Style { return checkboxBox(app.Theme(), checked()) },
						Children: func() *native.Node {
							return gui.Show(checked, func() *native.Node { return checkboxMark(true) })
						},
					}),
					gui.Text(gui.Props{Style: gui.Style{FontSize: 12.5, Color: app.Theme().Text}, Children: label}),
				})
			},
		},
		Checked:         func() gui.CheckedState { return checked() },
		OnCheckedChange: func(next bool, _ *native.Event) { onChange(next) },
	})
}

func checkboxBox(theme Theme, checked bool) gui.Style {
	border := theme.InputBorder
	background := theme.Input
	if checked {
		border = theme.Accent
		background = theme.Accent
	}
	return gui.Style{
		Display: "flex", Width: 15, Height: 15, FlexShrink: 0, AlignItems: "center", JustifyContent: "center",
		BorderRadius: 3.5, BorderWidth: 1, BorderColor: border, BackgroundColor: background,
	}
}

func checkboxMark(checked bool) *native.Node {
	if !checked {
		return nil
	}
	return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: UseApp().Theme().TextOnAccent}, Children: "✓"})
}

func newBranchDialog() *native.Node {
	app := UseApp()
	store := app.Store
	name, setName := gui.CreateSignal("")
	checkout, setCheckout := gui.CreateSignal(true)
	initial := app.Dialog().From
	if initial == "" {
		if status := store.Status(); status != nil && status.Branch != "" {
			initial = status.Branch
		} else {
			initial = "HEAD"
		}
	}
	base, setBase := gui.CreateSignal(initial)
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
		store.CreateBranch(name(), base(), checkout())
	}
	options := func() []gui.OptionDeclaration {
		seen := map[string]struct{}{"HEAD": {}, base(): {}}
		items := []gui.OptionDeclaration{{Value: "HEAD", Label: "HEAD"}}
		add := func(value string) {
			if _, ok := seen[value]; ok || value == "" {
				return
			}
			seen[value] = struct{}{}
			items = append(items, gui.OptionDeclaration{Value: value, Label: value})
		}
		for _, branch := range store.Refs().Local {
			add(branch.Name)
		}
		for _, branch := range store.Refs().Remote {
			add(branch.Name)
		}
		return items
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
			gui.Text(gui.Props{Style: gui.Style{FontSize: 12, FontWeight: 600, Color: app.Theme().TextSecondary}, Children: "Based on"}),
			gui.Select.Root(gui.SelectRootProps{
				PickerSourceProps: gui.PickerSourceProps{
					PartProps: gui.PartProps{
						AriaLabel: "Base branch",
						Style:     []gui.Style{app.Theme().InputStyle(), {FlexDirection: "row", AlignItems: "center", JustifyContent: "space-between", Gap: 8}},
						Children: func() *native.Node {
							return gui.Text(gui.Props{Style: gui.Style{FontSize: 13, Color: app.Theme().Text}, Children: func() string { return base() }})
						},
					},
					Items: options,
				},
				Value: func() *string {
					value := base()
					return &value
				},
				OnValueChange: func(value *string, _ *native.Event) {
					if value != nil {
						setBase(*value)
					}
				},
			}),
			CheckRow("Switch to the new branch", checkout, setCheckout),
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
			CheckRow("Create a new branch", createNew, setCreateNew),
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
			CheckRow("Include untracked files", include, setInclude),
		},
	}), gui.Button(gui.Props{OnClick: func(*native.Event) { submit() }, Style: app.Theme().Button("primary"), Children: "Stash"}))
}
