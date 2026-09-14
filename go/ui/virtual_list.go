package ui

import (
	"fmt"
	"math"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

const maxListItemHeightsJSONBytes = 1024 * 1024

func bindItemHeights(node *native.Node, value any) {
	var read func() []float64
	switch value := value.(type) {
	case []float64:
		read = func() []float64 { return value }
	case func() []float64:
		read = value
	case reactive.Accessor[[]float64]:
		read = value
	default:
		panic(fmt.Sprintf("QuickGUI item heights must be []float64 or an accessor, got %T", value))
	}
	node.Bind(func() {
		values := read()
		if values == nil {
			native.ClearProperty(node, protocol.ItemHeights)
			return
		}
		for _, value := range values {
			if math.IsNaN(value) || math.IsInf(value, 0) || value <= 0.0 {
				panic("QuickGUI item heights must be positive finite numbers")
			}
		}
		setJson(node, protocol.ItemHeights, maxListItemHeightsJSONBytes, values)
	})
}
