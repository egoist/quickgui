package ui

import (
	"fmt"
	"path/filepath"
	"time"

	"github.com/egoist/quickgui/examples/quick-git-go/internal/git"
	"github.com/egoist/quickgui/examples/quick-git-go/internal/model"
	"github.com/egoist/quickgui/packages/go/native"
	gui "github.com/egoist/quickgui/packages/go/ui"
)

func ChangesView() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, MinHeight: 0, FlexDirection: "row"},
		Children: []any{
			gui.View(gui.Props{
				Style: gui.Style{Display: "flex", Width: store.ChangesSplit(), FlexShrink: 0, MinHeight: 0, FlexDirection: "column"},
				Children: []any{
					gui.Show(func() bool { return store.ChangeCount() > 0 }, func() *native.Node {
						return gui.Fragment([]*native.Node{fileList(model.ListUnstaged), fileList(model.ListStaged)})
					}, func() *native.Node {
						return emptyState("No local changes", "The working tree matches the last commit.")
					}),
					commitComposer(),
				},
			}),
			gui.View(gui.Props{
				AriaLabel: "Resize file list",
				OnClick: func(*native.Event) {
					width := store.ChangesSplit()
					if width < 420 {
						store.SetChangesSplit(width + 40)
					} else {
						store.SetChangesSplit(320)
					}
				},
				Style: gui.Style{Width: 1, FlexShrink: 0, Cursor: "col-resize", AppRegion: "no-drag", BackgroundColor: app.Theme().Border},
			}),
			DiffPane(),
		},
	})
}

func fileList(list model.ListID) *native.Node {
	app := UseApp()
	store := app.Store
	label := "Unstaged"
	if list == model.ListStaged {
		label = "Staged"
	}
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", FlexDirection: "column", MinHeight: 0, Flex: 1, FlexBasis: 0},
		Children: []any{
			gui.View(gui.Props{
				Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Height: 32, FlexShrink: 0, PaddingLeft: 12, PaddingRight: 8, Gap: 6},
				Children: []any{
					gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 600, Color: app.Theme().TextTertiary}, Children: label}),
					gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: func() string {
						return fmt.Sprintf("%d", len(store.ListItems(list)))
					}}),
					gui.View(gui.Props{Style: gui.Style{Flex: 1}}),
					gui.Show(func() bool { return len(store.ListItems(list)) > 0 }, func() *native.Node {
						text := "Stage All"
						if list == model.ListStaged {
							text = "Unstage All"
						}
						return gui.Button(gui.Props{
							Disabled: store.Busy() != nil,
							OnClick: func(*native.Event) {
								if list == model.ListUnstaged {
									store.StageAll()
								} else {
									store.UnstageAll()
								}
							},
							Style:    app.Theme().Button("secondary"),
							Children: text,
						})
					}),
				},
			}),
			gui.Show(func() bool { return len(store.ListItems(list)) > 0 }, func() *native.Node {
				return gui.View(gui.Props{
					Style: gui.Style{Display: "flex", Flex: 1, MinHeight: 0, FlexDirection: "column", OverflowY: "auto"},
					Children: gui.For(func() []git.ChangeItem { return store.ListItems(list) }, func(item git.ChangeItem, index func() int) *native.Node {
						return changeRow(list, item, index())
					}, func(item git.ChangeItem) any { return item.ID }, nil),
				})
			}, func() *native.Node {
				empty := "No unstaged changes"
				if list == model.ListStaged {
					empty = "Nothing staged yet"
				}
				return gui.Text(gui.Props{Style: gui.Style{PaddingLeft: 12, PaddingBottom: 10, FontSize: 12, Color: app.Theme().TextTertiary}, Children: empty})
			}),
		},
	})
}

