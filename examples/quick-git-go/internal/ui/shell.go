package ui

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/egoist/quickgui/examples/quick-git-go/internal/git"
	"github.com/egoist/quickgui/examples/quick-git-go/internal/model"
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/reactive"
	gui "github.com/egoist/quickgui/packages/go/ui"
)

type toast struct {
	ID          int
	Type        string
	Title       string
	Description string
}

func App(store *model.Store, appearance reactive.Accessor[string], openRepository, openPath func(string)) func() *native.Node {
	return func() *native.Node {
		window := native.CurrentWindow()
		theme := gui.CreateMemo(func() Theme { return ThemeFor(appearance()) })
		dialog, setDialog := gui.CreateSignal(DialogRequest{})
		toasts, setToasts := gui.CreateSignal([]toast{})
		nextToast := 0
		store.SetNotifier(func(notice model.Notice) {
			nextToast++
			id := nextToast
			entry := toast{ID: id, Type: notice.Type, Title: notice.Title, Description: notice.Description}
			setToasts(append(toasts(), entry))
			timeout := notice.Timeout
			if timeout <= 0 {
				timeout = 4500
			}
			time.AfterFunc(time.Duration(timeout)*time.Millisecond, func() {
				native.Dispatch(func() {
					var next []toast
					for _, item := range toasts() {
						if item.ID != id {
							next = append(next, item)
						}
					}
					setToasts(next)
				})
			})
		})
		return ProvideApp(AppContext{
			Store: store, Window: window, Theme: theme,
			Dialog: dialog, OpenDialog: setDialog, CloseDialog: func() { setDialog(DialogRequest{}) },
			OpenRepository:     func() { openRepository("") },
			OpenRepositoryPath: func(path string) { openPath(path) },
		}, func() *native.Node {
			return shell(toasts, setToasts)
		})
	}
}

func shell(toasts reactive.Accessor[[]toast], setToasts reactive.Setter[[]toast]) *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{
			Position: "relative", Display: "flex", FlexDirection: "row",
			Width: "100%", Height: "100%", MinWidth: 0, MinHeight: 0, BackgroundColor: "transparent", Color: app.Theme().Text, FontSize: UIFontSize,
		},
		Children: []any{
			gui.Show(func() bool { return store.Repository() != nil }, func() *native.Node {
				return gui.Fragment([]*native.Node{
					gui.View(gui.Props{
						Style: gui.Style{
							Display: "flex", FlexDirection: "column", Height: "100%", MinWidth: 0, MinHeight: 0,
							Width: store.SidebarWidth(), BackgroundColor: app.Theme().SidebarWash,
						},
						Children: Sidebar(),
					}),
					gui.View(gui.Props{
						AriaLabel: "Resize sidebar",
						OnClick: func(*native.Event) {
							width := store.SidebarWidth()
							if width < 300 {
								store.SetSidebarWidth(width + 24)
							} else {
								store.SetSidebarWidth(236)
							}
						},
						Style: gui.Style{Width: 1, FlexShrink: 0, Cursor: "col-resize", AppRegion: "no-drag", BackgroundColor: app.Theme().Border},
					}),
					gui.View(gui.Props{
						Style:    gui.Style{Display: "flex", Flex: 1, MinWidth: 0, MinHeight: 0, FlexDirection: "column", BackgroundColor: app.Theme().Content},
						Children: []any{Toolbar(), mainView()},
					}),
				})
			}, func() *native.Node { return Welcome() }),
			Dialogs(),
			notices(toasts, setToasts),
		},
	})
}

func mainView() *native.Node {
	store := UseApp().Store
	return gui.Show(func() bool { return store.View() == model.ViewChanges }, func() *native.Node {
		return ChangesView()
	}, func() *native.Node {
		return gui.Show(func() bool { return store.View() == model.ViewHistory }, func() *native.Node {
			return HistoryView()
		}, func() *native.Node {
			return gui.Show(func() bool { return store.View() == model.ViewBranches }, func() *native.Node {
				return BranchesView()
			}, func() *native.Node {
				return gui.Show(func() bool { return store.View() == model.ViewWorktrees }, func() *native.Node {
					return WorktreesView()
				}, func() *native.Node { return StashesView() })
			})
		})
	})
}

