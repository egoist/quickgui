package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// TableColumnDeclaration is one declared table column.
type TableColumnDeclaration struct {
	ID        string   `json:"id"`
	Label     string   `json:"label,omitempty"`
	Width     *float64 `json:"width,omitempty"`
	MinWidth  *float64 `json:"minWidth,omitempty"`
	Align     string   `json:"align,omitempty"`
	Sortable  bool     `json:"sortable,omitempty"`
	RowHeader bool     `json:"rowHeader,omitempty"`
	Track     string   `json:"track,omitempty"`
}

// TableRowRange is one inclusive [start, end] row range.
type TableRowRange = []int

type TableRootProps struct {
	PartProps
	Columns              func() []TableColumnDeclaration
	RowCount             func() float64
	RowHeight            any
	HeaderHeight         any
	SelectionMode        string
	Selection            func() []TableRowRange
	Sort                 func() *TableSortState
	Editing              func() *TableCell
	OnVisibleRangeChange func(VisibleRange, *native.Event)
	OnSelectionChange    func([]TableRowRange, *native.Event)
	OnSortChange         func(*TableSortState, *native.Event)
	OnActiveCellChange   func(*TableCell, *native.Event)
	OnColumnResize       func([]ColumnWidth, *native.Event)
	OnColumnReorder      func([]string, *native.Event)
	OnEditEnd            func(TableEditEndDetails, *native.Event)
	OnActivate           func(TableCell, *native.Event)
}

type TableHeaderProps struct {
	PartProps
	Column string
}

type TableRowProps struct {
	PartProps
	Index any
}

type TableCellProps struct {
	PartProps
	Column string
	Index  *float64
}

// Table is a virtual table whose visible rows the application declares.
var Table = tableAPI{}

type tableAPI struct{}

func (tableAPI) Root(props TableRootProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartTable, createComponentScope("qg-table"), "")
	if props.Columns != nil {
		reactive.CreateRenderEffect(func() {
			setJson(node, protocol.Columns, 65536, props.Columns())
		})
	}
	if props.RowCount != nil {
		reactive.CreateRenderEffect(func() {
			setNumber(node, protocol.RowCount, props.RowCount())
		})
	}
	if props.RowHeight != nil {
		setNumber(node, protocol.RowHeight, props.RowHeight)
	}
	if props.HeaderHeight != nil {
		setNumber(node, protocol.HeaderHeight, props.HeaderHeight)
	}
	if props.SelectionMode != "" {
		setString(node, protocol.SelectionMode, props.SelectionMode)
	}
	if props.Selection != nil {
		reactive.CreateRenderEffect(func() {
			setJson(node, protocol.Selection, 65536, props.Selection())
		})
	}
	if props.Sort != nil {
		reactive.CreateRenderEffect(func() {
			current := props.Sort()
			if current == nil {
				setString(node, protocol.SortColumn, "")
				setString(node, protocol.SortDirection, "")
				return
			}
			setString(node, protocol.SortColumn, current.Column)
			setString(node, protocol.SortDirection, current.Direction)
		})
	}
	if props.Editing != nil {
		reactive.CreateRenderEffect(func() {
			setJson(node, protocol.Editing, 65536, props.Editing())
		})
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.VisibleRange != nil && props.OnVisibleRangeChange != nil {
			props.OnVisibleRangeChange(*details.VisibleRange, event)
		}
		if details.SelectedRanges != nil && props.OnSelectionChange != nil {
			props.OnSelectionChange(details.SelectedRanges, event)
		}
		if details.Sort.present() && props.OnSortChange != nil {
			props.OnSortChange(details.Sort.ptr(), event)
		}
		if details.ActiveCell.present() && props.OnActiveCellChange != nil {
			props.OnActiveCellChange(details.ActiveCell.ptr(), event)
		}
		if details.ColumnWidths != nil && props.OnColumnResize != nil {
			props.OnColumnResize(details.ColumnWidths, event)
		}
		if details.ColumnOrder != nil && props.OnColumnReorder != nil {
			props.OnColumnReorder(details.ColumnOrder, event)
		}
		if details.EditEnded != nil && props.OnEditEnd != nil {
			props.OnEditEnd(*details.EditEnded, event)
		}
	})
	if props.OnActivate != nil {
		setListener(node, protocol.EventCommit, func(event *native.Event) {
			details := CommitFromEvent(event)
			if details == nil || details.Row == nil || details.Column == nil {
				return
			}
			props.OnActivate(TableCell{Row: *details.Row, Column: *details.Column}, event)
		})
	}
	return finishPart(node, props.PartProps)
}