func changeRow(list model.ListID, item git.ChangeItem, index int) *native.Node {
	app := UseApp()
	store := app.Store
	selected := func() bool {
		ranges := store.Selection().Unstaged
		if list == model.ListStaged {
			ranges = store.Selection().Staged
		}
		for _, r := range ranges {
			if len(r) >= 2 && index >= r[0] && index <= r[1] {
				return true
			}
		}
		return false
	}
	stats := store.Numstat().Unstaged
	if list == model.ListStaged {
		stats = store.Numstat().Staged
	}
	counts := ""
	if entry, ok := stats[item.Path]; ok {
		if entry.Added != nil {
			counts += fmt.Sprintf("+%d", *entry.Added)
		}
		if entry.Removed != nil {
			if counts != "" {
				counts += " "
			}
			counts += fmt.Sprintf("-%d", *entry.Removed)
		}
	}
	return gui.Button(gui.Props{
		Selected: selected(),
		OnClick:  func(*native.Event) { store.SelectChange(list, index) },
		OnDoubleClick: func(*native.Event) {
			store.SelectChange(list, index)
			store.ToggleStaging(list)
		},
		OnContextMenu: func(*native.Event) { changeMenu(app, list, item) },
		Style:         rowStyle(app.Theme(), selected()),
		Children: []any{
			gui.Button(gui.Props{
				OnClick: func(*native.Event) {
					if list == model.ListUnstaged {
						store.StageItems([]git.ChangeItem{item})
					} else {
						store.UnstageItems([]git.ChangeItem{item})
					}
				},
				Style:    app.Theme().IconButton(),
				Children: checkboxGlyph(list == model.ListStaged),
			}),
			gui.Text(gui.Props{Style: gui.Style{Width: 16, FontWeight: 700, Color: StatusColor(app.Theme(), string(item.Code))}, Children: string(item.Code)}),
			gui.Text(gui.Props{Style: gui.Style{Flex: 1, MinWidth: 0, FontSize: 13, LineClamp: 1}, Children: item.Path}),
			gui.Show(func() bool { return counts != "" }, func() *native.Node {
				return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontFamily: "monospace", Color: app.Theme().TextTertiary}, Children: counts})
			}),
		},
	})
}

func checkboxGlyph(checked bool) string {
	if checked {
		return "☑"
	}
	return "☐"
}

func changeMenu(app AppContext, list model.ListID, item git.ChangeItem) {
	store := app.Store
	store.SelectChange(list, indexOf(store.ListItems(list), item.ID))
	targets := store.SelectedItems(list)
	if len(targets) == 0 {
		targets = []git.ChangeItem{item}
	}
	absolute := filepath.Join(store.Repository().Root(), item.Path)
	items := []native.MenuItem{}
	if list == model.ListUnstaged {
		items = append(items, native.MenuItem{Label: "Stage File", Click: func() { store.StageItems(targets) }})
		items = append(items, native.MenuItem{Label: "Discard Changes…", Click: func() { confirmDiscard(app, targets) }})
	} else {
		items = append(items, native.MenuItem{Label: "Unstage File", Click: func() { store.UnstageItems(targets) }})
	}
	items = append(items,
		native.MenuItem{Type: "separator"},
		native.MenuItem{Label: "Reveal in Finder", Click: func() { native.ShowItemInFolder(absolute, func(error) {}) }},
		native.MenuItem{Label: "Copy Path", Click: func() { native.WriteClipboardText(item.Path) }},
	)
	native.PopupMenu(app.Window, items, nil, nil, func(error) {})
}

func indexOf(items []git.ChangeItem, id string) int {
	for i, item := range items {
		if item.ID == id {
			return i
		}
	}
	return 0
}

func confirmDiscard(app AppContext, items []git.ChangeItem) {
	message := fmt.Sprintf("Discard changes to %d files?", len(items))
	if len(items) == 1 {
		message = "Discard changes to " + filepath.Base(items[0].Path) + "?"
	}
	native.ShowAlertDialog(native.AlertDialogOptions{
		Message: message, Detail: "The changes cannot be recovered.", Level: "warning",
		Buttons: []native.AlertDialogButton{{Label: "Cancel"}, {Label: "Discard", Role: "destructive"}},
		Window:  app.Window,
	}, func(index int, err error) {
		if err == nil && index == 1 {
			app.Store.DiscardItems(items)
		}
	})
}