func notices(toasts reactive.Accessor[[]toast], setToasts reactive.Setter[[]toast]) *native.Node {
	app := UseApp()
	return gui.View(gui.Props{
		Style: gui.Style{Position: "absolute", Right: 16, Bottom: 16, Width: 340, Display: "flex", FlexDirection: "column", Gap: 8},
		Children: gui.For(func() []toast { return toasts() }, func(entry toast, _ func() int) *native.Node {
			color := app.Theme().Accent
			switch entry.Type {
			case "error":
				color = app.Theme().Danger
			case "success":
				color = app.Theme().Success
			case "warning":
				color = app.Theme().Warning
			}
			return gui.View(gui.Props{
				Style: gui.Style{
					Display: "flex", FlexDirection: "row", AlignItems: "flex-start", Gap: 10,
					PaddingLeft: 12, PaddingRight: 8, PaddingTop: 10, PaddingBottom: 10,
					BackgroundColor: app.Theme().Raised, BorderWidth: 1, BorderColor: app.Theme().BorderStrong, BorderRadius: 8,
				},
				Children: []any{
					gui.View(gui.Props{Style: gui.Style{Width: 3, AlignSelf: "stretch", BorderRadius: 2, BackgroundColor: color}}),
					gui.View(gui.Props{
						Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, FlexDirection: "column", Gap: 2},
						Children: []any{
							gui.Text(gui.Props{Style: gui.Style{FontSize: 12.5, FontWeight: 700, Color: app.Theme().Text, LineClamp: 2}, Children: entry.Title}),
							gui.Show(func() bool { return entry.Description != "" }, func() *native.Node {
								return gui.Text(gui.Props{Style: gui.Style{FontSize: 12, LineHeight: 16, Color: app.Theme().TextSecondary, LineClamp: 4}, Children: entry.Description})
							}),
						},
					}),
					gui.Button(gui.Props{
						OnClick: func(*native.Event) {
							var next []toast
							for _, item := range toasts() {
								if item.ID != entry.ID {
									next = append(next, item)
								}
							}
							setToasts(next)
						},
						Style:    app.Theme().IconButton(),
						Children: "×",
					}),
				},
			})
		}, func(entry toast) any { return entry.ID }, nil),
	})
}

func Welcome() *native.Node {
	app := UseApp()
	store := app.Store
	home, _ := os.UserHomeDir()
	shorten := func(path string) string {
		if home != "" && strings.HasPrefix(path, home) {
			return "~" + path[len(home):]
		}
		return path
	}
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, MinHeight: 0, FlexDirection: "column", BackgroundColor: app.Theme().Content},
		Children: []any{
			gui.View(gui.Props{Style: gui.Style{Height: TitlebarHeight, FlexShrink: 0, AppRegion: "drag"}}),
			gui.View(gui.Props{
				Style: gui.Style{Display: "flex", Flex: 1, MinHeight: 0, FlexDirection: "column", AlignItems: "center", JustifyContent: "center", Gap: 20, Padding: 40},
				Children: []any{
					gui.View(gui.Props{
						Style: gui.Style{
							Display: "flex", Width: 64, Height: 64, AlignItems: "center", JustifyContent: "center",
							BorderRadius: 18, BackgroundColor: app.Theme().Accent, Color: app.Theme().TextOnAccent,
						},
						Children: gui.Text(gui.Props{Style: gui.Style{FontSize: 28, FontWeight: 700}, Children: "⌥"}),
					}),
					gui.Text(gui.Props{Style: gui.Style{FontSize: 22, FontWeight: 800, Color: app.Theme().Text}, Children: "Quick Git"}),
					gui.Text(gui.Props{Style: gui.Style{FontSize: 13, Color: app.Theme().TextSecondary, TextAlign: "center", LineHeight: 19}, Children: "Open a repository to review changes, history, and worktrees."}),
					gui.Button(gui.Props{
						OnClick:  func(*native.Event) { app.OpenRepository() },
						Style:    app.Theme().Button("primary"),
						Children: "Open Repository…",
					}),
					gui.Show(func() bool { return len(store.RecentRepositories()) > 0 }, func() *native.Node {
						return gui.View(gui.Props{
							Style: gui.Style{Display: "flex", FlexDirection: "column", Width: 420, MaxWidth: "100%", Gap: 2, MarginTop: 8},
							Children: []any{
								gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 700, LetterSpacing: 0.4, TextTransform: "uppercase", Color: app.Theme().TextTertiary, PaddingLeft: 10, MarginBottom: 4}, Children: "Recent"}),
								gui.For(func() []string {
									recent := store.RecentRepositories()
									if len(recent) > 8 {
										return recent[:8]
									}
									return recent
								}, func(path string, _ func() int) *native.Node {
									return gui.Button(gui.Props{
										Disabled: store.Opening() != "",
										OnClick:  func(*native.Event) { app.OpenRepositoryPath(path) },
										Style: gui.Style{
											Display: "flex", FlexDirection: "row", AlignItems: "center", Gap: 10, Height: 40,
											PaddingLeft: 10, PaddingRight: 10, BorderRadius: 8, BackgroundColor: "transparent", Cursor: "default",
											Hover: &gui.Style{BackgroundColor: app.Theme().Hover}, Disabled: &gui.Style{Opacity: 0.6},
										},
										Children: []any{
											gui.View(gui.Props{
												Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, FlexDirection: "column"},
												Children: []any{
													gui.Text(gui.Props{Style: gui.Style{FontSize: 13, FontWeight: 600, Color: app.Theme().Text, LineClamp: 1}, Children: filepath.Base(path)}),
													gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary, LineClamp: 1}, Children: shorten(filepath.Dir(path))}),
												},
											}),
											gui.Show(func() bool { return store.Opening() == path }, func() *native.Node {
												return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: "Opening…"})
											}),
										},
									})
								}, nil, nil),
							},
						})
					}),
				},
			}),
		},
	})
}