func (tableAPI) Header(props TableHeaderProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartTableHeader, "", props.Column)
	return finishPart(node, props.PartProps)
}

func (tableAPI) Row(props TableRowProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartTableRow, "", "")
	if props.Index != nil {
		bindNumber(node, protocol.RowIndex, props.Index)
	}
	return finishPart(node, props.PartProps)
}

func (tableAPI) Cell(props TableCellProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartTableCell, "", props.Column)
	if props.Index != nil {
		setNumber(node, protocol.ColumnIndex, *props.Index)
	}
	return finishPart(node, props.PartProps)
}

// TreeNodeDeclaration is one declared tree node.
type TreeNodeDeclaration struct {
	ID       string                `json:"id"`
	Label    string                `json:"label,omitempty"`
	Disabled bool                  `json:"disabled,omitempty"`
	Pending  bool                  `json:"pending,omitempty"`
	Children []TreeNodeDeclaration `json:"children,omitempty"`
}

// TreeChildrenSplice is one bounded lazy-children splice.
type TreeChildrenSplice struct {
	ID       string                `json:"id"`
	Children []TreeNodeDeclaration `json:"children"`
}

type TreeRootProps struct {
	PartProps
	Nodes                func() []TreeNodeDeclaration
	Expanded             func() []string
	DefaultExpanded      []string
	Value                func() *string
	RowHeight            any
	LoadingLabel         string
	Disclosure           string
	SetChildren          func() *TreeChildrenSplice
	OnExpandedChange     func([]string, *native.Event)
	OnValueChange        func(*string, *native.Event)
	OnLoadChildren       func(string, *native.Event)
	OnVisibleRangeChange func(VisibleRange, *native.Event)
	OnActivate           func(string, *native.Event)
}

type TreeRowProps struct {
	PartProps
	NodeID string
}

// Tree is a virtual tree whose expansion and selection the core owns.
var Tree = treeAPI{}

type treeAPI struct{}

func (treeAPI) Root(props TreeRootProps) *native.Node {
	uncontrolled := reactive.NewSignal(append([]string(nil), props.DefaultExpanded...))
	expanded := func() []string {
		if props.Expanded != nil {
			return props.Expanded()
		}
		return uncontrolled.Read()
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartTree, createComponentScope("qg-tree"), "")
	if props.Nodes != nil {
		reactive.CreateRenderEffect(func() {
			setJson(node, protocol.Nodes, 1048576, props.Nodes())
		})
	}
	reactive.CreateRenderEffect(func() {
		setJson(node, protocol.Expanded, 65536, expanded())
	})
	if props.Value != nil {
		reactive.CreateRenderEffect(func() {
			current := props.Value()
			if current == nil {
				setComponentValue(node, protocol.SelectedValue, "")
				return
			}
			setComponentValue(node, protocol.SelectedValue, *current)
		})
	}
	if props.RowHeight != nil {
		setNumber(node, protocol.RowHeight, props.RowHeight)
	}
	if props.LoadingLabel != "" {
		setString(node, protocol.LoadingLabel, props.LoadingLabel)
	}
	if props.Disclosure != "" {
		setString(node, protocol.Disclosure, props.Disclosure)
	}
	if props.SetChildren != nil {
		reactive.CreateRenderEffect(func() {
			setJson(node, protocol.SetChildren, 1048576, props.SetChildren())
		})
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.VisibleRange != nil && props.OnVisibleRangeChange != nil {
			props.OnVisibleRangeChange(*details.VisibleRange, event)
		}
		if details.Expanded != nil {
			if props.Expanded == nil {
				uncontrolled.Write(details.Expanded)
			}
			if props.OnExpandedChange != nil {
				props.OnExpandedChange(details.Expanded, event)
			}
		}
		if details.Value.present() && props.OnValueChange != nil {
			props.OnValueChange(details.Value.ptr(), event)
		}
		if details.LoadChildren != "" && props.OnLoadChildren != nil {
			props.OnLoadChildren(details.LoadChildren, event)
		}
	})
	if props.OnActivate != nil {
		setListener(node, protocol.EventCommit, func(event *native.Event) {
			details := CommitFromEvent(event)
			if details == nil || details.Value == "" {
				return
			}
			props.OnActivate(details.Value, event)
		})
	}
	return finishPart(node, props.PartProps)
}

func (treeAPI) Row(props TreeRowProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartTreeRow, "", props.NodeID)
	return finishPart(node, props.PartProps)
}