func commitComposer() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", FlexDirection: "column", FlexShrink: 0, Gap: 8, Padding: 12, BorderWidth: 1, BorderColor: app.Theme().Border},
		Children: []any{
			gui.Input(gui.Props{
				Placeholder: "Commit summary",
				Value:       func() string { return store.Subject() },
				OnInput:     func(event *native.Event) { store.SetSubject(event.Value) },
				OnSubmit:    func(*native.Event) { store.Commit() },
				Style:       app.Theme().InputStyle(),
			}),
			gui.TextArea(gui.Props{
				Placeholder: "Description",
				Value:       func() string { return store.Body() },
				OnInput:     func(event *native.Event) { store.SetBody(event.Value) },
				Style: gui.Style{
					Display: "flex", Width: "100%", MinHeight: 72, PaddingLeft: 7, PaddingRight: 7, PaddingTop: 4, PaddingBottom: 4,
					BackgroundColor: app.Theme().Input, Color: app.Theme().Text, BorderWidth: 1, BorderColor: app.Theme().InputBorder, BorderRadius: 6, FontSize: UIFontSize,
				},
			}),
			gui.View(gui.Props{
				Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Gap: 8},
				Children: []any{
					gui.Button(gui.Props{
						OnClick: func(*native.Event) { store.SetAmend(!store.Amend()) },
						Style:   app.Theme().Button("secondary"),
						Children: func() string {
							if store.Amend() {
								return "☑ Amend"
							}
							return "☐ Amend"
						},
					}),
					gui.View(gui.Props{Style: gui.Style{Flex: 1}}),
					gui.Show(func() bool { return len(store.Agents()) > 0 }, func() *native.Node {
						return gui.Button(gui.Props{
							Disabled: store.Generating() != nil,
							OnClick:  func(*native.Event) { store.GenerateMessage("") },
							Style:    app.Theme().Button("secondary"),
							Children: func() string {
								if store.Generating() != nil {
									return "Generating…"
								}
								return "Generate"
							},
						})
					}),
					gui.Button(gui.Props{
						Disabled: !store.CanCommit(),
						OnClick:  func(*native.Event) { store.Commit() },
						Style:    app.Theme().Button("primary"),
						Children: "Commit",
					}),
				},
			}),
		},
	})
}

func DiffPane() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, MinHeight: 0, FlexDirection: "column", BackgroundColor: app.Theme().Content},
		Children: gui.Show(func() bool { return store.Diff().Target != nil }, func() *native.Node {
			return gui.Fragment([]*native.Node{
				gui.View(gui.Props{
					Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Gap: 8, Height: 40, FlexShrink: 0, PaddingLeft: 14, PaddingRight: 10, BorderWidth: 1, BorderColor: app.Theme().Border},
					Children: []any{
						gui.Text(gui.Props{Style: gui.Style{Flex: 1, MinWidth: 0, FontSize: 13, FontWeight: 600, LineClamp: 1}, Children: func() string {
							if target := store.Diff().Target; target != nil {
								return target.Path
							}
							return ""
						}}),
						gui.Show(func() bool { return store.DiffStats().Added > 0 }, func() *native.Node {
							return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 700, Color: app.Theme().Success, FontFamily: "monospace"}, Children: fmt.Sprintf("+%d", store.DiffStats().Added)})
						}),
						gui.Show(func() bool { return store.DiffStats().Removed > 0 }, func() *native.Node {
							return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 700, Color: app.Theme().Danger, FontFamily: "monospace"}, Children: fmt.Sprintf("-%d", store.DiffStats().Removed)})
						}),
						gui.Show(func() bool { return store.Diff().Loading }, func() *native.Node {
							return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: "Loading…"})
						}),
						diffActions(),
					},
				}),
				gui.Show(func() bool { return store.Diff().Error != "" }, func() *native.Node {
					return gui.Text(gui.Props{Style: gui.Style{Padding: 12, Color: app.Theme().Danger, FontSize: 12}, Children: store.Diff().Error})
				}),
				gui.View(gui.Props{
					Style: gui.Style{Display: "flex", Flex: 1, MinHeight: 0, FlexDirection: "column", OverflowY: "auto", FontFamily: "monospace", FontSize: MonoFontSize},
					Children: gui.For(store.DiffRows, func(row git.DiffRow, index func() int) *native.Node {
						return diffRow(row, index())
					}, func(row git.DiffRow) any {
						return fmt.Sprintf("%s:%d:%d:%d", row.Kind, row.FileIndex, row.HunkIndex, row.LineIndex)
					}, nil),
				}),
			})
		}, func() *native.Node {
			title := "Select a file"
			if store.View() == model.ViewHistory {
				title = "Select a commit"
			}
			return emptyState(title, "Choose a change to review its diff.")
		}),
	})
}