func Sidebar() *native.Node {
	app := UseApp()
	store := app.Store
	nav := []struct {
		ID    model.ViewID
		Label string
	}{
		{model.ViewChanges, "Changes"},
		{model.ViewHistory, "History"},
	}
	return gui.Fragment([]*native.Node{
		gui.View(gui.Props{Style: gui.Style{Display: "flex", Height: TitlebarHeight, FlexShrink: 0, AlignItems: "center", PaddingLeft: 84, PaddingRight: 10, AppRegion: "drag"}}),
		gui.View(gui.Props{
			Style: gui.Style{Display: "flex", Flex: 1, MinHeight: 0, FlexDirection: "column", Gap: 2, PaddingTop: 4, PaddingLeft: 10, PaddingRight: 10, PaddingBottom: 12, OverflowY: "auto"},
			Children: []any{
				gui.Button(gui.Props{
					AriaLabel: "Repository actions",
					OnClick:   func(*native.Event) { repositoryMenu(app) },
					Style: gui.Style{
						Display: "flex", FlexDirection: "row", AlignItems: "center", Gap: 9, Height: 44, FlexShrink: 0,
						PaddingLeft: 8, PaddingRight: 8, MarginBottom: 6, BorderRadius: 8, BackgroundColor: "transparent", Cursor: "default",
						Hover: &gui.Style{BackgroundColor: app.Theme().Hover},
					},
					Children: []any{
						gui.View(gui.Props{
							Style:    gui.Style{Display: "flex", Width: 28, Height: 28, FlexShrink: 0, AlignItems: "center", JustifyContent: "center", BorderRadius: 7, BackgroundColor: app.Theme().Accent, Color: app.Theme().TextOnAccent},
							Children: gui.Text(gui.Props{Children: "⌥"}),
						}),
						gui.View(gui.Props{
							Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, FlexDirection: "column", Gap: 1},
							Children: []any{
								gui.Text(gui.Props{Style: gui.Style{FontSize: 13, FontWeight: 700, Color: app.Theme().Text, LineClamp: 1}, Children: func() string { return store.RepositoryName() }}),
								gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary, LineClamp: 1}, Children: func() string {
									if status := store.Status(); status != nil {
										if status.Branch != "" {
											return status.Branch
										}
										if status.Detached {
											return "Detached HEAD"
										}
									}
									return ""
								}}),
							},
						}),
						gui.Text(gui.Props{Style: gui.Style{Color: app.Theme().TextTertiary}, Children: "▾"}),
					},
				}),
				gui.For(func() []struct {
					ID    model.ViewID
					Label string
				} {
					return nav
				}, func(item struct {
					ID    model.ViewID
					Label string
				}, _ func() int) *native.Node {
					return navRow(item.Label, func() bool { return store.View() == item.ID }, func() string {
						if item.ID == model.ViewChanges && store.ChangeCount() > 0 {
							return fmt.Sprintf("%d", store.ChangeCount())
						}
						return ""
					}, func() { store.SetView(item.ID) })
				}, func(item struct {
					ID    model.ViewID
					Label string
				}) any {
					return item.ID
				}, nil),
				sectionRow("Branches", func() int { return len(store.Refs().Local) }, func() bool { return store.View() == model.ViewBranches }, func() { store.SetView(model.ViewBranches) }, func() { app.OpenDialog(DialogRequest{Kind: DialogNewBranch}) }),
				gui.For(func() []git.BranchRef {
					local := store.Refs().Local
					if len(local) > 8 {
						return local[:8]
					}
					return local
				}, func(branch git.BranchRef, _ func() int) *native.Node {
					return navRow(branch.Name, func() bool { return false }, func() string {
						if branch.Current {
							return "✓"
						}
						return ""
					}, func() {
						if branch.Current {
							return
						}
						if branch.WorktreePath != "" && store.Repository() != nil && branch.WorktreePath != store.Repository().Root() {
							store.SelectWorktree(branch.WorktreePath)
							return
						}
						store.SwitchBranch(branch.Name)
					})
				}, func(branch git.BranchRef) any { return branch.FullName }, nil),
				sectionRow("Worktrees", func() int { return len(store.Worktrees()) }, func() bool { return store.View() == model.ViewWorktrees }, func() { store.SetView(model.ViewWorktrees) }, func() { app.OpenDialog(DialogRequest{Kind: DialogNewWorktree}) }),
				gui.For(store.Worktrees, func(worktree git.Worktree, _ func() int) *native.Node {
					label := worktree.BranchName
					if label == "" {
						if worktree.Detached && len(worktree.HeadSha) >= 7 {
							label = worktree.HeadSha[:7] + " (detached)"
						} else {
							label = filepath.Base(worktree.Path)
						}
					}
					return navRow(label, func() bool { return false }, func() string {
						if store.Repository() != nil && store.Repository().Root() == worktree.Path {
							return "✓"
						}
						return ""
					}, func() { store.SelectWorktree(worktree.Path) })
				}, func(worktree git.Worktree) any { return worktree.Path }, nil),
				sectionRow("Stashes", func() int { return len(store.Stashes()) }, func() bool { return store.View() == model.ViewStashes }, func() { store.SetView(model.ViewStashes) }, func() { app.OpenDialog(DialogRequest{Kind: DialogStash}) }),
				gui.For(func() []git.StashEntry {
					stashes := store.Stashes()
					if len(stashes) > 5 {
						return stashes[:5]
					}
					return stashes
				}, func(stash git.StashEntry, _ func() int) *native.Node {
					return navRow(stash.Summary, func() bool { return false }, func() string { return git.RelativeTime(stash.Time, time.Now()) }, func() { store.SetView(model.ViewStashes) })
				}, func(stash git.StashEntry) any { return stash.Ref }, nil),
			},
		}),
	})
}

