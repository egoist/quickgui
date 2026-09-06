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

type visibleItem[T any] struct {
	Index int
	Item  T
}

func visibleWindow[T any](items []T, window gui.VisibleRange) []visibleItem[T] {
	start, end := window.Start, window.End
	if start < 0 {
		start = 0
	}
	if end > len(items) {
		end = len(items)
	}
	if start > end {
		start = end
	}
	rows := make([]visibleItem[T], 0, end-start)
	for index := start; index < end; index++ {
		rows = append(rows, visibleItem[T]{Index: index, Item: items[index]})
	}
	return rows
}

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
				return changeTable(list)
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

func changeTable(list model.ListID) *native.Node {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.ChangeItem] {
		return visibleWindow(store.ListItems(list), visibleRange())
	}
	selection := func() []gui.TableRowRange {
		ranges := store.Selection().Unstaged
		if list == model.ListStaged {
			ranges = store.Selection().Staged
		}
		return ranges
	}
	return gui.Table.Root(gui.TableRootProps{
		PartProps: gui.PartProps{
			Style: gui.Style{Flex: 1, MinHeight: 0, OverflowY: "scroll"},
			Children: func() *native.Node {
				return gui.KeyedFor(visible, func(row visibleItem[git.ChangeItem]) any { return row.Index }, func(row func() visibleItem[git.ChangeItem], _ func() int) *native.Node {
					return gui.Table.Row(gui.TableRowProps{
						Index: func() float64 { return float64(row().Index) },
						PartProps: gui.PartProps{
							Style: gui.Style{Hover: &gui.Style{BackgroundColor: app.Theme().Hover}, Selected: &gui.Style{BackgroundColor: app.Theme().Selection}},
							Children: func() *native.Node {
								return gui.Fragment([]*native.Node{
									gui.Table.Cell(gui.TableCellProps{
										Column: "toggle",
										PartProps: gui.PartProps{Children: func() *native.Node {
											return stageToggle(list, func() git.ChangeItem { return row().Item })
										}},
									}),
									gui.Table.Cell(gui.TableCellProps{
										Column: "status",
										PartProps: gui.PartProps{Children: func() *native.Node {
											return gui.Text(gui.Props{Style: gui.Style{Width: 16, FontWeight: 700, Color: StatusColor(app.Theme(), string(row().Item.Code))}, Children: func() string { return string(row().Item.Code) }})
										}},
									}),
									gui.Table.Cell(gui.TableCellProps{
										Column: "name",
										PartProps: gui.PartProps{
											Style:         gui.Style{PaddingRight: 8, MinWidth: 0},
											OnContextMenu: func(*native.Event) { changeMenu(app, list, row().Item) },
											OnDoubleClick: func(*native.Event) { store.ToggleStaging(list) },
											Children:      func() *native.Node { return changeName(list, func() git.ChangeItem { return row().Item }) },
										},
									}),
								})
							},
						},
					})
				}, nil)
			},
		},
		Columns: func() []gui.TableColumnDeclaration {
			return []gui.TableColumnDeclaration{
				{ID: "toggle", Track: "40px", Align: "center"},
				{ID: "status", Track: "24px", Align: "center"},
				{ID: "name", Track: "1fr", RowHeader: true},
			}
		},
		RowCount:      func() float64 { return float64(len(store.ListItems(list))) },
		RowHeight:     28,
		HeaderHeight:  0,
		SelectionMode: "multiple",
		Selection:     selection,
		OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
			setVisibleRange(next)
		},
		OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
			store.SetSelection(list, ranges)
		},
		OnActivate: func(_ gui.TableCell, _ *native.Event) {
			store.ToggleStaging(list)
		},
	})
}

