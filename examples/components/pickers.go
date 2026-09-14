package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
	"strconv"
	"strings"
)

func pickerAppearance() *ui.PickerAppearance {
	return &ui.PickerAppearance{
		Width:               ptr(280.0),
		RowHeight:           ptr(30.0),
		MaxVisibleRows:      ptr(7.0),
		FontSize:            ptr(12.0),
		Radius:              ptr(10.0),
		Background:          p().Popup,
		Color:               p().Ink,
		HighlightBackground: p().Accent,
		HighlightColor:      p().OnAccent,
		SelectedBackground:  p().Selection,
		MutedColor:          p().Muted,
	}
}
func pickerSource(label string, items []ui.OptionDeclaration) ui.PickerSourceProps {
	s := inputStyle()
	s = s.Width(280)
	return ui.PickerSourceProps{
		Items: func() []ui.OptionDeclaration {
			return items
		},
		Appearance: pickerAppearance(),
		PartProps:  ui.PartProps{AriaLabel: label, Style: s},
	}
}
func AutocompleteDemo() *ui.Element {
	query, setQuery := ui.CreateSignal("")
	committed, setCommitted := ui.CreateSignal("—")
	open, setOpen := ui.CreateSignal(false)
	items := []ui.OptionDeclaration{
		{Value: "window", Label: "Window"},
		{Value: "widget", Label: "Widget"},
		{Value: "tabs", Label: "Tabs"},
		{Value: "table", Label: "Table"},
		{Value: "toolbar", Label: "Toolbar"},
		{Value: "tooltip", Label: "Tooltip"},
	}
	return panel("Autocomplete", "Free-form text with suggestions. The core filters, ranks, and paints the popup rows.", func() *native.Node {
		var children []*native.Node
		source := pickerSource("Search components", items)
		source.FilterMode = "fuzzy"
		source.OnOpenChange = change(setOpen)
		source.OnCommit = func(d ui.CommitDetails, _ *native.Event) {
			setCommitted(choose(d.Value != "", d.Value, d.InputValue))
		}
		children = append(children, func() *native.Node {
			autocomplete26 := ui.NewAutocomplete(ui.AutocompleteRootProps{PickerInputProps: ui.PickerInputProps{
				PickerSourceProps:  source,
				Placeholder:        "Type win, tab, or tool",
				OnInputValueChange: change(setQuery),
			}})
			return autocomplete26.Root().NativeNode()
		}())
		children = append(children, note(func() string {
			return "query " + strconv.Quote(query()) + " · popup " + strconv.FormatBool(open()) + " · committed " + committed()
		}).Node)
		return ui.Fragment(children)
	})
}
func ComboboxDemo() *ui.Element {
	fruit, setFruit := ui.CreateSignal(ptr("apple"))
	tags, setTags := ui.CreateSignal([]string{"go"})
	fruits := []ui.OptionDeclaration{
		{Value: "apple", Label: "Apple", Group: "Common"},
		{Value: "banana", Label: "Banana", Group: "Common"},
		{Value: "lychee", Label: "Lychee", Group: "Tropical"},
		{Value: "mango", Label: "Mango", Group: "Tropical"},
	}
	languages := []ui.OptionDeclaration{
		{Value: "go", Label: "Go"},
		{Value: "zig", Label: "Zig"},
		{Value: "swift", Label: "Swift"},
		{Value: "typescript", Label: "TypeScript"},
	}
	return panel("Combobox", "A constrained picker and a multiple combobox. Removable chips reflect the selected values reported by the core.", func() *native.Node {
		var children []*native.Node
		source := pickerSource("Fruit", fruits)
		source.FilterMode = "contains"
		children = append(children, func() *native.Node {
			combobox27 := ui.NewCombobox(ui.ComboboxRootProps{
				Value:         fruit,
				OnValueChange: change(setFruit),
				AutoHighlight: ptr(true),
				PickerInputProps: ui.PickerInputProps{
					PickerSourceProps: source,
					Placeholder:       "Pick a fruit",
				},
			})
			return combobox27.Root().NativeNode()
		}())
		source = pickerSource("Tags", languages)
		source.FilterMode = "startsWith"
		children = append(children, func() *native.Node {
			combobox28 := ui.NewCombobox(ui.ComboboxRootProps{
				Multiple:       true,
				Values:         tags,
				OnValuesChange: change(setTags),
				AutoHighlight:  ptr(true),
				PickerInputProps: ui.PickerInputProps{
					PickerSourceProps: source,
					Placeholder:       "Add tags",
				},
			})
			return combobox28.Root().
				Children(func() *native.Node {
					var children []*native.Node
					chips := ui.UseComboboxChips()
					state := ui.UseComboboxState()
					children = append(children, combobox28.Chips(rowPart()).
						Children(func() *native.Node {
							return ui.For(
								chips,
								func(chip ui.ComboboxChip, index func() int) *native.Node {
									return combobox28.Chip(ui.ComboboxChipProps{
										Index: ptr(index()),
										PartProps: ui.PartProps{Style: ui.Style().
											Display("flex").
											AlignItems("center").
											Gap(4).
											PaddingLeft(8).
											PaddingRight(6).
											Height(22).
											BorderRadius(11).
											BackgroundColor(color(func(p palette) string {
												return p.Selection
											}))},
									}).
										Children(func() *native.Node {
											return ui.Fragment([]*native.Node{
												label(chip.Label).Node,
												combobox28.ChipRemove(ui.ComboboxChipProps{
													Index:     ptr(index()),
													PartProps: ui.PartProps{AriaLabel: "Remove " + chip.Label},
												}).
													Children("×").
													NativeNode(),
											})
										}).
										NativeNode()
								},
								func(chip ui.ComboboxChip) any {
									return chip.Value
								},
								nil,
							)
						}).
						NativeNode())
					children = append(children, note(func() string {
						return "open " + strconv.FormatBool(state().PopupOpen) + " · results " + strconv.Itoa(state().ResultCount)
					}).Node)
					return ui.Fragment(children)
				}).
				NativeNode()
		}())
		children = append(children, note(func() string {
			return "fruit " + textValue(fruit()) + " · tags [" + strings.Join(tags(), ", ") + "]"
		}).Node)
		return ui.Fragment(children)
	})
}
func SelectDemo() *ui.Element {
	theme, setTheme := ui.CreateSignal(ptr("system"))
	sizes, setSizes := ui.CreateSignal([]string{"m"})
	themes := []ui.OptionDeclaration{
		{Value: "light", Label: "Light"},
		{Value: "dark", Label: "Dark"},
		{Value: "system", Label: "Match system"},
	}
	options := []ui.OptionDeclaration{
		{Value: "s", Label: "Small"},
		{Value: "m", Label: "Medium"},
		{Value: "l", Label: "Large"},
		{Value: "xl", Label: "Extra large"},
	}
	return panel("Select", "Single and multiple selection. The popup is rendered from the declared appearance, with ordered selected values.", func() *native.Node {
		var children []*native.Node
		source := pickerSource("Theme", themes)
		source.Style = controlStyle()
		children = append(children, func() *native.Node {
			select29 := ui.NewSelect(ui.SelectRootProps{
				Value:             theme,
				OnValueChange:     change(setTheme),
				PickerSourceProps: source,
			})
			return select29.Root().
				Children(func() *native.Node {
					return ui.Fragment([]*native.Node{
						select29.Value(ui.PartProps{}).
							Children(func() *ui.Element {
								text := ui.UseSelectValueText()
								return label(func() string {
									return textValue(text())
								})
							}).
							NativeNode(),
						select29.Icon(ui.PartProps{}).Children("▾").NativeNode(),
					})
				}).
				NativeNode()
		}())
		source = pickerSource("Sizes", options)
		source.Style = controlStyle()
		children = append(children, func() *native.Node {
			select30 := ui.NewSelect(ui.SelectRootProps{
				Multiple:             true,
				Values:               sizes,
				OnValuesChange:       change(setSizes),
				AlignItemWithTrigger: ptr(true),
				PickerSourceProps:    source,
			})
			return select30.Root().
				Children(func() *native.Node {
					var children []*native.Node
					children = append(children, select30.Value(ui.PartProps{}).
						Children(func() *ui.Element {
							return label(func() string {
								return choose(len(sizes()) == 0, "Pick sizes", strings.Join(sizes(), ", "))
							})
						}).
						NativeNode())
					children = append(children, select30.Icon(ui.PartProps{}).Children("▾").NativeNode())
					children = append(children, select30.Positioner(ui.PickerPositionerProps{
						Side:       "bottom",
						Align:      "start",
						SideOffset: 6,
					}).
						Children(func() *native.Node {
							return ui.Fragment([]*native.Node{
								select30.ScrollUpArrow(ui.PartProps{}).NativeNode(),
								select30.ScrollDownArrow(ui.PartProps{}).NativeNode(),
							})
						}).
						NativeNode())
					state := ui.UseSelectState()
					children = append(children, note(func() string {
						return "popup " + strconv.FormatBool(state().PopupOpen) + " · side " + state().PopupSide + " · filled " + strconv.FormatBool(state().Filled) + " · touched " + strconv.FormatBool(state().Touched)
					}).Node)
					return ui.Fragment(children)
				}).
				NativeNode()
		}())
		children = append(children, note(func() string {
			return "theme " + textValue(theme()) + " · sizes [" + strings.Join(sizes(), ", ") + "]"
		}).Node)
		return ui.Fragment(children)
	})
}
