package ui

import (
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

func TestSwiftUIHostAndButtonDeclareDedicatedTags(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		host := SwiftUI.Host(SwiftUIHostProps{
			MatchContents: true,
			PartProps: PartProps{
				Children: func() *native.Node {
					return SwiftUI.Button(SwiftUIButtonProps{
						Label:       "Continue",
						SystemImage: "arrow.right",
						Modifiers:   []SwiftUIModifier{SwiftUI.ButtonStyle("glass"), SwiftUI.ControlSize("large")},
					})
				},
			},
		})
		if host.Tag != protocol.TagSwiftUIHost || len(host.Children) != 1 || host.Children[0].Tag != protocol.TagSwiftUIButton {
			t.Fatalf("host tag %d children %d", host.Tag, len(host.Children))
		}
		return struct{}{}
	})
}

func TestSwiftUISliderInputUpdatesControlledValue(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal(0.25)
		slider := SwiftUI.Slider(SwiftUISliderProps{
			Value: value,
			Min:   0,
			Max:   1,
			Label: func() string { return "Volume" },
			OnValueChange: func(next float64, _ *native.Event) {
				setValue(next)
			},
		})
		if slider.Tag != protocol.TagSwiftUISlider {
			t.Fatalf("tag %d", slider.Tag)
		}
		host := &native.NodeHost{Nodes: map[uint32]*native.Node{slider.ID: slider}}
		native.DispatchEvent(host, protocol.EventInput, slider.ID, "0.75", true)
		if value() != 0.75 {
			t.Fatalf("value %v", value())
		}
		native.DispatchEvent(host, protocol.EventInput, slider.ID, "", false)
		if value() != 0.75 {
			t.Fatal("missing payload changed the value")
		}
		return struct{}{}
	})
}

func TestSwiftUIDatePickerConvertsMilliseconds(t *testing.T) {
	native.ResetTreeStateForTests()
	got := 0.0
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		picker := SwiftUI.DatePicker(SwiftUIDatePickerProps{
			Value:               float64(1_700_000_000_000),
			DisplayedComponents: "dateAndTime",
			Style:               "field",
			OnValueChange: func(next float64, _ *native.Event) {
				got = next
			},
		})
		host := &native.NodeHost{Nodes: map[uint32]*native.Node{picker.ID: picker}}
		native.DispatchEvent(host, protocol.EventInput, picker.ID, "1700000001.5", true)
		if got != 1_700_000_001_500 {
			t.Fatalf("got %v", got)
		}
		return struct{}{}
	})
}

func TestSwiftUIPopoverToggleRequiresRoot(t *testing.T) {
	native.ResetTreeStateForTests()
	presented := false
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		open, setOpen := CreateSignal(false)
		root := SwiftUI.Popover.Root(SwiftUIPopoverProps{
			IsPresented:         open,
			OnIsPresentedChange: setOpen,
			Children: func() *native.Node {
				return SwiftUI.Popover.Trigger(SwiftUIPopoverTriggerProps{
					Render: func() *native.Node {
						return SwiftUI.Button(SwiftUIButtonProps{Label: "Open"})
					},
				})
			},
		})
		if root.Tag != protocol.TagSwiftUIPopover {
			t.Fatalf("tag %d", root.Tag)
		}
		trigger := root.Children[0]
		host := &native.NodeHost{Nodes: map[uint32]*native.Node{trigger.ID: trigger}}
		native.DispatchEvent(host, protocol.EventClick, trigger.ID, "", false)
		if !open() {
			t.Fatal("trigger did not present")
		}
		presented = open()
		return struct{}{}
	})
	if !presented {
		t.Fatal("popover stayed closed")
	}
}

func TestSwiftUIModifiersEncodeDollarType(t *testing.T) {
	payload, err := json.Marshal([]SwiftUIModifier{SwiftUI.ButtonStyle("glass"), SwiftUI.Disabled(true)})
	if err != nil {
		t.Fatal(err)
	}
	if string(payload) != `[{"$type":"buttonStyle","style":"glass"},{"$type":"disabled","disabled":true}]` {
		t.Fatalf("%s", payload)
	}
}
