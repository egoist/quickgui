package main

import (
	"cmp"
	"fmt"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
	"slices"
	"strconv"
	"strings"
)

func ScrollAreaDemo() *ui.Element {
	state, setState := ui.CreateSignal(ui.ScrollAreaState{})
	return panel("Scroll area", "Forty rows in a fixed viewport. Scroll offsets, overflow flags, and thumb geometry come from the native scroll model.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				scrollArea31 := ui.NewScrollArea(ui.ScrollAreaRootProps{
					ViewportSize: func() ui.Extent {
						return ui.Extent{Width: 320, Height: 160}
					},
					ContentSize: func() ui.Extent {
						return ui.Extent{Width: 320, Height: 880}
					},
					OverflowEdgeThreshold: 2,
					OnScrollStateChange:   change(setState),
					PartProps:             ui.PartProps{Style: ui.Style().Display("flex").Gap(4).Height(160)},
				})
				return scrollArea31.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							scrollArea31.Viewport(ui.PartProps{Style: ui.Style().
								Width(320).
								Height(160).
								BackgroundColor(color(func(p palette) string {
									return p.PanelAlt
								})).
								BorderRadius(10).
								BorderWidth(1).
								BorderColor(color(func(p palette) string {
									return p.Border
								})).
								Overflow("hidden")}).
								Children(func() *native.Node {
									return scrollArea31.Content(ui.PartProps{Style: func() ui.StyleBuilder {
										return ui.Style().
											Display("flex").
											FlexDirection("column").
											PaddingLeft(10).
											Transform("translateY(" + strconv.FormatFloat(-state().Offset.Y, 'g', -1, 64) + "px)")
									}}).
										Children(func() *native.Node {
											var children_ []*native.Node
											for i := range 40 {
												children_ = append(children_, ui.Text("log line "+strconv.Itoa(i+1)).
													FontSize(12).
													Height(22).
													FlexShrink(0).
													LineHeight(22).
													TextColor(color(func(p palette) string {
														return p.Muted
													})).Node)
											}
											return ui.Fragment(children_)
										}).
										NativeNode()
								}).
								NativeNode(),
							scrollArea31.Scrollbar(ui.ScrollAreaScrollbarProps{
								Orientation: "vertical",
								PartProps: ui.PartProps{Style: ui.Style().
									Width(8).
									Height(160).
									BackgroundColor(color(func(p palette) string {
										return p.Track
									})).
									BorderRadius(4)},
							}).
								Children(func() *native.Node {
									live := ui.UseScrollAreaState()
									return scrollArea31.Thumb(ui.ScrollAreaThumbProps{
										Orientation: "vertical",
										PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
											return ui.Style().
												Width(8).
												BorderRadius(4).
												BackgroundColor(choose(live().Scrolling, p().Accent, p().Border))
										}},
									}).
										NativeNode()
								}).
								NativeNode(),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				s := state()
				return "offset " + strconv.FormatFloat(s.Offset.Y, 'f', 0, 64) + " · scrolling " + strconv.FormatBool(s.Scrolling) + " · overflow " + strconv.FormatBool(s.HasOverflowY) + " · start " + strconv.FormatBool(s.OverflowYStart) + " · end " + strconv.FormatBool(s.OverflowYEnd)
			}).Node,
		})
	})
}

type asset struct {
	Name string
	Size int
}
type visibleAsset struct {
	Row   int
	Asset asset
}