func diffActions() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.Show(func() bool { return store.Diff().Target != nil && store.Diff().Target.Kind != "commit" }, func() *native.Node {
		mode := store.Diff().Target.Kind
		lineCount := store.SelectedDiffLineCount()
		return gui.View(gui.Props{
			Style: gui.Style{Display: "flex", FlexDirection: "row", Gap: 6},
			Children: gui.Show(func() bool { return lineCount > 0 }, func() *native.Node {
				if mode == "staged" {
					return gui.Button(gui.Props{OnClick: func(*native.Event) { store.ApplySelectedLines("unstage") }, Style: app.Theme().Button("primary"), Children: fmt.Sprintf("Unstage %d Lines", lineCount)})
				}
				return gui.Fragment([]*native.Node{
					gui.Button(gui.Props{OnClick: func(*native.Event) { store.ApplySelectedLines("discard") }, Style: app.Theme().Button("danger"), Children: "Discard Lines…"}),
					gui.Button(gui.Props{OnClick: func(*native.Event) { store.ApplySelectedLines("stage") }, Style: app.Theme().Button("primary"), Children: fmt.Sprintf("Stage %d Lines", lineCount)}),
				})
			}, func() *native.Node {
				if mode == "staged" {
					return gui.Button(gui.Props{OnClick: func(*native.Event) {
						if item := store.ActiveItem(); item != nil {
							store.UnstageItems([]git.ChangeItem{*item})
						}
					}, Style: app.Theme().Button("secondary"), Children: "Unstage File"})
				}
				return gui.Fragment([]*native.Node{
					gui.Button(gui.Props{OnClick: func(*native.Event) {
						if item := store.ActiveItem(); item != nil {
							confirmDiscard(app, []git.ChangeItem{*item})
						}
					}, Style: app.Theme().Button("danger"), Children: "Discard…"}),
					gui.Button(gui.Props{OnClick: func(*native.Event) {
						if item := store.ActiveItem(); item != nil {
							store.StageItems([]git.ChangeItem{*item})
						}
					}, Style: app.Theme().Button("primary"), Children: "Stage File"}),
				})
			}),
		})
	})
}

