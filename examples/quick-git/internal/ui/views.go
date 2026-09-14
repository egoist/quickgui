package ui

import (
	"github.com/egoist/quickgui/go/native"
	gui "github.com/egoist/quickgui/go/ui"
	"path/filepath"
	"quickgui.example/quick-git/internal/git"
	"quickgui.example/quick-git/internal/model"
	"strconv"
	"time"
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
func ChangesView() *gui.Element {
	app := UseApp()
	store := app.Store
	return gui.View().
		Children(
			resizablePanel("Resize file list", store.ChangesSplit, store.SetChangesSplit, 220, 800, func() *native.Node {
				return gui.Fragment([]*native.Node{
					gui.Show(
						store.ChangeCount() > 0,
						func() *native.Node {
							return gui.Fragment([]*native.Node{
								fileList(model.ListUnstaged).Node,
								fileList(model.ListStaged).Node,
							})
						},
						func() *gui.Element {
							return emptyState("No local changes", "The working tree matches the last commit.")
						},
					),
					commitComposer().Node,
				})
			}, gui.Style().Display("flex").FlexShrink(0).MinHeight(0).FlexDirection("column")),
			DiffPane(),
		).
		Display("flex").
		Flex(1).
		MinWidth(0).
		MinHeight(0).
		FlexDirection("row")
}
func fileList(list model.ListID) *gui.Element {
	app := UseApp()
	store := app.Store
	label := "Unstaged"
	if list == model.ListStaged {
		label = "Staged"
	}
	return gui.View().
		Children(
			gui.View().
				Children(
					gui.Text(label).FontSize(11).FontWeight(600).TextColor(app.Theme().TextTertiary),
					gui.Text(len(store.ListItems(list))).
						FontSize(11).
						TextColor(app.Theme().TextTertiary),
					gui.View().Flex(1),
					gui.Show(
						len(store.ListItems(list)) > 0,
						func() *gui.Element {
							text := "Stage All"
							if list == model.ListStaged {
								text = "Unstage All"
							}
							return gui.Button().
								Style(app.Theme().Button("secondary")).
								Child(text).
								Disabled(store.Busy() != nil).
								OnClick(func() {
									if list == model.ListUnstaged {
										store.StageAll()
									} else {
										store.UnstageAll()
									}
								})
						},
					),
				).
				Display("flex").
				FlexDirection("row").
				AlignItems("center").
				Height(32).
				FlexShrink(0).
				PaddingLeft(12).
				PaddingRight(8).
				Gap(6),
			gui.Show(
				len(store.ListItems(list)) > 0,
				func() *native.Node {
					return changeTable(list)
				},
				func() *gui.Element {
					empty := "No unstaged changes"
					if list == model.ListStaged {
						empty = "Nothing staged yet"
					}
					return gui.Text(empty).
						PaddingLeft(12).
						PaddingBottom(10).
						FontSize(12).
						TextColor(app.Theme().TextTertiary)
				},
			),
		).
		Display("flex").
		FlexDirection("column").
		MinHeight(0).
		Flex(1).
		FlexBasis(0)
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
	table70 := gui.NewTable(gui.TableRootProps{
		PartProps: gui.PartProps{Style: gui.Style().Flex(1).MinHeight(0).OverflowY("scroll")},
		Columns: func() []gui.TableColumnDeclaration {
			return []gui.TableColumnDeclaration{
				{ID: "toggle", Track: "40px", Align: "center"},
				{ID: "status", Track: "24px", Align: "center"},
				{ID: "name", Track: "1fr", RowHeader: true},
			}
		},
		RowCount: func() float64 {
			return float64(len(store.ListItems(list)))
		},
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
	return table70.Root().
		Children(func() *native.Node {
			return gui.KeyedFor(
				visible,
				func(row visibleItem[git.ChangeItem]) any {
					return row.Index
				},
				func(row func() visibleItem[git.ChangeItem], _ func() int) *native.Node {
					return table70.Row(gui.TableRowProps{
						Index: func() float64 {
							return float64(row().Index)
						},
						PartProps: gui.PartProps{Style: gui.Style().
							Hover(func(s gui.StyleBuilder) gui.StyleBuilder {
								return s.BackgroundColor(app.Theme().Hover)
							}).
							SelectedStyle(func(s gui.StyleBuilder) gui.StyleBuilder {
								return s.BackgroundColor(app.Theme().Selection)
							})},
					}).
						Children(func() *native.Node {
							return gui.Fragment([]*native.Node{
								table70.Cell(gui.TableCellProps{
									Column:    "toggle",
									PartProps: gui.PartProps{},
								}).
									Children(func() *native.Node {
										return stageToggle(list, func() git.ChangeItem {
											return row().Item
										})
									}).
									NativeNode(),
								table70.Cell(gui.TableCellProps{
									Column:    "status",
									PartProps: gui.PartProps{},
								}).
									Children(func() *gui.Element {
										return gui.Text(string(row().Item.Code)).
											Width(16).
											FontWeight(700).
											TextColor(StatusColor(app.Theme(), string(row().Item.Code)))
									}).
									NativeNode(),
								table70.Cell(gui.TableCellProps{
									Column: "name",
									PartProps: gui.PartProps{
										Style: gui.Style().PaddingRight(8).MinWidth(0),
										OnContextMenu: func(*native.Event) {
											changeMenu(app, list, row().Item)
										},
										OnDoubleClick: func(*native.Event) {
											store.ToggleStaging(list)
										},
									},
								}).
									Children(func() *gui.Element {
										return changeName(list, func() git.ChangeItem {
											return row().Item
										})
									}).
									NativeNode(),
							})
						}).
						NativeNode()
				},
				nil,
			)
		}).
		NativeNode()
}
func stageToggle(list model.ListID, item func() git.ChangeItem) *native.Node {
	app := UseApp()
	store := app.Store
	checked := list == model.ListStaged
	focus := false
	checkbox71 := gui.NewCheckbox(gui.CheckboxProps{
		PartProps: gui.PartProps{
			AriaLabel: func() string {
				if checked {
					return "Unstage " + item().Path
				}
				return "Stage " + item().Path
			},
			FocusOnPointer: &focus,
			Style: gui.Style().
				Display("flex").
				AlignItems("center").
				JustifyContent("center").
				Width(22).
				Height(22).
				BorderRadius(4).
				Cursor("default"),
		},
		Checked: func() gui.CheckedState {
			return checked
		},
		OnCheckedChange: func(next bool, _ *native.Event) {
			current := item()
			if next {
				store.StageItems([]git.ChangeItem{current})
				return
			}
			store.UnstageItems([]git.ChangeItem{current})
		},
	})
	return checkbox71.Root().
		Children(func() *native.Node {
			return checkbox71.Indicator(gui.PartProps{Style: checkboxBox(app.Theme(), checked)}).
				Children(func() *native.Node {
					return gui.Show(
						checked,
						func() *gui.Element {
							return checkboxMark(true)
						},
					)
				}).
				NativeNode()
		}).
		NativeNode()
}
func changeName(list model.ListID, item func() git.ChangeItem) *gui.Element {
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
			text += "+" + strconv.Itoa(*entry.Added)
		}
		if entry.Removed != nil {
			if text != "" {
				text += " "
			}
			text += "-" + strconv.Itoa(*entry.Removed)
		}
		return text
	}
	return gui.View().
		Children(
			gui.Text(item().Path).Flex(1).MinWidth(0).FontSize(13).LineClamp(1),
			gui.Show(
				counts() != "",
				func() *gui.Element {
					return gui.Text(counts).
						FontSize(11).
						FontFamily("monospace").
						TextColor(app.Theme().TextTertiary)
				},
			),
		).
		Display("flex").
		Flex(1).
		MinWidth(0).
		FlexDirection("row").
		AlignItems("center").
		Gap(6).
		Height("100%")
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
		items = append(items, native.MenuItem{
			Label: "Stage File",
			Click: func() {
				store.StageItems(targets)
			},
		})
		items = append(items, native.MenuItem{
			Label: "Discard Changes…",
			Click: func() {
				confirmDiscard(app, targets)
			},
		})
	} else {
		items = append(items, native.MenuItem{
			Label: "Unstage File",
			Click: func() {
				store.UnstageItems(targets)
			},
		})
	}
	items = append(items, native.MenuItem{Type: "separator"}, native.MenuItem{
		Label: "Reveal in Finder",
		Click: func() {
			native.ShowItemInFolder(
				absolute,
				func(error) {
				},
			)
		},
	}, native.MenuItem{
		Label: "Copy Path",
		Click: func() {
			native.WriteClipboardText(item.Path)
		},
	})
	native.PopupMenu(
		app.Window,
		items,
		nil,
		nil,
		func(error) {
		},
	)
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
	message := "Discard changes to " + strconv.Itoa(len(items)) + " files?"
	if len(items) == 1 {
		message = "Discard changes to " + filepath.Base(items[0].Path) + "?"
	}
	native.ShowAlertDialog(
		native.AlertDialogOptions{
			Message: message,
			Detail:  "The changes cannot be recovered.",
			Level:   "warning",
			Buttons: []native.AlertDialogButton{
				{Label: "Cancel"},
				{Label: "Discard", Role: "destructive"},
			},
			Window: app.Window,
		},
		func(index int, err error) {
			if err == nil && index == 1 {
				app.Store.DiscardItems(items)
			}
		},
	)
}
func commitComposer() *gui.Element {
	app := UseApp()
	store := app.Store
	return gui.View().
		Children(
			gui.Input().
				Style(app.Theme().InputStyle()).
				Placeholder("Commit summary").
				Value(store.Subject()).
				OnInputEvent(func(event *native.Event) {
					store.SetSubject(event.Value)
				}).
				OnSubmitEvent(func(*native.Event) {
					store.Commit()
				}),
			gui.TextArea().
				Placeholder("Description").
				Value(store.Body()).
				OnInputEvent(func(event *native.Event) {
					store.SetBody(event.Value)
				}).
				Display("flex").
				Width("100%").
				MinHeight(72).
				PaddingLeft(7).
				PaddingRight(7).
				PaddingTop(4).
				PaddingBottom(4).
				BackgroundColor(app.Theme().Input).
				TextColor(app.Theme().Text).
				BorderWidth(1).
				BorderColor(app.Theme().InputBorder).
				BorderRadius(6).
				FontSize(UIFontSize),
			gui.View().
				Children(
					CheckRow("Amend", store.Amend, store.SetAmend),
					gui.View().Flex(1),
					gui.Show(
						len(store.Agents()) > 0,
						func() *gui.Element {
							return gui.Button().
								Style(app.Theme().Button("secondary")).
								Child(func() string {
									if store.Generating() != nil {
										return "Generating…"
									}
									return "Generate"
								}).
								Disabled(store.Generating() != nil).
								OnClick(func() {
									store.GenerateMessage("")
								})
						},
					),
					gui.Button().
						Style(app.Theme().Button("primary")).
						Child("Commit").
						Disabled(!store.CanCommit()).
						OnClick(func() {
							store.Commit()
						}),
				).
				Display("flex").
				FlexDirection("row").
				AlignItems("center").
				Gap(8),
		).
		Display("flex").
		FlexDirection("column").
		FlexShrink(0).
		Gap(8).
		Padding(12).
		BorderTopWidth(1).
		BorderColor(app.Theme().Border)
}
func DiffPane() *gui.Element {
	app := UseApp()
	store := app.Store
	return gui.View().
		Child(gui.Show(
			store.Diff().Target != nil,
			func() *native.Node {
				return gui.Fragment([]*native.Node{
					gui.View().
						Children(
							gui.Text(func() string {
								if target := store.Diff().Target; target != nil {
									return target.Path
								}
								return ""
							}).
								Flex(1).
								MinWidth(0).
								FontSize(13).
								FontWeight(600).
								LineClamp(1),
							gui.Show(
								store.DiffStats().Added > 0,
								func() *gui.Element {
									return gui.Text("+" + strconv.Itoa(store.DiffStats().Added)).
										FontSize(11).
										FontWeight(700).
										TextColor(app.Theme().Success).
										FontFamily("monospace")
								},
							),
							gui.Show(
								store.DiffStats().Removed > 0,
								func() *gui.Element {
									return gui.Text("-" + strconv.Itoa(store.DiffStats().Removed)).
										FontSize(11).
										FontWeight(700).
										TextColor(app.Theme().Danger).
										FontFamily("monospace")
								},
							),
							gui.Show(
								store.Diff().Loading && store.Diff().Diff == nil,
								func() *gui.Element {
									return gui.Text("Loading…").
										FontSize(11).
										TextColor(app.Theme().TextTertiary)
								},
							),
							diffActions(),
						).
						Display("flex").
						FlexDirection("row").
						AlignItems("center").
						Gap(8).
						Height(40).
						FlexShrink(0).
						PaddingLeft(14).
						PaddingRight(10).
						BorderBottomWidth(1).
						BorderColor(app.Theme().Border).Node,
					gui.Show(
						store.Diff().Error != "",
						func() *gui.Element {
							return gui.Text(store.Diff().Error).
								Padding(12).
								TextColor(app.Theme().Danger).
								FontSize(12)
						},
					),
					diffTable(),
				})
			},
			func() *gui.Element {
				title := "Select a file"
				if store.View() == model.ViewHistory {
					title = "Select a commit"
				}
				return emptyState(title, "Choose a change to review its diff.")
			},
		)).
		Display("flex").
		Flex(1).
		MinWidth(0).
		MinHeight(0).
		FlexDirection("column").
		BackgroundColor(app.Theme().Content)
}
func diffActions() *native.Node {
	app := UseApp()
	store := app.Store
	return gui.Show(
		store.Diff().Target != nil && store.Diff().Target.Kind != "commit",
		func() *gui.Element {
			mode := store.Diff().Target.Kind
			lineCount := store.SelectedDiffLineCount()
			return gui.View().
				Child(gui.Show(
					lineCount > 0,
					func() *native.Node {
						var children []*native.Node
						if mode == "staged" {
							children = append(children, gui.Button().
								Style(app.Theme().Button("primary")).
								Child("Unstage "+strconv.Itoa(lineCount)+" Lines").
								OnClick(func() {
									store.ApplySelectedLines("unstage")
								}).Node)
							return gui.Fragment(children)
						}
						children = append(children, gui.Button().
							Style(app.Theme().Button("danger")).
							Child("Discard Lines…").
							OnClick(func() {
								store.ApplySelectedLines("discard")
							}).Node)
						children = append(children, gui.Button().
							Style(app.Theme().Button("primary")).
							Child("Stage "+strconv.Itoa(lineCount)+" Lines").
							OnClick(func() {
								store.ApplySelectedLines("stage")
							}).Node)
						return gui.Fragment(children)
					},
					func() *native.Node {
						var children []*native.Node
						if mode == "staged" {
							children = append(children, gui.Button().
								Style(app.Theme().Button("secondary")).
								Child("Unstage File").
								OnClick(func() {
									if item := store.ActiveItem(); item != nil {
										store.UnstageItems([]git.ChangeItem{*item})
									}
								}).Node)
							return gui.Fragment(children)
						}
						children = append(children, gui.Button().
							Style(app.Theme().Button("danger")).
							Child("Discard…").
							OnClick(func() {
								if item := store.ActiveItem(); item != nil {
									confirmDiscard(app, []git.ChangeItem{*item})
								}
							}).Node)
						children = append(children, gui.Button().
							Style(app.Theme().Button("primary")).
							Child("Stage File").
							OnClick(func() {
								if item := store.ActiveItem(); item != nil {
									store.StageItems([]git.ChangeItem{*item})
								}
							}).Node)
						return gui.Fragment(children)
					},
				)).
				Display("flex").
				FlexDirection("row").
				Gap(6)
		},
	)
}
func diffTable() *native.Node {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.DiffRow] {
		return visibleWindow(store.DiffRows(), visibleRange())
	}
	table72 := gui.NewTable(gui.TableRootProps{
		PartProps: gui.PartProps{Style: gui.Style().
			Flex(1).
			MinHeight(0).
			OverflowY("scroll").
			FontFamily("monospace").
			FontSize(MonoFontSize)},
		Columns: func() []gui.TableColumnDeclaration {
			return []gui.TableColumnDeclaration{
				{ID: "old", Track: "46px", Align: "end"},
				{ID: "new", Track: "46px", Align: "end"},
				{ID: "text", Track: "1fr"},
			}
		},
		RowCount: func() float64 {
			return float64(store.DiffRowCount())
		},
		RowHeight:     20,
		HeaderHeight:  0,
		SelectionMode: "multiple",
		Selection: func() []gui.TableRowRange {
			return store.DiffSelection()
		},
		OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
			setVisibleRange(next)
		},
		OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
			store.SetDiffSelection(ranges)
		},
	})
	return table72.Root().
		Children(func() *native.Node {
			return gui.KeyedFor(
				visible,
				func(row visibleItem[git.DiffRow]) any {
					return row.Index
				},
				func(row func() visibleItem[git.DiffRow], _ func() int) *native.Node {
					return diffTableRow(table72, func() git.DiffRow {
						return row().Item
					}, func() int {
						return row().Index
					})
				},
				nil,
			)
		}).
		NativeNode()
}
func diffTableRow(table *gui.TableComponent, row func() git.DiffRow, index func() int) *native.Node {
	app := UseApp()
	store := app.Store
	theme := app.Theme()
	current := row()
	background := "transparent"
	textColor := theme.Text
	gutter := theme.ContentAlt
	selected := theme.SelectionMuted
	mark := ""
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
			mark = plusIcon
			markColor = theme.DiffAddedText
		case git.LineRemoved:
			background = theme.DiffRemoved
			textColor = theme.DiffRemovedText
			gutter = theme.DiffRemovedGutter
			selected = "#f7c7c3"
			mark = minusIcon
			markColor = theme.DiffRemovedText
		}
	}
	numberStyle := gui.Style().
		FontFamily("monospace").
		FontSize(MonoFontSize - 1).
		TextColor(theme.DiffLineNumber).
		TextAlign("right").
		PaddingRight(6).
		UserSelect("none")
	return table.Row(gui.TableRowProps{
		Index: func() float64 {
			return float64(index())
		},
		PartProps: gui.PartProps{
			Group: true,
			Style: gui.Style().
				BackgroundColor(background).
				SelectedStyle(func(s gui.StyleBuilder) gui.StyleBuilder {
					return s.BackgroundColor(selected)
				}),
		},
	}).
		Child(func() *native.Node {
			return gui.Fragment([]*native.Node{
				table.Cell(gui.TableCellProps{
					Column:    "old",
					PartProps: gui.PartProps{Style: gui.Style().BackgroundColor(gutter)},
				}).
					Child(func() *native.Node {
						return gui.Show(
							row().Kind == "line" && row().OldLineNumber != nil,
							func() *gui.Element {
								return gui.Text(*row().OldLineNumber, numberStyle)
							},
						)
					}).
					NativeNode(),
				table.Cell(gui.TableCellProps{
					Column:    "new",
					PartProps: gui.PartProps{Style: gui.Style().BackgroundColor(gutter)},
				}).
					Child(func() *native.Node {
						return gui.Show(
							row().Kind == "line" && row().NewLineNumber != nil,
							func() *gui.Element {
								return gui.Text(*row().NewLineNumber, numberStyle)
							},
						)
					}).
					NativeNode(),
				table.Cell(gui.TableCellProps{
					Column:    "text",
					PartProps: gui.PartProps{Style: gui.Style().PaddingLeft(8).PaddingRight(8).MinWidth(0).Gap(8)},
				}).
					Child(func() *gui.Element {
						return gui.View().
							Children(
								gui.View().
									Child(func() *native.Node {
										var children []*native.Node
										if mark != "" {
											children = append(children, icon(mark, 12, func() string {
												return markColor
											}).Node)
										}
										return gui.Fragment(children)
									}).
									Width(12).
									FlexShrink(0),
								gui.Text(row().Text).
									Flex(1).
									MinWidth(0).
									FontFamily("monospace").
									FontSize(MonoFontSize).
									TextColor(textColor).
									WhiteSpace("nowrap"),
								gui.Show(
									row().Kind == "hunk" && store.Diff().Target != nil && store.Diff().Target.Kind != "commit",
									func() *gui.Element {
										return gui.Button().
											Style(app.Theme().Button("secondary")).
											Child(func() string {
												if store.Diff().Target != nil && store.Diff().Target.Kind == "staged" {
													return "Unstage hunk"
												}
												return "Stage hunk"
											}).
											OnClick(func() {
												current := row()
												if store.Diff().Target.Kind == "staged" {
													store.UnstageHunk(current.FileIndex, current.HunkIndex)
													return
												}
												store.StageHunk(current.FileIndex, current.HunkIndex)
											}).
											Height(16).
											FontFamily("system-ui").
											FontSize(11).
											LineHeight(14).
											PaddingLeft(6).
											PaddingRight(6).
											BorderRadius(4)
									},
								),
							).
							Display("flex").
							Flex(1).
							MinWidth(0).
							FlexDirection("row").
							AlignItems("center").
							Gap(8)
					}).
					NativeNode(),
			})
		}).
		NativeNode()
}
func historyTable() *native.Node {
	app := UseApp()
	store := app.Store
	visibleRange, setVisibleRange := gui.CreateSignal(gui.VisibleRange{Start: 0, End: 0})
	visible := func() []visibleItem[git.Commit] {
		history := store.History()
		window := visibleRange()
		if window.End > len(history.Commits)-60 && !history.Exhausted && !history.Loading && len(history.Commits) > 0 {
			store.LoadHistory(false)
		}
		return visibleWindow(history.Commits, window)
	}
	graphWidth := gui.CreateMemo(func() float64 {
		lanes := 1
		for _, row := range visibleWindow(store.History().Graph, visibleRange()) {
			lanes = max(lanes, row.Item.LaneCount)
		}
		return 2*graphInset + float64(min(lanes, 8))*graphLaneWidth
	})
	table73 := gui.NewTable(gui.TableRootProps{
		PartProps: gui.PartProps{Style: gui.Style().Flex(1).MinHeight(0).OverflowY("scroll")},
		Columns: func() []gui.TableColumnDeclaration {
			return []gui.TableColumnDeclaration{
				{ID: "graph", Track: strconv.FormatFloat(graphWidth(), 'g', -1, 64) + "px"},
				{ID: "subject", Track: "1fr", RowHeader: true},
				{ID: "author", Track: "110px"},
				{ID: "date", Track: "84px", Align: "end"},
			}
		},
		RowCount: func() float64 {
			return float64(len(store.History().Commits))
		},
		RowHeight:     historyRowHeight,
		HeaderHeight:  0,
		SelectionMode: "single",
		Selection: func() []gui.TableRowRange {
			return store.HistorySelection()
		},
		OnVisibleRangeChange: func(next gui.VisibleRange, _ *native.Event) {
			setVisibleRange(next)
		},
		OnSelectionChange: func(ranges []gui.TableRowRange, _ *native.Event) {
			store.SetHistorySelection(ranges)
		},
	})
	return table73.Root().
		Children(func() *native.Node {
			return gui.KeyedFor(
				visible,
				func(row visibleItem[git.Commit]) any {
					return row.Index
				},
				func(row func() visibleItem[git.Commit], _ func() int) *native.Node {
					return historyTableRow(table73, row, graphWidth)
				},
				nil,
			)
		}).
		NativeNode()
}
func historyTableRow(table *gui.TableComponent, row func() visibleItem[git.Commit], graphWidth func() float64) *native.Node {
	app := UseApp()
	store := app.Store
	return table.Row(gui.TableRowProps{
		Index: func() float64 {
			return float64(row().Index)
		},
		PartProps: gui.PartProps{
			Style: gui.Style().
				Hover(func(s gui.StyleBuilder) gui.StyleBuilder {
					return s.BackgroundColor(app.Theme().Hover)
				}).
				SelectedStyle(func(s gui.StyleBuilder) gui.StyleBuilder {
					return s.BackgroundColor(app.Theme().Selection)
				}),
			OnContextMenu: func(*native.Event) {
				commit := row().Item
				native.PopupMenu(
					app.Window,
					[]native.MenuItem{
						{Label: "Copy SHA", Click: func() {
							native.WriteClipboardText(commit.Sha)
						}},
						{Label: "New Branch from Here…", Click: func() {
							app.OpenDialog(DialogRequest{Kind: DialogNewBranch, From: commit.Sha})
						}},
						{Label: "Checkout (Detached)", Click: func() {
							store.CheckoutCommit(commit.Sha)
						}},
					},
					nil,
					nil,
					func(error) {
					},
				)
			},
		},
	}).
		Child(func() *native.Node {
			return gui.Fragment([]*native.Node{
				table.Cell(gui.TableCellProps{
					Column:    "graph",
					PartProps: gui.PartProps{},
				}).
					Child(func() *gui.Element {
						return historyGraph(func() *git.GraphRow {
							graph := store.History().Graph
							index := row().Index
							if index < 0 || index >= len(graph) {
								return nil
							}
							return &graph[index]
						}, graphWidth)
					}).
					NativeNode(),
				table.Cell(gui.TableCellProps{
					Column:    "subject",
					PartProps: gui.PartProps{Style: gui.Style().MinWidth(0).PaddingRight(8).Overflow("hidden")},
				}).
					Child(func() *gui.Element {
						return gui.View().
							Children(
								commitRefs(func() []git.CommitRef {
									return row().Item.Refs
								}),
								gui.Text(row().Item.Subject).
									Flex(1).
									FontSize(12.5).
									LineClamp(1).
									MinWidth(0),
							).
							Display("flex").
							Flex(1).
							MinWidth(0).
							FlexDirection("row").
							AlignItems("center").
							Gap(6)
					}).
					NativeNode(),
				table.Cell(gui.TableCellProps{
					Column:    "author",
					PartProps: gui.PartProps{Style: gui.Style().MinWidth(0).PaddingRight(8).Overflow("hidden")},
				}).
					Child(func() *gui.Element {
						return gui.Text(row().Item.AuthorName).
							FontSize(11).
							TextColor(app.Theme().TextTertiary).
							LineClamp(1)
					}).
					NativeNode(),
				table.Cell(gui.TableCellProps{
					Column:    "date",
					PartProps: gui.PartProps{Style: gui.Style().PaddingRight(10)},
				}).
					Child(func() *gui.Element {
						return gui.Text(git.RelativeTime(row().Item.AuthorTime, time.Now())).
							FontSize(11).
							TextColor(app.Theme().TextTertiary).
							TextAlign("right")
					}).
					NativeNode(),
			})
		}).
		NativeNode()
}
func HistoryView() *gui.Element {
	app := UseApp()
	store := app.Store
	return gui.View().
		Children(
			resizablePanel("Resize history", store.HistorySplit, store.SetHistorySplit, 260, 1000, func() *native.Node {
				return gui.Fragment([]*native.Node{
					gui.View().
						Children(
							gui.Text(func() string {
								if store.History().AllBranches {
									return "All branches"
								}
								if status := store.Status(); status != nil && status.Branch != "" {
									return status.Branch
								}
								return "History"
							}).
								FontSize(11).
								FontWeight(700).
								TextTransform("uppercase").
								TextColor(app.Theme().TextTertiary),
							gui.Text(func() string {
								suffix := ""
								if !store.History().Exhausted {
									suffix = "+"
								}
								return strconv.Itoa(len(store.History().Commits)) + suffix + " commits"
							}).
								FontSize(11).
								TextColor(app.Theme().TextTertiary),
							gui.View().Flex(1),
							CheckRow("All branches", func() bool {
								return store.History().AllBranches
							}, store.SetHistoryAllBranches),
						).
						Display("flex").
						FlexDirection("row").
						AlignItems("center").
						Height(32).
						FlexShrink(0).
						PaddingLeft(12).
						PaddingRight(8).
						Gap(8).
						BorderBottomWidth(1).
						BorderColor(app.Theme().Border).Node,
					gui.Show(
						len(store.History().Commits) > 0,
						func() *native.Node {
							return historyTable()
						},
						func() *gui.Element {
							return emptyState("No commits", "This repository has no history yet.")
						},
					),
				})
			}, gui.Style().Display("flex").FlexShrink(0).MinHeight(0).FlexDirection("column")),
			gui.View().
				Children(
					commitDetail(),
					DiffPane(),
				).
				Display("flex").
				Flex(1).
				MinWidth(0).
				MinHeight(0).
				FlexDirection("column"),
		).
		Display("flex").
		Flex(1).
		MinWidth(0).
		MinHeight(0).
		FlexDirection("row")
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
		if path == "" && len(store.CommitDetail().Files) > 0 {
			path = store.CommitDetail().Files[0].Path
		}
		for index, file := range store.CommitDetail().Files {
			if file.Path == path {
				return []gui.TableRowRange{{index, index}}
			}
		}
		return nil
	}
	table74 := gui.NewTable(gui.TableRootProps{
		PartProps: gui.PartProps{Style: gui.Style().
			Height(func() float64 {
				return float64(min(8, len(store.CommitDetail().Files)) * 24)
			}).
			FlexShrink(0).
			MinHeight(0).
			OverflowY("scroll")},
		Columns: func() []gui.TableColumnDeclaration {
			return []gui.TableColumnDeclaration{
				{ID: "status", Track: "36px", Align: "start"},
				{ID: "name", Track: "1fr", RowHeader: true},
			}
		},
		RowCount: func() float64 {
			return float64(len(store.CommitDetail().Files))
		},
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
	return table74.Root().
		Children(func() *native.Node {
			return gui.KeyedFor(
				visible,
				func(row visibleItem[git.CommitFile]) any {
					return row.Item.Path
				},
				func(row func() visibleItem[git.CommitFile], _ func() int) *native.Node {
					return table74.Row(gui.TableRowProps{
						Index: func() float64 {
							return float64(row().Index)
						},
						PartProps: gui.PartProps{Style: gui.Style().
							Hover(func(s gui.StyleBuilder) gui.StyleBuilder {
								return s.BackgroundColor(app.Theme().Hover)
							}).
							SelectedStyle(func(s gui.StyleBuilder) gui.StyleBuilder {
								return s.BackgroundColor(app.Theme().Selection)
							})},
					}).
						Children(func() *native.Node {
							return gui.Fragment([]*native.Node{
								table74.Cell(gui.TableCellProps{
									Column:    "status",
									PartProps: gui.PartProps{Style: gui.Style().PaddingLeft(12)},
								}).
									Children(func() *gui.Element {
										return gui.Text(row().Item.Status).
											Width(16).
											FontWeight(700).
											TextColor(StatusColor(app.Theme(), row().Item.Status))
									}).
									NativeNode(),
								table74.Cell(gui.TableCellProps{
									Column:    "name",
									PartProps: gui.PartProps{},
								}).
									Children(func() *gui.Element {
										return gui.Text(row().Item.Path).Flex(1).MinWidth(0).LineClamp(1)
									}).
									NativeNode(),
							})
						}).
						NativeNode()
				},
				nil,
			)
		}).
		NativeNode()
}
func commitDetail() *gui.Element {
	app := UseApp()
	store := app.Store
	return gui.View().
		Child(gui.Show(
			store.SelectedCommit() != nil,
			func() *native.Node {
				return gui.Fragment([]*native.Node{
					gui.View().
						Children(
							gui.Text(func() string {
								if commit := store.SelectedCommit(); commit != nil {
									return commit.Subject
								}
								return ""
							}).
								FontSize(14).
								FontWeight(700),
							gui.Show(
								func() bool {
									commit := store.SelectedCommit()
									return commit != nil && commit.Body != ""
								},
								func() *gui.Element {
									return gui.Text(func() string {
										if commit := store.SelectedCommit(); commit != nil {
											return commit.Body
										}
										return ""
									}).
										FontSize(12.5).
										LineHeight(18).
										UserSelect("text").
										TextColor(app.Theme().TextSecondary)
								},
							),
							gui.Text(func() string {
								if commit := store.SelectedCommit(); commit != nil {
									return commit.AuthorName + " · " + git.AbsoluteTime(commit.AuthorTime) + " · " + commit.ShortSha
								}
								return ""
							}).
								FontSize(12).
								TextColor(app.Theme().TextSecondary),
						).
						Padding(12).
						Display("flex").
						FlexDirection("column").
						Gap(4).Node,
					gui.Show(
						store.CommitDetail().Loading,
						func() *gui.Element {
							return gui.Text("Loading files…").PaddingLeft(12).FontSize(12)
						},
					),
					commitFileTable(),
				})
			},
			func() *gui.Element {
				return emptyState("Select a commit", "Its files appear here.")
			},
		)).
		Display("flex").
		FlexDirection("column").
		FlexShrink(0).
		MaxHeight("45%").
		MinHeight(0).
		OverflowY("auto").
		BorderBottomWidth(1).
		BorderColor(app.Theme().Border)
}
func BranchesView() *gui.Element {
	app := UseApp()
	store := app.Store
	return gui.View().
		Children(
			gui.View().
				Children(
					gui.Text("Local branches").
						FontSize(11).
						FontWeight(700).
						TextTransform("uppercase").
						TextColor(app.Theme().TextTertiary).
						Flex(1),
					gui.Button().
						Style(app.Theme().Button("primary")).
						Child("New Branch").
						OnClick(func() {
							app.OpenDialog(DialogRequest{Kind: DialogNewBranch})
						}),
				).
				Display("flex").
				FlexDirection("row").
				AlignItems("center").
				Height(32).
				Gap(8),
			gui.For(
				func() []git.BranchRef {
					return store.Refs().Local
				},
				func(branch git.BranchRef, _ func() int) *gui.Element {
					return gui.Button().
						Style(rowStyle(app.Theme(), branch.Current)).
						Children(
							gui.Text(branch.Name).
								Flex(1).
								FontWeight(ternary(branch.Current, 700, 500)).
								LineClamp(1),
							gui.Show(
								branch.Current,
								func() *gui.Element {
									return gui.Text("current").FontSize(11).TextColor(app.Theme().Accent)
								},
							),
							gui.Text(branch.ShortSha).
								FontSize(11).
								TextColor(app.Theme().TextTertiary),
						).
						OnClick(func() {
						}).
						OnDoubleClick(func(*native.Event) {
							if !branch.Current {
								store.SwitchBranch(branch.Name)
							}
						}).
						OnContextMenu(func(*native.Event) {
							native.PopupMenu(
								app.Window,
								[]native.MenuItem{
									{Label: "Switch to Branch", Click: func() {
										store.SwitchBranch(branch.Name)
									}},
									{Label: "New Branch from Here…", Click: func() {
										app.OpenDialog(DialogRequest{Kind: DialogNewBranch, From: branch.Name})
									}},
									{Label: "Copy Branch Name", Click: func() {
										native.WriteClipboardText(branch.Name)
									}},
									{Type: "separator"},
									{Label: "Delete Branch…", Click: func() {
										store.DeleteBranch(branch.Name, true)
									}},
								},
								nil,
								nil,
								func(error) {
								},
							)
						})
				},
				func(branch git.BranchRef) any {
					return branch.FullName
				},
				nil,
			),
			gui.Show(
				len(store.Refs().Remote) > 0,
				func() *gui.Element {
					return gui.Text("Remote branches").
						FontSize(11).
						FontWeight(700).
						TextTransform("uppercase").
						TextColor(app.Theme().TextTertiary).
						MarginTop(16)
				},
			),
			gui.For(
				func() []git.BranchRef {
					return store.Refs().Remote
				},
				func(branch git.BranchRef, _ func() int) *gui.Element {
					return gui.View().
						Children(
							gui.Text(branch.Name).
								Flex(1).
								LineClamp(1).
								TextColor(app.Theme().TextSecondary),
							gui.Text(branch.ShortSha).
								FontSize(11).
								TextColor(app.Theme().TextTertiary),
							rowStyle(app.Theme(), false),
						)
				},
				func(branch git.BranchRef) any {
					return branch.FullName
				},
				nil,
			),
		).
		Display("flex").
		Flex(1).
		MinHeight(0).
		FlexDirection("column").
		OverflowY("auto").
		Padding(12).
		Gap(4)
}
func WorktreesView() *gui.Element {
	app := UseApp()
	store := app.Store
	return gui.View().
		Children(
			gui.View().
				Children(
					gui.Text("Worktrees").
						FontSize(11).
						FontWeight(700).
						TextTransform("uppercase").
						TextColor(app.Theme().TextTertiary).
						Flex(1),
					gui.Button().
						Style(app.Theme().Button("primary")).
						Child("New Worktree").
						OnClick(func() {
							app.OpenDialog(DialogRequest{Kind: DialogNewWorktree})
						}),
				).
				Display("flex").
				FlexDirection("row").
				AlignItems("center").
				Height(32),
			gui.For(
				store.Worktrees,
				func(worktree git.Worktree, _ func() int) *gui.Element {
					active := store.Repository() != nil && store.Repository().Root() == worktree.Path
					label := worktree.BranchName
					if label == "" {
						label = filepath.Base(worktree.Path)
					}
					return gui.Button().
						Style(rowStyle(app.Theme(), active)).
						Children(
							gui.Text(label).Flex(1).LineClamp(1),
							gui.Text(filepath.Base(worktree.Path)).
								FontSize(11).
								TextColor(app.Theme().TextTertiary),
						).
						OnClick(func() {
							store.SelectWorktree(worktree.Path)
						}).
						OnContextMenu(func(*native.Event) {
							items := []native.MenuItem{
								{Label: "Reveal in Finder", Click: func() {
									native.ShowItemInFolder(
										worktree.Path,
										func(error) {
										},
									)
								}},
								{Label: "Copy Path", Click: func() {
									native.WriteClipboardText(worktree.Path)
								}},
							}
							if !worktree.Main {
								items = append(items, native.MenuItem{Type: "separator"}, native.MenuItem{
									Label: "Remove Worktree",
									Click: func() {
										store.RemoveWorktree(worktree.Path, false)
									},
								})
							}
							native.PopupMenu(
								app.Window,
								items,
								nil,
								nil,
								func(error) {
								},
							)
						})
				},
				func(worktree git.Worktree) any {
					return worktree.Path
				},
				nil,
			),
		).
		Display("flex").
		Flex(1).
		MinHeight(0).
		FlexDirection("column").
		OverflowY("auto").
		Padding(12).
		Gap(4)
}
func StashesView() *gui.Element {
	app := UseApp()
	store := app.Store
	return gui.View().
		Children(
			gui.View().
				Children(
					gui.Text("Stashes").
						FontSize(11).
						FontWeight(700).
						TextTransform("uppercase").
						TextColor(app.Theme().TextTertiary).
						Flex(1),
					gui.Button().
						Style(app.Theme().Button("primary")).
						Child("Stash Changes").
						Disabled(store.ChangeCount() == 0).
						OnClick(func() {
							app.OpenDialog(DialogRequest{Kind: DialogStash})
						}),
				).
				Display("flex").
				FlexDirection("row").
				AlignItems("center").
				Height(32),
			gui.Show(
				len(store.Stashes()) > 0,
				func() *native.Node {
					return gui.For(
						store.Stashes,
						func(stash git.StashEntry, _ func() int) *gui.Element {
							return gui.Button().
								Style(rowStyle(app.Theme(), false)).
								Children(
									gui.View().
										FlexCol().
										Flex1().
										MinWidth(0).
										Children(
											gui.Text(stash.Summary).LineClamp(1),
											gui.Text(stash.Ref+" · "+git.RelativeTime(stash.Time, time.Now())).
												FontSize(11).
												TextColor(app.Theme().TextTertiary),
										),
									gui.Button().
										Style(app.Theme().Button("secondary")).
										Child("Apply").
										OnClick(func() { store.StashApply(stash.Ref) }),
									gui.Button().
										Style(app.Theme().Button("secondary")).
										Child("Pop").
										OnClick(func() { store.StashPop(stash.Ref) }),
								).
								OnContextMenu(func(*native.Event) {
									native.PopupMenu(
										app.Window,
										[]native.MenuItem{
											{Label: "Apply", Click: func() {
												store.StashApply(stash.Ref)
											}},
											{Label: "Pop", Click: func() {
												store.StashPop(stash.Ref)
											}},
											{Label: "Drop", Click: func() {
												store.StashDrop(stash.Ref)
											}},
										},
										nil,
										nil,
										func(error) {
										},
									)
								})
						},
						func(stash git.StashEntry) any {
							return stash.Ref
						},
						nil,
					)
				},
				func() *gui.Element {
					return emptyState("No stashes", "Stash local changes to save them for later.")
				},
			),
		).
		Display("flex").
		Flex(1).
		MinHeight(0).
		FlexDirection("column").
		OverflowY("auto").
		Padding(12).
		Gap(4)
}
func emptyState(title, description string) *gui.Element {
	app := UseApp()
	return gui.View().
		Children(
			gui.Text(title).
				FontSize(15).
				FontWeight(500).
				TextColor(app.Theme().TextTertiary).
				TextAlign("center"),
			gui.Text(description).
				FontSize(12).
				LineHeight(17).
				TextColor(app.Theme().TextTertiary).
				TextAlign("center").
				MaxWidth(360),
		).
		Display("flex").
		Flex(1).
		MinHeight(0).
		FlexDirection("column").
		AlignItems("center").
		JustifyContent("center").
		Gap(8).
		Padding(32)
}
func ternary[T any](cond bool, a, b T) T {
	if cond {
		return a
	}
	return b
}