func stageToggle(list model.ListID, item func() git.ChangeItem) *native.Node {
	app := UseApp()
	store := app.Store
	checked := list == model.ListStaged
	focus := false
	return gui.Checkbox.Root(gui.CheckboxProps{
		PartProps: gui.PartProps{
			AriaLabel: func() string {
				if checked {
					return "Unstage " + item().Path
				}
				return "Stage " + item().Path
			},
			FocusOnPointer: &focus,
			Style:          gui.Style{Display: "flex", AlignItems: "center", JustifyContent: "center", Width: 22, Height: 22, BorderRadius: 4, Cursor: "default"},
			Children: func() *native.Node {
				return gui.Checkbox.Indicator(gui.PartProps{
					Style: checkboxBox(app.Theme(), checked),
					Children: func() *native.Node {
						return gui.Show(func() bool { return checked }, func() *native.Node { return checkboxMark(true) })
					},
				})
			},
		},
		Checked: func() gui.CheckedState { return checked },
		OnCheckedChange: func(next bool, _ *native.Event) {
			current := item()
			if next {
				store.StageItems([]git.ChangeItem{current})
				return
			}
			store.UnstageItems([]git.ChangeItem{current})
		},
	})
}

func changeName(list model.ListID, item func() git.ChangeItem) *native.Node {
	app := UseApp()
	store := app.Store
	counts := func() string {
		stats := store.Numstat().Unstaged
		if list == model.ListStaged {
			stats = store.Numstat().Staged
		}
		entry, ok := stats[item().Path]
		if !ok {
			return ""
		}
		text := ""
		if entry.Added != nil {
			text += fmt.Sprintf("+%d", *entry.Added)
		}
		if entry.Removed != nil {
			if text != "" {
				text += " "
			}
			text += fmt.Sprintf("-%d", *entry.Removed)
		}
		return text
	}
	return gui.View(gui.Props{
		Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, FlexDirection: "row", AlignItems: "center", Gap: 6, Height: "100%"},
		Children: []any{
			gui.Text(gui.Props{Style: gui.Style{Flex: 1, MinWidth: 0, FontSize: 13, LineClamp: 1}, Children: func() string { return item().Path }}),
			gui.Show(func() bool { return counts() != "" }, func() *native.Node {
				return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, FontFamily: "monospace", Color: app.Theme().TextTertiary}, Children: counts})
			}),
		},
	})
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
					CheckRow("Amend", store.Amend, store.SetAmend),
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
				diffTable(),
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

func diffTable() *native.Node {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.DiffRow] {
		return visibleWindow(store.DiffRows(), visibleRange())
	}
	return gui.Table.Root(gui.TableRootProps{
		PartProps: gui.PartProps{
			Style: gui.Style{Flex: 1, MinHeight: 0, OverflowY: "scroll", FontFamily: "monospace", FontSize: MonoFontSize},
			Children: func() *native.Node {
				return gui.KeyedFor(visible, func(row visibleItem[git.DiffRow]) any { return row.Index }, func(row func() visibleItem[git.DiffRow], _ func() int) *native.Node {
					return diffTableRow(func() git.DiffRow { return row().Item }, func() int { return row().Index })
				}, nil)
			},
		},
		Columns: func() []gui.TableColumnDeclaration {
			return []gui.TableColumnDeclaration{
				{ID: "old", Track: "46px", Align: "end"},
				{ID: "new", Track: "46px", Align: "end"},
				{ID: "text", Track: "1fr"},
			}
		},
		RowCount:      func() float64 { return float64(store.DiffRowCount()) },
		RowHeight:     20,
		HeaderHeight:  0,
		SelectionMode: "multiple",
		Selection:     func() []gui.TableRowRange { return store.DiffSelection() },
		OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
			setVisibleRange(next)
		},
		OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
			store.SetDiffSelection(ranges)
		},
	})
}