func diffRow(row git.DiffRow, index int) *native.Node {
	app := UseApp()
	store := app.Store
	selected := false
	for _, r := range store.DiffSelection() {
		if len(r) >= 2 && index >= r[0] && index <= r[1] {
			selected = true
		}
	}
	background := "transparent"
	color := app.Theme().Text
	switch row.Kind {
	case "hunk":
		background = app.Theme().DiffHunk
		color = app.Theme().DiffHunkText
	case "notice", "file":
		color = app.Theme().TextTertiary
	default:
		switch row.LineKind {
		case git.LineAdded:
			background = app.Theme().DiffAdded
			color = app.Theme().DiffAddedText
		case git.LineRemoved:
			background = app.Theme().DiffRemoved
			color = app.Theme().DiffRemovedText
		}
	}
	if selected && row.Kind == "line" {
		if row.LineKind == git.LineAdded {
			background = "#bfe9cb"
		} else if row.LineKind == git.LineRemoved {
			background = "#f7c7c3"
		} else {
			background = app.Theme().SelectionMuted
		}
	}
	oldNo, newNo := " ", " "
	if row.OldLineNumber != nil {
		oldNo = fmt.Sprintf("%d", *row.OldLineNumber)
	}
	if row.NewLineNumber != nil {
		newNo = fmt.Sprintf("%d", *row.NewLineNumber)
	}
	children := []any{
		gui.Text(gui.Props{Style: gui.Style{Width: 36, Color: app.Theme().DiffLineNumber, TextAlign: "right"}, Children: oldNo}),
		gui.Text(gui.Props{Style: gui.Style{Width: 36, Color: app.Theme().DiffLineNumber, TextAlign: "right"}, Children: newNo}),
		gui.Text(gui.Props{Style: gui.Style{Flex: 1, MinWidth: 0, Color: color, WhiteSpace: "pre"}, Children: row.Text}),
	}
	if row.Kind == "hunk" && store.Diff().Target != nil && store.Diff().Target.Kind != "commit" {
		action := "Stage hunk"
		if store.Diff().Target.Kind == "staged" {
			action = "Unstage hunk"
		}
		children = append(children, gui.Button(gui.Props{
			OnClick: func(*native.Event) {
				if store.Diff().Target.Kind == "staged" {
					store.UnstageHunk(row.FileIndex, row.HunkIndex)
				} else {
					store.StageHunk(row.FileIndex, row.HunkIndex)
				}
			},
			Style:    app.Theme().Button("secondary"),
			Children: action,
		}))
	}
	return gui.Button(gui.Props{
		OnClick:  func(*native.Event) { store.SetDiffSelection([][]int{{index, index}}) },
		Style:    gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Gap: 8, MinHeight: 20, PaddingLeft: 8, PaddingRight: 8, BackgroundColor: background, Cursor: "default"},
		Children: children,
	})
}

func HistoryView() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, MinHeight: 0, FlexDirection: "row"},
		Children: []any{
			gui.View(gui.Props{
				Style: gui.Style{Display: "flex", Width: store.HistorySplit(), FlexShrink: 0, MinHeight: 0, FlexDirection: "column"},
				Children: []any{
					gui.View(gui.Props{
						Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Height: 32, FlexShrink: 0, PaddingLeft: 12, PaddingRight: 8, Gap: 8, BorderWidth: 1, BorderColor: app.Theme().Border},
						Children: []any{
							gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 700, TextTransform: "uppercase", Color: app.Theme().TextTertiary}, Children: func() string {
								if store.History().AllBranches {
									return "All branches"
								}
								if status := store.Status(); status != nil && status.Branch != "" {
									return status.Branch
								}
								return "History"
							}}),
							gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: func() string {
								suffix := ""
								if !store.History().Exhausted {
									suffix = "+"
								}
								return fmt.Sprintf("%d%s commits", len(store.History().Commits), suffix)
							}}),
							gui.View(gui.Props{Style: gui.Style{Flex: 1}}),
							gui.Button(gui.Props{
								OnClick: func(*native.Event) { store.SetHistoryAllBranches(!store.History().AllBranches) },
								Style:   app.Theme().Button("secondary"),
								Children: func() string {
									if store.History().AllBranches {
										return "☑ All branches"
									}
									return "☐ All branches"
								},
							}),
						},
					}),
					gui.Show(func() bool { return len(store.History().Commits) > 0 }, func() *native.Node {
						return gui.View(gui.Props{
							Style: gui.Style{Display: "flex", Flex: 1, MinHeight: 0, FlexDirection: "column", OverflowY: "auto"},
							Children: gui.For(func() []git.Commit { return store.History().Commits }, func(commit git.Commit, index func() int) *native.Node {
								i := index()
								selected := func() bool {
									for _, r := range store.HistorySelection() {
										if len(r) >= 2 && i >= r[0] && i <= r[1] {
											return true
										}
									}
									return false
								}
								lane := ""
								graph := store.History().Graph
								if i < len(graph) {
									lane = fmt.Sprintf("•%d", graph[i].Lane)
								}
								return gui.Button(gui.Props{
									Selected: selected(),
									OnClick:  func(*native.Event) { store.SelectCommit(i) },
									OnContextMenu: func(*native.Event) {
										native.PopupMenu(app.Window, []native.MenuItem{
											{Label: "Copy SHA", Click: func() { native.WriteClipboardText(commit.Sha) }},
											{Label: "New Branch from Here…", Click: func() { app.OpenDialog(DialogRequest{Kind: DialogNewBranch, From: commit.Sha}) }},
											{Label: "Checkout (Detached)", Click: func() { store.CheckoutCommit(commit.Sha) }},
										}, nil, nil, func(error) {})
									},
									Style: rowStyle(app.Theme(), selected()),
									Children: []any{
										gui.Text(gui.Props{Style: gui.Style{Width: 28, Color: app.Theme().Accent, FontFamily: "monospace"}, Children: lane}),
										gui.View(gui.Props{
											Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, FlexDirection: "column"},
											Children: []any{
												gui.Text(gui.Props{Style: gui.Style{FontSize: 13, LineClamp: 1}, Children: commit.Subject}),
												gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: commit.ShortSha + " · " + commit.AuthorName + " · " + git.RelativeTime(commit.AuthorTime, time.Now())}),
											},
										}),
									},
								})
							}, func(commit git.Commit) any { return commit.Sha }, nil),
						})
					}, func() *native.Node {
						return emptyState("No commits", "This repository has no history yet.")
					}),
				},
			}),
			gui.View(gui.Props{
				OnClick: func(*native.Event) {
					width := store.HistorySplit()
					if width < 520 {
						store.SetHistorySplit(width + 40)
					} else {
						store.SetHistorySplit(420)
					}
				},
				Style: gui.Style{Width: 1, FlexShrink: 0, Cursor: "col-resize", AppRegion: "no-drag", BackgroundColor: app.Theme().Border},
			}),
			gui.View(gui.Props{
				Style:    gui.Style{Display: "flex", Flex: 1, MinWidth: 0, MinHeight: 0, FlexDirection: "column"},
				Children: []any{commitDetail(), DiffPane()},
			}),
		},
	})
}

