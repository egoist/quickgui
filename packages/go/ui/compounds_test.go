package ui

import (
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

func TestCheckboxDeclaresTheCorePart(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		checked, setChecked := CreateSignal(false)
		node := Checkbox.Root(CheckboxProps{
			Checked: func() CheckedState { return checked() },
			OnCheckedChange: func(next bool, _ *native.Event) {
				setChecked(next)
			},
			PartProps: PartProps{
				Children: func() *native.Node {
					return Checkbox.Indicator(PartProps{})
				},
			},
		})
		if node.Tag != protocol.TagButton {
			t.Fatalf("tag %d", node.Tag)
		}
		if node.Pending == nil || node.Pending.Empty() {
			t.Fatal("expected checkbox mutations")
		}
		return struct{}{}
	})
}

func TestDialogAndTableDeclareCoreParts(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		open := func() bool { return true }
		dialog := Dialog.Root(DialogRootProps{
			Open: open,
			Children: func() *native.Node {
				return Dialog.Portal(PartProps{
					Children: func() *native.Node {
						return Dialog.Popup(DialogPopupProps{
							PartProps: PartProps{Children: func() *native.Node {
								return Dialog.Title(PartProps{Children: func() *native.Node {
									return Text(Props{Children: "Title"})
								}})
							}},
						})
					},
				})
			},
		})
		if dialog == nil {
			t.Fatal("dialog")
		}
		table := Table.Root(TableRootProps{
			Columns: func() []TableColumnDeclaration {
				return []TableColumnDeclaration{{ID: "name", Track: "1fr"}}
			},
			RowCount:     func() float64 { return 2 },
			RowHeight:    24,
			HeaderHeight: 0,
			PartProps: PartProps{
				Children: func() *native.Node {
					return Table.Row(TableRowProps{
						Index: 0,
						PartProps: PartProps{Children: func() *native.Node {
							return Table.Cell(TableCellProps{Column: "name", PartProps: PartProps{
								Children: func() *native.Node { return Text(Props{Children: "a"}) },
							}})
						}},
					})
				},
			},
		})
		if table.Pending == nil || table.Pending.Empty() {
			t.Fatal("expected table mutations")
		}
		return struct{}{}
	})
}

func TestToastManagerOwnsTheQueue(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		var manager *ToastManager
		Toast.Provider(ToastProviderProps{
			Timeout: 1000,
			Limit:   2,
			Children: func() *native.Node {
				manager = UseToastManager()
				manager.Add(ToastRequest{Title: "Saved", Type: ToastSuccess})
				return Text(Props{Children: "ok"})
			},
		})
		if manager == nil {
			t.Fatal("manager")
		}
		if got := manager.Toasts(); len(got) != 1 || got[0].Title != "Saved" {
			t.Fatalf("toasts %#v", got)
		}
		manager.Close(manager.Toasts()[0].ID)
		if len(manager.Toasts()) != 0 {
			t.Fatal("close left the toast queued")
		}
		return struct{}{}
	})
}

func TestComponentChangeDistinguishesNullFromMissing(t *testing.T) {
	var missing ComponentChangeDetails
	if err := json.Unmarshal([]byte(`{"visibleRange":{"start":0,"end":4}}`), &missing); err != nil {
		t.Fatal(err)
	}
	if missing.Sort.present() {
		t.Fatal("sort should be missing")
	}
	if missing.VisibleRange == nil || missing.VisibleRange.End != 4 {
		t.Fatalf("range %#v", missing.VisibleRange)
	}
	var cleared ComponentChangeDetails
	if err := json.Unmarshal([]byte(`{"sort":null,"activeCell":null}`), &cleared); err != nil {
		t.Fatal(err)
	}
	if !cleared.Sort.present() || cleared.Sort.ptr() != nil {
		t.Fatal("sort should be an explicit null")
	}
	if !cleared.ActiveCell.present() || cleared.ActiveCell.ptr() != nil {
		t.Fatal("activeCell should be an explicit null")
	}
}