func diffTableRow(row func() git.DiffRow, index func() int) *native.Node {
	app := UseApp()
	store := app.Store
	theme := app.Theme()
	current := row()
	background := "transparent"
	textColor := theme.Text
	gutter := theme.ContentAlt
	selected := theme.SelectionMuted
	mark := " "
	markColor := theme.TextTertiary
	switch current.Kind {
	case "hunk":
		background = theme.DiffHunk
		textColor = theme.DiffHunkText
		gutter = theme.DiffHunk
	case "file":
		background = theme.ContentAlt
		textColor = theme.Text
		gutter = theme.ContentAlt
	case "notice":
		textColor = theme.TextTertiary
	case "line":
		switch current.LineKind {
		case git.LineAdded:
			background = theme.DiffAdded
			textColor = theme.DiffAddedText
			gutter = theme.DiffAddedGutter
			selected = "#bfe9cb"
			mark = "+"
			markColor = theme.DiffAddedText
		case git.LineRemoved:
			background = theme.DiffRemoved
			textColor = theme.DiffRemovedText
			gutter = theme.DiffRemovedGutter
			selected = "#f7c7c3"
			mark = "−"
			markColor = theme.DiffRemovedText
		}
	}
	numberStyle := gui.Style{FontFamily: "monospace", FontSize: MonoFontSize - 1, Color: theme.DiffLineNumber, TextAlign: "right", PaddingRight: 6, UserSelect: "none"}
	return gui.Table.Row(gui.TableRowProps{
		Index: func() float64 { return float64(index()) },
		PartProps: gui.PartProps{
			Group: true,
			Style: gui.Style{BackgroundColor: background, Selected: &gui.Style{BackgroundColor: selected}},
			Children: func() *native.Node {
				return gui.Fragment([]*native.Node{
					gui.Table.Cell(gui.TableCellProps{
						Column: "old",
						PartProps: gui.PartProps{
							Style: gui.Style{BackgroundColor: gutter},
							Children: func() *native.Node {
								return gui.Show(func() bool { return row().Kind == "line" && row().OldLineNumber != nil }, func() *native.Node {
									return gui.Text(gui.Props{Style: numberStyle, Children: func() string { return fmt.Sprintf("%d", *row().OldLineNumber) }})
								})
							},
						},
					}),
					gui.Table.Cell(gui.TableCellProps{
						Column: "new",
						PartProps: gui.PartProps{
							Style: gui.Style{BackgroundColor: gutter},
							Children: func() *native.Node {
								return gui.Show(func() bool { return row().Kind == "line" && row().NewLineNumber != nil }, func() *native.Node {
									return gui.Text(gui.Props{Style: numberStyle, Children: func() string { return fmt.Sprintf("%d", *row().NewLineNumber) }})
								})
							},
						},
					}),
					gui.Table.Cell(gui.TableCellProps{
						Column: "text",
						PartProps: gui.PartProps{
							Style: gui.Style{PaddingLeft: 8, PaddingRight: 8, MinWidth: 0, Gap: 8},
							Children: func() *native.Node {
								return gui.View(gui.Props{
									Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, FlexDirection: "row", AlignItems: "center", Gap: 8},
									Children: []any{
										gui.Text(gui.Props{Style: gui.Style{Width: 12, FlexShrink: 0, FontFamily: "monospace", FontSize: MonoFontSize, FontWeight: 700, Color: markColor, UserSelect: "none"}, Children: mark}),
										gui.Text(gui.Props{Style: gui.Style{Flex: 1, MinWidth: 0, FontFamily: "monospace", FontSize: MonoFontSize, Color: textColor, WhiteSpace: "nowrap"}, Children: func() string { return row().Text }}),
										gui.Show(func() bool {
											return row().Kind == "hunk" && store.Diff().Target != nil && store.Diff().Target.Kind != "commit"
										}, func() *native.Node {
											return gui.Button(gui.Props{
												OnClick: func(*native.Event) {
													current := row()
													if store.Diff().Target.Kind == "staged" {
														store.UnstageHunk(current.FileIndex, current.HunkIndex)
														return
													}
													store.StageHunk(current.FileIndex, current.HunkIndex)
												},
												Style: app.Theme().Button("secondary"),
												Children: func() string {
													if store.Diff().Target != nil && store.Diff().Target.Kind == "staged" {
														return "Unstage hunk"
													}
													return "Stage hunk"
												},
											})
										}),
									},
								})
							},
						},
					}),
				})
			},
		},
	})
}