func commitDetail() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", FlexDirection: "column", FlexShrink: 0, MaxHeight: 220, BorderWidth: 1, BorderColor: app.Theme().Border},
		Children: gui.Show(func() bool { return store.SelectedCommit() != nil }, func() *native.Node {
			return gui.Fragment([]*native.Node{
				gui.View(gui.Props{
					Style: gui.Style{Padding: 12, Display: "flex", FlexDirection: "column", Gap: 4},
					Children: []any{
						gui.Text(gui.Props{Style: gui.Style{FontSize: 14, FontWeight: 700}, Children: func() string {
							if commit := store.SelectedCommit(); commit != nil {
								return commit.Subject
							}
							return ""
						}}),
						gui.Text(gui.Props{Style: gui.Style{FontSize: 12, Color: app.Theme().TextSecondary}, Children: func() string {
							if commit := store.SelectedCommit(); commit != nil {
								return commit.AuthorName + " · " + git.AbsoluteTime(commit.AuthorTime) + " · " + commit.ShortSha
							}
							return ""
						}}),
					},
				}),
				gui.View(gui.Props{
					Style: gui.Style{Display: "flex", FlexDirection: "column", OverflowY: "auto", MaxHeight: 140},
					Children: gui.For(func() []git.CommitFile { return store.CommitDetail().Files }, func(file git.CommitFile, _ func() int) *native.Node {
						return gui.Button(gui.Props{
							OnClick: func(*native.Event) { store.SelectCommitFile(file.Path) },
							Style:   rowStyle(app.Theme(), store.CommitDetail().SelectedPath == file.Path),
							Children: []any{
								gui.Text(gui.Props{Style: gui.Style{Width: 16, FontWeight: 700, Color: StatusColor(app.Theme(), file.Status)}, Children: file.Status}),
								gui.Text(gui.Props{Style: gui.Style{Flex: 1, MinWidth: 0, LineClamp: 1}, Children: file.Path}),
							},
						})
					}, func(file git.CommitFile) any { return file.Path }, nil),
				}),
			})
		}, func() *native.Node { return emptyState("Select a commit", "Its files appear here.") }),
	})
}

