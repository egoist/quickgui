package ui

import (
	"bytes"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/model"
)

func TestToastPortalIsCenteredAgainstTheAppRoot(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		theme, _ := gui.CreateSignal(ThemeFor("light"))
		toast := gui.NewToast()
		var portal *native.Node
		var toastID string
		root := captureComponent(func() *native.Node {
			return ProvideApp(AppContext{Store: &model.Store{}, Theme: theme}, func() *native.Node {
				return toast.Provider().
					Child(func() *native.Node {
						toastID = gui.UseToastManager().Add(gui.ToastRequest{Title: "Short"})
						portal = notices(toast)
						return portal
					}).
					NativeNode()
			})
		})
		if root == nil || portal == nil || len(portal.Children) != 1 {
			t.Fatal("toast portal or viewport was not mounted")
		}
		for _, declaration := range []*protocol.Batch{
			func() *protocol.Batch {
				batch := protocol.NewBatch()
				batch.SetNumber(portal.ID, protocol.Left, 0)
				return batch
			}(),
			func() *protocol.Batch {
				batch := protocol.NewBatch()
				batch.SetNumber(portal.ID, protocol.Right, 0)
				return batch
			}(),
			func() *protocol.Batch {
				batch := protocol.NewBatch()
				batch.SetString(portal.ID, protocol.JustifyContent, "center")
				return batch
			}(),
			func() *protocol.Batch {
				batch := protocol.NewBatch()
				batch.SetNumber(portal.Children[0].ID, protocol.Width, 340)
				return batch
			}(),
		} {
			if !bytes.Contains(portal.Pending.Body(), declaration.Body()) {
				t.Fatal("toast portal lost its root-relative centered layout")
			}
		}

		viewport := portal.Children[0]
		host := native.NewNodeHost(1, 1)
		host.Nodes[viewport.ID] = viewport
		native.DispatchEvent(
			host,
			protocol.EventComponentChange,
			viewport.ID,
			`{"toasts":[{"id":"`+toastID+`","index":0,"type":"default","limited":false,"expanded":false,"swiping":false,"swipeMovement":0,"offset":0}]}`,
			true,
		)
		var positioner *native.Node
		for _, child := range viewport.Children {
			if child.Tag != protocol.TagSentinel {
				positioner = child
				break
			}
		}
		if positioner == nil || len(positioner.Children) != 1 {
			t.Fatal("reported toast did not mount its positioner and root")
		}
		toastRoot := positioner.Children[0]
		before := portal.Pending.MutationCount()
		native.SetString(positioner, protocol.Width, "100%")
		native.SetNumber(positioner, protocol.MinWidth, 0)
		native.SetString(toastRoot, protocol.Width, "100%")
		native.SetNumber(toastRoot, protocol.MinWidth, 0)
		if portal.Pending.MutationCount() != before {
			t.Fatal("toast positioner or root does not fill the centered viewport")
		}
		return struct{}{}
	})
}