func historyTable() *native.Node {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.Commit] {
		history := store.History()
		window := visibleRange()
		if window.End > len(history.Commits)-60 && !history.Exhausted && !history.Loading && len(history.Commits) > 0 {
			go store.LoadHistory(false)
		}
		return visibleWindow(history.Commits, window)
	}
	return gui.Table.Root(gui.TableRootProps{
		PartProps: gui.PartProps{
			Style: gui.Style{Flex: 1, MinHeight: 0, OverflowY: "scroll"},
			Children: func() *native.Node {
				return gui.KeyedFor(visible, func(row visibleItem[git.Commit]) any { return row.Index }, func(row func() visibleItem[git.Commit], _ func() int) *native.Node {
					return historyTableRow(row)
				}, nil)
			},
		},
		Columns: func() []gui.TableColumnDeclaration {
			return []gui.TableColumnDeclaration{
				{ID: "graph", Track: "28px"},
				{ID: "subject", Track: "1fr", RowHeader: true},
				{ID: "author", Track: "110px"},
				{ID: "date", Track: "84px", Align: "end"},
			}
		},
		RowCount:      func() float64 { return float64(len(store.History().Commits)) },
		RowHeight:     26,
		HeaderHeight:  0,
		SelectionMode: "single",
		Selection:     func() []gui.TableRowRange { return store.HistorySelection() },
		OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
			setVisibleRange(next)
		},
		OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
			store.SetHistorySelection(ranges)
		},
	})
}