func BranchesView() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", Flex: 1, MinHeight: 0, FlexDirection: "column", OverflowY: "auto", Padding: 12, Gap: 4},
		Children: []any{
			gui.View(gui.Props{
				Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Height: 32, Gap: 8},
				Children: []any{
					gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 700, TextTransform: "uppercase", Color: app.Theme().TextTertiary, Flex: 1}, Children: "Local branches"}),
					gui.Button(gui.Props{OnClick: func(*native.Event) { app.OpenDialog(DialogRequest{Kind: DialogNewBranch}) }, Style: app.Theme().Button("primary"), Children: "New Branch"}),
				},
			}),
			gui.For(func() []git.BranchRef { return store.Refs().Local }, func(branch git.BranchRef, _ func() int) *native.Node {
				return gui.Button(gui.Props{
					OnClick: func(*native.Event) {},
					OnDoubleClick: func(*native.Event) {
						if !branch.Current {
							store.SwitchBranch(branch.Name)
						}
					},
					OnContextMenu: func(*native.Event) {
						native.PopupMenu(app.Window, []native.MenuItem{
							{Label: "Switch to Branch", Click: func() { store.SwitchBranch(branch.Name) }},
							{Label: "New Branch from Here…", Click: func() { app.OpenDialog(DialogRequest{Kind: DialogNewBranch, From: branch.Name}) }},
							{Label: "Copy Branch Name", Click: func() { native.WriteClipboardText(branch.Name) }},
							{Type: "separator"},
							{Label: "Delete Branch…", Click: func() { store.DeleteBranch(branch.Name, true) }},
						}, nil, nil, func(error) {})
					},
					Style: rowStyle(app.Theme(), branch.Current),
					Children: []any{
						gui.Text(gui.Props{Style: gui.Style{Flex: 1, FontWeight: ternary(branch.Current, 700, 500), LineClamp: 1}, Children: branch.Name}),
						gui.Show(func() bool { return branch.Current }, func() *native.Node {
							return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().Accent}, Children: "current"})
						}),
						gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: branch.ShortSha}),
					},
				})
			}, func(branch git.BranchRef) any { return branch.FullName }, nil),
			gui.Show(func() bool { return len(store.Refs().Remote) > 0 }, func() *native.Node {
				return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 700, TextTransform: "uppercase", Color: app.Theme().TextTertiary, MarginTop: 16}, Children: "Remote branches"})
			}),
			gui.For(func() []git.BranchRef { return store.Refs().Remote }, func(branch git.BranchRef, _ func() int) *native.Node {
				return gui.View(gui.Props{
					Style: rowStyle(app.Theme(), false),
					Children: []any{
						gui.Text(gui.Props{Style: gui.Style{Flex: 1, LineClamp: 1, Color: app.Theme().TextSecondary}, Children: branch.Name}),
						gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: branch.ShortSha}),
					},
				})
			}, func(branch git.BranchRef) any { return branch.FullName }, nil),
		},
	})
}

func WorktreesView() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", Flex: 1, MinHeight: 0, FlexDirection: "column", OverflowY: "auto", Padding: 12, Gap: 4},
		Children: []any{
			gui.View(gui.Props{
				Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Height: 32},
				Children: []any{
					gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 700, TextTransform: "uppercase", Color: app.Theme().TextTertiary, Flex: 1}, Children: "Worktrees"}),
					gui.Button(gui.Props{OnClick: func(*native.Event) { app.OpenDialog(DialogRequest{Kind: DialogNewWorktree}) }, Style: app.Theme().Button("primary"), Children: "New Worktree"}),
				},
			}),
			gui.For(store.Worktrees, func(worktree git.Worktree, _ func() int) *native.Node {
				active := store.Repository() != nil && store.Repository().Root() == worktree.Path
				label := worktree.BranchName
				if label == "" {
					label = filepath.Base(worktree.Path)
				}
				return gui.Button(gui.Props{
					OnClick: func(*native.Event) { store.SelectWorktree(worktree.Path) },
					OnContextMenu: func(*native.Event) {
						items := []native.MenuItem{
							{Label: "Reveal in Finder", Click: func() { native.ShowItemInFolder(worktree.Path, func(error) {}) }},
							{Label: "Copy Path", Click: func() { native.WriteClipboardText(worktree.Path) }},
						}
						if !worktree.Main {
							items = append(items, native.MenuItem{Type: "separator"}, native.MenuItem{Label: "Remove Worktree", Click: func() { store.RemoveWorktree(worktree.Path, false) }})
						}
						native.PopupMenu(app.Window, items, nil, nil, func(error) {})
					},
					Style: rowStyle(app.Theme(), active),
					Children: []any{
						gui.Text(gui.Props{Style: gui.Style{Flex: 1, LineClamp: 1}, Children: label}),
						gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: filepath.Base(worktree.Path)}),
					},
				})
			}, func(worktree git.Worktree) any { return worktree.Path }, nil),
		},
	})
}

