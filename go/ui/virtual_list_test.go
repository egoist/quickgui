package ui

import (
	"bytes"
	"testing"

	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestVirtualListDistanceOverscanAndKnownHeightsAreReactive(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		heights := reactive.NewSignal([]float64{20, 2_000_000})
		list := VirtualList().OverscanPixels(96).ItemHeights(heights.Read)

		expectedOverscan := protocol.NewBatch()
		expectedOverscan.SetNumber(list.ID, protocol.OverscanPixels, 96)
		if !bytes.Contains(list.Pending.Body(), expectedOverscan.Body()) ||
			!bytes.Contains(list.Pending.Body(), []byte(`[20,2000000]`)) {
			t.Fatal("virtual-list distance overscan or bounded known heights were not declared")
		}

		offset := len(list.Pending.Body())
		heights.Write([]float64{24, 48})
		expectedHeights := protocol.NewBatch()
		expectedHeights.SetString(list.ID, protocol.ItemHeights, `[24,48]`)
		if !bytes.Equal(list.Pending.Body()[offset:], expectedHeights.Body()) {
			t.Fatal("updating known heights changed unrelated properties")
		}

		offset = len(list.Pending.Body())
		heights.Write(nil)
		cleared := protocol.NewBatch()
		cleared.ClearProperty(list.ID, protocol.ItemHeights)
		if !bytes.Equal(list.Pending.Body()[offset:], cleared.Body()) {
			t.Fatal("clearing known heights retained the native declaration")
		}
		return struct{}{}
	})
}