func historyTableRow(row func() visibleItem[git.Commit]) *native.Node {
	app := UseApp()
	store := app.Store
	return gui.Table.Row(gui.TableRowProps{
		Index: func() float64 { return float64(row().Index) },
		PartProps: gui.PartProps{
			Style: gui.Style{Hover: &gui.Style{BackgroundColor: app.Theme().Hover}, Selected: &gui.Style{BackgroundColor: app.Theme().Selection}},
			OnContextMenu: func(*native.Event) {
				commit := row().Item
				native.PopupMenu(app.Window, []native.MenuItem{
					{Label: "Copy SHA", Click: func() { native.WriteClipboardText(commit.Sha) }},
					{Label: "New Branch from Here…", Click: func() { app.OpenDialog(DialogRequest{Kind: DialogNewBranch, From: commit.Sha}) }},
					{Label: "Checkout (Detached)", Click: func() { store.CheckoutCommit(commit.Sha) }},
				}, nil, nil, func(error) {})
			},
			Children: func() *native.Node {
				return gui.Fragment([]*native.Node{
					gui.Table.Cell(gui.TableCellProps{
						Column: "graph",
						PartProps: gui.PartProps{Children: func() *native.Node {
							return gui.Text(gui.Props{Style: gui.Style{Width: 28, Color: app.Theme().Accent, FontFamily: "monospace"}, Children: func() string {
								graph := store.History().Graph
								index := row().Index
								if index < len(graph) {
									return fmt.Sprintf("•%d", graph[index].Lane)
								}
								return ""
							}})
						}},
					}),
					gui.Table.Cell(gui.TableCellProps{
						Column: "subject",
						PartProps: gui.PartProps{
							Style: gui.Style{MinWidth: 0, PaddingRight: 8},
							Children: func() *native.Node {
								return gui.View(gui.Props{
									Style: gui.Style{Display: "flex", Flex: 1, MinWidth: 0, FlexDirection: "row", AlignItems: "center", Gap: 6},
									Children: []any{
										gui.For(func() []git.CommitRef {
											refs := row().Item.Refs
											visible := make([]git.CommitRef, 0, 3)
											for _, ref := range refs {
												if ref.Kind == "head" {
													continue
												}
												visible = append(visible, ref)
												if len(visible) == 3 {
													break
												}
											}
											return visible
										}, func(ref git.CommitRef, _ func() int) *native.Node {
											return gui.Text(gui.Props{Style: gui.Style{FontSize: 10, FontWeight: 700, Color: app.Theme().Accent, FlexShrink: 0}, Children: ref.Name})
										}, func(ref git.CommitRef) any { return string(ref.Kind) + ":" + ref.Name }, nil),
										gui.Text(gui.Props{Style: gui.Style{FontSize: 13, LineClamp: 1, MinWidth: 0}, Children: func() string { return row().Item.Subject }}),
									},
								})
							},
						},
					}),
					gui.Table.Cell(gui.TableCellProps{
						Column: "author",
						PartProps: gui.PartProps{Children: func() *native.Node {
							return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary, LineClamp: 1}, Children: func() string { return row().Item.AuthorName }})
						}},
					}),
					gui.Table.Cell(gui.TableCellProps{
						Column: "date",
						PartProps: gui.PartProps{Children: func() *native.Node {
							return gui.Text(gui.Props{Style: gui.Style{FontSize: 11, Color: app.Theme().TextTertiary, TextAlign: "right"}, Children: func() string {
								return git.RelativeTime(row().Item.AuthorTime, time.Now())
							}})
						}},
					}),
				})
			},
		},
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
							CheckRow("All branches", func() bool { return store.History().AllBranches }, store.SetHistoryAllBranches),
						},
					}),
					gui.Show(func() bool { return len(store.History().Commits) > 0 }, func() *native.Node {
						return historyTable()
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

func commitFileTable() *native.Node {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.CommitFile] {
		return visibleWindow(store.CommitDetail().Files, visibleRange())
	}
	selection := func() []gui.TableRowRange {
		path := store.CommitDetail().SelectedPath
		for index, file := range store.CommitDetail().Files {
			if file.Path == path {
				return []gui.TableRowRange{{index, index}}
			}
		}
		return nil
	}
	return gui.Table.Root(gui.TableRootProps{
		PartProps: gui.PartProps{
			Style: gui.Style{Flex: 1, MinHeight: 0, MaxHeight: 140, OverflowY: "scroll"},
			Children: func() *native.Node {
				return gui.KeyedFor(visible, func(row visibleItem[git.CommitFile]) any { return row.Item.Path }, func(row func() visibleItem[git.CommitFile], _ func() int) *native.Node {
					return gui.Table.Row(gui.TableRowProps{
						Index: func() float64 { return float64(row().Index) },
						PartProps: gui.PartProps{
							Style: gui.Style{Hover: &gui.Style{BackgroundColor: app.Theme().Hover}, Selected: &gui.Style{BackgroundColor: app.Theme().Selection}},
							Children: func() *native.Node {
								return gui.Fragment([]*native.Node{
									gui.Table.Cell(gui.TableCellProps{
										Column: "status",
										PartProps: gui.PartProps{Children: func() *native.Node {
											return gui.Text(gui.Props{Style: gui.Style{Width: 16, FontWeight: 700, Color: StatusColor(app.Theme(), row().Item.Status)}, Children: func() string { return row().Item.Status }})
										}},
									}),
									gui.Table.Cell(gui.TableCellProps{
										Column: "name",
										PartProps: gui.PartProps{Children: func() *native.Node {
											return gui.Text(gui.Props{Style: gui.Style{Flex: 1, MinWidth: 0, LineClamp: 1}, Children: func() string { return row().Item.Path }})
										}},
									}),
								})
							},
						},
					})
				}, nil)
			},
		},
		Columns: func() []gui.TableColumnDeclaration {
			return []gui.TableColumnDeclaration{
				{ID: "status", Track: "24px", Align: "center"},
				{ID: "name", Track: "1fr", RowHeader: true},
			}
		},
		RowCount:      func() float64 { return float64(len(store.CommitDetail().Files)) },
		RowHeight:     24,
		HeaderHeight:  0,
		SelectionMode: "single",
		Selection:     selection,
		OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
			setVisibleRange(next)
		},
		OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
			if len(ranges) == 0 || len(ranges[0]) == 0 {
				return
			}
			files := store.CommitDetail().Files
			index := ranges[0][0]
			if index >= 0 && index < len(files) {
				store.SelectCommitFile(files[index].Path)
			}
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
				commitFileTable(),
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