func StashesView() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", Flex: 1, MinHeight: 0, FlexDirection: "column", OverflowY: "auto", Padding: 12, Gap: 4},
		Children: []any{
			gui.View(gui.Props{
				Style: gui.Style{Display: "flex", FlexDirection: "row", AlignItems: "center", Height: 32},
				Children: []any{
					gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontWeight: 700, TextTransform: "uppercase", Color: app.Theme().TextTertiary, Flex: 1}, Children: "Stashes"}),
					gui.Button(gui.Props{Disabled: store.ChangeCount() == 0, OnClick: func(*native.Event) { app.OpenDialog(DialogRequest{Kind: DialogStash}) }, Style: app.Theme().Button("primary"), Children: "Stash Changes"}),
				},
			}),
			gui.Show(func() bool { return len(store.Stashes()) > 0 }, func() *native.Node {
				return gui.For(store.Stashes, func(stash git.StashEntry, _ func() int) *native.Node {
					return gui.Button(gui.Props{
						OnContextMenu: func(*native.Event) {
							native.PopupMenu(app.Window, []native.MenuItem{
								{Label: "Apply", Click: func() { store.StashApply(stash.Ref) }},
								{Label: "Pop", Click: func() { store.StashPop(stash.Ref) }},
								{Label: "Drop", Click: func() { store.StashDrop(stash.Ref) }},
							}, nil, nil, func(error) {})
						},
						Style: rowStyle(app.Theme(), false),
						Children: []any{
							gui.View(gui.Props{
								Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, FlexDirection: "column"},
								Children: []any{
									gui.Text(gui.Props{Style: gui.Style{LineClamp: 1}, Children: stash.Summary}),
									gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary}, Children: stash.Ref + " · " + git.RelativeTime(stash.Time, time.Now())}),
								},
							}),
							gui.Button(gui.Props{OnClick: func(*native.Event) { store.StashApply(stash.Ref) }, Style: app.Theme().Button("secondary"), Children: "Apply"}),
							gui.Button(gui.Props{OnClick: func(*native.Event) { store.StashPop(stash.Ref) }, Style: app.Theme().Button("secondary"), Children: "Pop"}),
						},
					})
				}, func(stash git.StashEntry) any { return stash.Ref }, nil)
			}, func() *native.Node { return emptyState("No stashes", "Stash local changes to save them for later.") }),
		},
	})
}

func emptyState(title, description string) *native.Node {
	app := UseApp()
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", Flex: 1, MinHeight: 0, FlexDirection: "column", AlignItems: "center", JustifyContent: "center", Gap: 8, Padding: 32},
		Children: []any{
			gui.Text(gui.Props{Style: gui.Style{FontSize: 15, FontWeight: 500, Color: app.Theme().TextTertiary, TextAlign: "center"}, Children: title}),
			gui.Text(gui.Props{Style: gui.Style{FontSize: 12, LineHeight: 17, Color: app.Theme().TextTertiary, TextAlign: "center", MaxWidth: 360}, Children: description}),
		},
	})
}

func ternary[T any](cond bool, a, b T) T {
	if cond {
		return a
	}
	return b
}