func TableDemo() *ui.Element {
	assets := make([]asset, 5000)
	for i := range assets {
		assets[i] = asset{Name: fmt.Sprintf("asset-%04d.png", i), Size: 8 + (i*37)%4000}
	}
	sort, setSort := ui.CreateSignal[*ui.TableSortState](nil)
	visibleRange, setRange := ui.CreateSignal(ui.VisibleRange{Start: 0, End: 20})
	selection, setSelection := ui.CreateSignal([]ui.TableRowRange{})
	activated, setActivated := ui.CreateSignal("nothing yet")
	widths, setWidths := ui.CreateSignal("declared")
	ordered := ui.CreateMemo(func() []asset {
		order := sort()
		if order == nil {
			return assets
		}
		list := slices.Clone(assets)
		slices.SortFunc(list, func(a, b asset) int {
			result := cmp.Compare(a.Name, b.Name)
			if order.Column == "size" {
				result = cmp.Compare(a.Size, b.Size)
			}
			if order.Direction == "descending" {
				return -result
			}
			return result
		})
		return list
	})
	visible := func() []visibleAsset {
		list := ordered()
		window := visibleRange()
		rows := []visibleAsset{}
		for i := max(0, window.Start); i < min(len(list), window.End); i++ {
			rows = append(rows, visibleAsset{Row: i, Asset: list[i]})
		}
		return rows
	}
	columns := []ui.TableColumnDeclaration{
		{ID: "name", Label: "Name", Track: "1fr", MinWidth: ptr(140.0), Sortable: true, RowHeader: true},
		{ID: "size", Label: "Size", Width: ptr(140.0), MinWidth: ptr(90.0), Align: "right", Sortable: true},
	}
	return panel("Table", "Five thousand sortable rows. Only the visible range is mounted. Drag column edges, select multiple rows, or double-click to activate.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				table32 := ui.NewTable(ui.TableRootProps{
					Columns: func() []ui.TableColumnDeclaration {
						return columns
					},
					RowCount: func() float64 {
						return float64(len(assets))
					},
					RowHeight:            26,
					HeaderHeight:         28,
					SelectionMode:        "multiple",
					Selection:            selection,
					Sort:                 sort,
					OnVisibleRangeChange: change(setRange),
					OnSelectionChange:    change(setSelection),
					OnSortChange:         change(setSort),
					OnColumnResize: func(next []ui.ColumnWidth, _ *native.Event) {
						parts := []string{}
						for _, entry := range next {
							parts = append(parts, entry.ID+" "+strconv.FormatFloat(entry.Width, 'f', 0, 64))
						}
						setWidths(strings.Join(parts, ", "))
					},
					OnActivate: func(cell ui.TableCell, _ *native.Event) {
						setActivated("row " + strconv.Itoa(cell.Row) + ", column " + strconv.Itoa(cell.Column))
					},
					PartProps: ui.PartProps{Style: ui.Style().
						Height(260).
						BorderRadius(10).
						BorderWidth(1).
						BorderColor(color(func(p palette) string {
							return p.Border
						})).
						BackgroundColor(color(func(p palette) string {
							return p.PanelAlt
						})).
						OverflowY("scroll")},
				})
				return table32.Root().
					Children(func() *native.Node {
						var children_ []*native.Node
						for _, column := range columns {
							children_ = append(children_, table32.Header(ui.TableHeaderProps{
								Column: column.ID,
								PartProps: ui.PartProps{Style: ui.Style().
									PaddingLeft(10).
									PaddingRight(10).
									FontSize(11).
									FontWeight(700).
									TextColor(color(func(p palette) string {
										return p.Faint
									}))},
							}).
								Children(func() *ui.Element {
									return ui.Text(func() string {
										order := sort()
										marker := ""
										if order != nil && order.Column == column.ID {
											marker = choose(order.Direction == "ascending", " ↑", " ↓")
										}
										return strings.ToUpper(column.Label) + marker
									})
								}).
								NativeNode())
						}
						children_ = append(children_, ui.KeyedFor(
							visible,
							func(entry visibleAsset) any {
								return entry.Row
							},
							func(entry func() visibleAsset, _ func() int) *native.Node {
								return table32.Row(ui.TableRowProps{Index: func() int {
									return entry().Row
								}}).
									Children(func() *native.Node {
										return ui.Fragment([]*native.Node{
											table32.Cell(ui.TableCellProps{
												Column:    "name",
												PartProps: ui.PartProps{Style: ui.Style().PaddingLeft(10)},
											}).
												Children(func() *ui.Element {
													return label(func() string {
														return entry().Asset.Name
													})
												}).
												NativeNode(),
											table32.Cell(ui.TableCellProps{
												Column: "size",
												PartProps: ui.PartProps{Style: ui.Style().
													PaddingRight(10).
													JustifyContent("flex-end")},
											}).
												Children(func() *ui.Element {
													return muted(func() string {
														return strconv.Itoa(entry().Asset.Size) + " KB"
													})
												}).
												NativeNode(),
										})
									}).
									NativeNode()
							},
							nil,
						))
						return ui.Fragment(children_)
					}).
					NativeNode()
			}(),
			note(func() string {
				selected := 0
				for _, span := range selection() {
					if len(span) >= 2 {
						selected += span[1] - span[0] + 1
					}
				}
				return fmt.Sprintf("rows %d–%d of %d · %d selected · sort %+v · widths %s · activated %s", visibleRange().Start, visibleRange().End, len(assets), selected, sort(), widths(), activated())
			}).Node,
		})
	})
}

