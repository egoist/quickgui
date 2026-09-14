package ui

import (
	"github.com/egoist/quickgui/go/native"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/git"
)

func Dialogs() *native.Node {
	app := UseApp()
	return gui.Show(
		app.Dialog().Kind != DialogNone,
		func() *native.Node {
			var children []*native.Node
			switch app.Dialog().Kind {
			case DialogNewBranch:
				children = append(children, newBranchDialog())
				return gui.Fragment(children)
			case DialogNewWorktree:
				children = append(children, newWorktreeDialog())
				return gui.Fragment(children)
			case DialogStash:
				children = append(children, stashDialog())
				return gui.Fragment(children)
			default:
				return gui.Fragment(children)
			}
			return gui.Fragment(children)
		},
	)
}
func dialogFrame(title, description string, body *gui.Element, actions *gui.Element) *native.Node {
	app := UseApp()
	dialog67 := gui.NewDialog(gui.DialogRootProps{
		Open: func() bool {
			return true
		},
		OnOpenChange: func(open bool, _ gui.DialogOpenChangeDetails) {
			if !open {
				app.CloseDialog()
			}
		},
		ExitDuration: 0,
	})
	return dialog67.Root().
		Children(func() *native.Node {
			return dialog67.Portal(gui.PartProps{Style: gui.Style().
				Position("absolute").
				Top(0).
				Right(0).
				Bottom(0).
				Left(0).
				Display("flex").
				AlignItems("center").
				JustifyContent("center")}).
				Children(func() *native.Node {
					return gui.Fragment([]*native.Node{
						dialog67.Backdrop(gui.PartProps{Style: gui.Style().
							Position("absolute").
							Top(0).
							Right(0).
							Bottom(0).
							Left(0).
							BackgroundColor(app.Theme().Scrim)}).
							NativeNode(),
						dialog67.Popup(gui.DialogPopupProps{PartProps: gui.PartProps{Style: gui.Style().
							Display("flex").
							FlexDirection("column").
							Width(440).
							Gap(14).
							Padding(20).
							BackgroundColor(app.Theme().Raised).
							BorderWidth(1).
							BorderColor(app.Theme().BorderStrong).
							BorderRadius(10)}}).
							Children(func() *native.Node {
								return gui.Fragment([]*native.Node{
									dialog67.Title(gui.PartProps{}).
										Children(func() *gui.Element {
											return gui.Text(title).
												FontSize(15).
												FontWeight(700).
												TextColor(app.Theme().Text)
										}).
										NativeNode(),
									gui.Show(
										description != "",
										func() *native.Node {
											return dialog67.Description(gui.PartProps{}).
												Children(func() *gui.Element {
													return gui.Text(description).
														FontSize(12.5).
														LineHeight(18).
														TextColor(app.Theme().TextSecondary)
												}).
												NativeNode()
										},
									),
									dialog67.Viewport(gui.PartProps{Style: gui.Style().
										Display("flex").
										FlexDirection("column").
										Gap(12).
										Padding(2)}).
										Children(body).
										NativeNode(),
									gui.View().
										Children(
											dialog67.Close(gui.PartProps{Style: app.Theme().Button("secondary")}).
												Children(func() *gui.Element {
													return gui.Text("Cancel")
												}).
												NativeNode(),
											actions,
										).
										Display("flex").
										FlexDirection("row").
										JustifyContent("flex-end").
										Gap(8).
										MarginTop(4).Node,
								})
							}).
							NativeNode(),
					})
				}).
				NativeNode()
		}).
		NativeNode()
}
func CheckRow(label string, checked func() bool, onChange func(bool)) *native.Node {
	app := UseApp()
	checkbox68 := gui.NewCheckbox(gui.CheckboxProps{
		PartProps: gui.PartProps{Style: gui.Style().
			Display("flex").
			FlexDirection("row").
			AlignItems("center").
			Gap(8).
			Height(24).
			Cursor("default").
			UserSelect("none").
			BorderRadius(4).
			FocusStyle(func(s gui.StyleBuilder) gui.StyleBuilder {
				return s.Outline("2px solid " + app.Theme().FocusRing)
			}).
			DisabledStyle(func(s gui.StyleBuilder) gui.StyleBuilder {
				return s.Opacity(0.5)
			})},
		Checked: func() gui.CheckedState {
			return checked()
		},
		OnCheckedChange: func(next bool, _ *native.Event) {
			onChange(next)
		},
	})
	return checkbox68.Root().
		Children(func() *native.Node {
			return gui.Fragment([]*native.Node{
				checkbox68.Indicator(gui.PartProps{Style: func() gui.StyleBuilder {
					return checkboxBox(app.Theme(), checked())
				}}).
					Children(func() *native.Node {
						return gui.Show(
							checked,
							func() *gui.Element {
								return checkboxMark(true)
							},
						)
					}).
					NativeNode(),
				gui.Text(label).FontSize(12.5).TextColor(app.Theme().Text).Node,
			})
		}).
		NativeNode()
}
func checkboxBox(theme Theme, checked bool) gui.StyleBuilder {
	border := theme.InputBorder
	background := theme.Input
	if checked {
		border = theme.Accent
		background = theme.Accent
	}
	return gui.Style().
		Display("flex").
		Width(15).
		Height(15).
		FlexShrink(0).
		AlignItems("center").
		JustifyContent("center").
		BorderRadius(3.5).
		BorderWidth(1).
		BorderColor(border).
		BackgroundColor(background)
}
func checkboxMark(checked bool) *gui.Element {
	if !checked {
		return nil
	}
	app := UseApp()
	return icon(checkIcon, 12, func() string {
		return app.Theme().TextOnAccent
	})
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
	problem := func() string {
		return git.BranchNameProblem(name())
	}
	exists := func() bool {
		for _, branch := range store.Refs().Local {
			if branch.Name == name() {
				return true
			}
		}
		return false
	}
	valid := func() bool {
		return name() != "" && problem() == "" && !exists()
	}
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
	return dialogFrame("New Branch", "", gui.View().
		Children(
			gui.Text("Name").FontSize(12).FontWeight(600).TextColor(app.Theme().TextSecondary),
			gui.Input().
				Style(app.Theme().InputStyle()).
				Placeholder("feature/great-idea").
				Value(name()).
				OnInputEvent(func(event *native.Event) {
					setName(event.Value)
				}).
				OnSubmitEvent(func(*native.Event) {
					submit()
				}),
			gui.Show(
				problem() != "" || exists(),
				func() *gui.Element {
					text := problem()
					if text == "" {
						text = "A branch with this name already exists."
					}
					return gui.Text(text).FontSize(11.5).TextColor(app.Theme().Danger)
				},
			),
			gui.Text("Based on").FontSize(12).FontWeight(600).TextColor(app.Theme().TextSecondary),
			func() *native.Node {
				select69 := gui.NewSelect(gui.SelectRootProps{
					PickerSourceProps: gui.PickerSourceProps{
						PartProps: gui.PartProps{
							AriaLabel: "Base branch",
							Style: gui.Style().
								Merge(app.Theme().InputStyle()).
								FlexDirection("row").
								AlignItems("center").
								JustifyContent("space-between").
								Gap(8),
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
				})
				return select69.Root().
					Children(func() *gui.Element {
						return gui.Text(base()).FontSize(13).TextColor(app.Theme().Text)
					}).
					NativeNode()
			}(),
			CheckRow("Switch to the new branch", checkout, setCheckout),
		).
		Display("flex").
		FlexDirection("column").
		Gap(10), gui.Button().
		Style(app.Theme().Button("primary")).
		Child(func() string {
			if checkout() {
				return "Create and Switch"
			}
			return "Create"
		}).
		Disabled(!valid()).
		OnClick(func() {
			submit()
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
	return dialogFrame("New Worktree", "Adds a linked working tree beside this repository.", gui.View().
		Children(
			CheckRow("Create a new branch", createNew, setCreateNew),
			gui.Text("Branch").FontSize(12).FontWeight(600).TextColor(app.Theme().TextSecondary),
			gui.Input().
				Style(app.Theme().InputStyle()).
				Value(branch()).
				OnInputEvent(func(event *native.Event) {
					setBranch(event.Value)
					setPath(store.SuggestWorktreePath(event.Value))
				}),
			gui.Show(
				problem() != "",
				func() *gui.Element {
					return gui.Text(problem()).FontSize(11.5).TextColor(app.Theme().Danger)
				},
			),
			gui.Text("Path").FontSize(12).FontWeight(600).TextColor(app.Theme().TextSecondary),
			gui.Input().
				Style(app.Theme().InputStyle()).
				Value(path()).
				OnInputEvent(func(event *native.Event) {
					setPath(event.Value)
				}),
		).
		Display("flex").
		FlexDirection("column").
		Gap(10), gui.Button().
		Style(app.Theme().Button("primary")).
		Child("Add Worktree").
		OnClick(func() {
			submit()
		}))
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
	return dialogFrame("Stash Changes", "Saves local changes and returns the working tree to HEAD.", gui.View().
		Children(
			gui.Input().
				Style(app.Theme().InputStyle()).
				Placeholder("Optional message").
				Value(message()).
				OnInputEvent(func(event *native.Event) {
					setMessage(event.Value)
				}).
				OnSubmitEvent(func(*native.Event) {
					submit()
				}),
			CheckRow("Include untracked files", include, setInclude),
		).
		Display("flex").
		FlexDirection("column").
		Gap(10), gui.Button().
		Style(app.Theme().Button("primary")).
		Child("Stash").
		OnClick(func() {
			submit()
		}))
}