func navRow(label string, selected func() bool, trailing func() string, onClick func()) *native.Node {
	app := UseApp()
	return gui.Button(gui.Props{
		AriaLabel: label,
		Selected:  selected(),
		OnClick:   func(*native.Event) { onClick() },
		Style:     rowStyle(app.Theme(), selected()),
		Children: []any{
			gui.Text(gui.Props{Style: gui.Style{Flex: 1, MinWidth: 0, FontSize: 13, LineClamp: 1}, Children: label}),
			gui.Show(func() bool { return trailing() != "" }, func() *native.Node {
				return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: trailing()})
			}),
		},
	})
}

func sectionRow(label string, count func() int, selected func() bool, onClick, action func()) *native.Node {
	app := UseApp()
	color := app.Theme().TextTertiary
	if selected() {
		color = app.Theme().Accent
	}
	return gui.View(gui.Props{
		Group: true,
		Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Gap: 4, MarginTop: 14, PaddingRight: 2},
		Children: []any{
			gui.Button(gui.Props{
				OnClick: func(*native.Event) { onClick() },
				Style: gui.Style{
					Display: "flex", Flex: 1, MinWidth: 0, FlexDirection: "row", AlignItems: "center", Gap: 6, Height: 22,
					PaddingLeft: 9, PaddingRight: 6, BorderRadius: 6, BackgroundColor: "transparent", Cursor: "default",
					Hover: &gui.Style{BackgroundColor: app.Theme().Hover},
				},
				Children: []any{
					gui.Text(gui.Props{Style: gui.Style{Flex: 1, MinWidth: 0, FontSize: 11, FontWeight: 600, Color: color}, Children: label}),
					gui.Show(func() bool { return count() > 0 }, func() *native.Node {
						return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 600, Color: app.Theme().TextTertiary}, Children: fmt.Sprintf("%d", count())})
					}),
				},
			}),
			gui.Button(gui.Props{OnClick: func(*native.Event) { action() }, Style: app.Theme().IconButton(), Children: "+"}),
		},
	})
}