type flatTreeRow struct {
	Node  ui.TreeNodeDeclaration
	Depth int
}

func flattenTree(nodes []ui.TreeNodeDeclaration, depth int, expanded []string, loaded *ui.TreeChildrenSplice, rows *[]flatTreeRow) {
	for _, node := range nodes {
		*rows = append(*rows, flatTreeRow{node, depth})
		if slices.Contains(expanded, node.ID) {
			children := node.Children
			if loaded != nil && loaded.ID == node.ID {
				children = loaded.Children
			}
			flattenTree(children, depth+1, expanded, loaded, rows)
		}
	}
}
func TreeDemo() *ui.Element {
	nodes := []ui.TreeNodeDeclaration{
		{ID: "src", Label: "src", Children: []ui.TreeNodeDeclaration{
			{ID: "src/main.go", Label: "main.go"},
			{ID: "src/ui.go", Label: "ui.go"},
			{ID: "src/element", Label: "element", Pending: true},
		}},
		{ID: "docs", Label: "docs", Children: []ui.TreeNodeDeclaration{
			{ID: "docs/native.md", Label: "native.md"},
			{ID: "docs/tabs.md", Label: "tabs.md"},
		}},
	}
	expanded, setExpanded := ui.CreateSignal([]string{"src"})
	selected, setSelected := ui.CreateSignal[*string](nil)
	activated, setActivated := ui.CreateSignal("nothing yet")
	visibleRange, setRange := ui.CreateSignal(ui.VisibleRange{Start: 0, End: 30})
	loaded, setLoaded := ui.CreateSignal[*ui.TreeChildrenSplice](nil)
	asked, setAsked := ui.CreateSignal(0)
	visible := func() []flatTreeRow {
		rows := []flatTreeRow{}
		flattenTree(nodes, 0, expanded(), loaded(), &rows)
		window := visibleRange()
		start := min(len(rows), max(0, window.Start))
		return rows[start:min(len(rows), max(start, window.End))]
	}
	return panel("Tree", "Expand the pending element branch to request its children once. Arrow keys navigate, and double-click reports activation.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				tree33 := ui.NewTree(ui.TreeRootProps{
					Nodes: func() []ui.TreeNodeDeclaration {
						return nodes
					},
					Expanded:             expanded,
					Value:                selected,
					SetChildren:          loaded,
					RowHeight:            26,
					LoadingLabel:         "Loading…",
					Disclosure:           "leading",
					OnExpandedChange:     change(setExpanded),
					OnValueChange:        change(setSelected),
					OnVisibleRangeChange: change(setRange),
					OnActivate:           change(setActivated),
					OnLoadChildren: func(id string, _ *native.Event) {
						setAsked(asked() + 1)
						setLoaded(&ui.TreeChildrenSplice{
							ID: id,
							Children: []ui.TreeNodeDeclaration{
								{ID: id + "/style.go", Label: "style.go"},
								{ID: id + "/state.go", Label: "state.go"},
							},
						})
					},
					PartProps: ui.PartProps{Style: ui.Style().
						Height(220).
						BorderRadius(10).
						BorderWidth(1).
						BorderColor(color(func(p palette) string {
							return p.Border
						})).
						BackgroundColor(color(func(p palette) string {
							return p.PanelAlt
						})).
						OverflowY("scroll")},
				})
				return tree33.Root().
					Children(func() *native.Node {
						return ui.KeyedFor(
							visible,
							func(row flatTreeRow) any {
								return row.Node.ID
							},
							func(row func() flatTreeRow, _ func() int) *native.Node {
								return tree33.Row(ui.TreeRowProps{
									NodeID: row().Node.ID,
									PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
										return ui.Style().
											Display("flex").
											AlignItems("center").
											Height(26).
											PaddingLeft(10 + row().Depth*16)
									}},
								}).
									Children(func() *ui.Element {
										return label(func() string {
											return row().Node.Label
										})
									}).
									NativeNode()
							},
							nil,
						)
					}).
					NativeNode()
			}(),
			note(func() string {
				return "expanded [" + strings.Join(expanded(), ", ") + "] · selected " + textValue(selected()) + " · activated " + activated() + " · load requests " + strconv.Itoa(asked())
			}).Node,
		})
	})
}