func repositoryMenu(app AppContext) {
	store := app.Store
	recent := store.RecentRepositories()
	labels := RepositoryLabels(recent)
	current := ""
	if main := store.MainRepository(); main != nil {
		current = main.Root()
	}
	items := make([]native.MenuItem, 0, len(recent)+6)
	for _, path := range recent {
		p := path
		label := labels[path]
		if label == "" {
			label = filepath.Base(path)
		}
		items = append(items, native.MenuItem{Label: label, Checked: path == current, Click: func() {
			if p != current {
				app.OpenRepositoryPath(p)
			}
		}})
	}
	if len(recent) > 0 {
		items = append(items, native.MenuItem{Type: "separator"})
	}
	items = append(items,
		native.MenuItem{Label: "Open Repository…", Click: app.OpenRepository},
		native.MenuItem{Type: "separator"},
		native.MenuItem{Label: "Reveal in Finder", Click: func() {
			if repo := store.Repository(); repo != nil {
				native.ShowItemInFolder(repo.Root(), func(error) {})
			}
		}},
		native.MenuItem{Label: "Copy Path", Click: func() {
			if repo := store.Repository(); repo != nil {
				native.WriteClipboardText(repo.Root())
			}
		}},
		native.MenuItem{Type: "separator"},
		native.MenuItem{Label: "Close Repository", Click: store.CloseRepository},
	)
	native.PopupMenu(app.Window, items, nil, nil, func(error) {})
}

func Toolbar() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{
			Display: "flex", Height: TitlebarHeight, FlexShrink: 0, AlignItems: "center", Gap: 6,
			PaddingLeft: 14, PaddingRight: 12, BorderWidth: 1, BorderColor: app.Theme().Border, BackgroundColor: app.Theme().Content, AppRegion: "drag",
		},
		Children: []any{
			gui.Button(gui.Props{
				OnClick: func(*native.Event) { store.SetView(model.ViewBranches) },
				Style:   gui.Style{Display: "flex", Height: 22, AlignItems: "center", PaddingLeft: 8, PaddingRight: 8, BorderRadius: 6, AppRegion: "no-drag", Hover: &gui.Style{BackgroundColor: app.Theme().Hover}},
				Children: gui.Text(gui.Props{Children: func() string {
					if status := store.Status(); status != nil {
						if status.Branch != "" {
							return status.Branch
						}
						if status.Detached && len(status.HeadSha) >= 7 {
							return status.HeadSha[:7] + " (detached)"
						}
					}
					return "…"
				}}),
			}),
			gui.Show(func() bool {
				status := store.Status()
				return status != nil && status.HasUpstreamCounts && (status.Ahead > 0 || status.Behind > 0)
			}, func() *native.Node {
				status := store.Status()
				return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextSecondary, AppRegion: "no-drag"}, Children: fmt.Sprintf("↑%d ↓%d", status.Ahead, status.Behind)})
			}),
			gui.Show(func() bool { return store.Conflicts() > 0 }, func() *native.Node {
				return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 600, Color: app.Theme().Warning, AppRegion: "no-drag"}, Children: fmt.Sprintf("%d conflicted", store.Conflicts())})
			}),
			gui.View(gui.Props{Style: gui.Style{Flex: 1, AppRegion: "drag"}}),
			gui.Show(func() bool { return store.Busy() != nil }, func() *native.Node {
				return gui.View(gui.Props{
					Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Gap: 8, MarginRight: 8, AppRegion: "no-drag"},
					Children: []any{
						gui.Text(gui.Props{Style: gui.Style{FontSize: 12, Color: app.Theme().TextSecondary}, Children: store.Busy().Label + "…"}),
						gui.Show(func() bool { return store.Busy() != nil && store.Busy().Cancel != nil }, func() *native.Node {
							return gui.Button(gui.Props{OnClick: func(*native.Event) { store.CancelBusy() }, Style: app.Theme().Button("secondary"), Children: "Cancel"})
						}),
					},
				})
			}),
			gui.View(gui.Props{
				Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Gap: 6, AppRegion: "no-drag"},
				Children: []any{
					toolButton(app, "Fetch", func() { store.Fetch() }, store.Busy() != nil),
					toolButton(app, "Pull", func() { store.Pull() }, store.Busy() != nil || store.Status() == nil || store.Status().Upstream == ""),
					toolButton(app, "Push", func() { store.Push() }, store.Busy() != nil || store.Status() == nil || store.Status().Branch == ""),
					toolButton(app, "↻", func() { store.Refresh() }, store.Busy() != nil),
				},
			}),
		},
	})
}

func toolButton(app AppContext, label string, onClick func(), disabled bool) *native.Node {
	return gui.Button(gui.Props{
		Disabled: disabled,
		OnClick:  func(*native.Event) { onClick() },
		Style:    app.Theme().Button("secondary"),
		Children: label,
	})
}
