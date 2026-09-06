package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// OptionDeclaration is one entry in a declared select, combobox, or autocomplete option source.
type OptionDeclaration struct {
	Value    string `json:"value"`
	Label    string `json:"label,omitempty"`
	Detail   string `json:"detail,omitempty"`
	Group    string `json:"group,omitempty"`
	Keywords string `json:"keywords,omitempty"`
	Disabled bool   `json:"disabled,omitempty"`
}

// PickerAppearance is structural geometry and paint for the rows the core renders.
type PickerAppearance struct {
	Width               *float64
	RowHeight           *float64
	MaxVisibleRows      *float64
	AnchorGap           *float64
	FontSize            *float64
	Radius              *float64
	Padding             *float64
	VerticalPadding     *float64
	Background          any
	Color               any
	HighlightBackground any
	HighlightColor      any
	SelectedBackground  any
	MutedColor          any
}

type encodedPickerAppearance struct {
	Width               *float64 `json:"width,omitempty"`
	RowHeight           *float64 `json:"rowHeight,omitempty"`
	MaxVisibleRows      *float64 `json:"maxVisibleRows,omitempty"`
	AnchorGap           *float64 `json:"anchorGap,omitempty"`
	FontSize            *float64 `json:"fontSize,omitempty"`
	Radius              *float64 `json:"radius,omitempty"`
	Padding             *float64 `json:"padding,omitempty"`
	VerticalPadding     *float64 `json:"verticalPadding,omitempty"`
	Background          *uint32  `json:"background,omitempty"`
	Color               *uint32  `json:"color,omitempty"`
	HighlightBackground *uint32  `json:"highlightBackground,omitempty"`
	HighlightColor      *uint32  `json:"highlightColor,omitempty"`
	SelectedBackground  *uint32  `json:"selectedBackground,omitempty"`
	MutedColor          *uint32  `json:"mutedColor,omitempty"`
}

func encodePickerAppearance(appearance PickerAppearance) encodedPickerAppearance {
	return encodedPickerAppearance{
		Width: appearance.Width, RowHeight: appearance.RowHeight, MaxVisibleRows: appearance.MaxVisibleRows,
		AnchorGap: appearance.AnchorGap, FontSize: appearance.FontSize, Radius: appearance.Radius,
		Padding: appearance.Padding, VerticalPadding: appearance.VerticalPadding,
		Background: packedColor(appearance.Background), Color: packedColor(appearance.Color),
		HighlightBackground: packedColor(appearance.HighlightBackground),
		HighlightColor:      packedColor(appearance.HighlightColor),
		SelectedBackground:  packedColor(appearance.SelectedBackground),
		MutedColor:          packedColor(appearance.MutedColor),
	}
}

// PickerFilterMode is Base UI's filter, answered by the core.
type PickerFilterMode string

const (
	PickerFilterFuzzy      PickerFilterMode = "fuzzy"
	PickerFilterContains   PickerFilterMode = "contains"
	PickerFilterStartsWith PickerFilterMode = "startsWith"
	PickerFilterNone       PickerFilterMode = "none"
)

type PickerSourceProps struct {
	PartProps
	Items        func() []OptionDeclaration
	Appearance   *PickerAppearance
	FilterMode   PickerFilterMode
	OnOpenChange func(bool, *native.Event)
	OnCommit     func(CommitDetails, *native.Event)
}

type SelectRootProps struct {
	PickerSourceProps
	Value                func() *string
	DefaultValue         string
	OnValueChange        func(*string, *native.Event)
	Multiple             bool
	Values               func() []string
	DefaultValues        []string
	OnValuesChange       func([]string, *native.Event)
	Required             bool
	ReadOnly             bool
	Modal                *bool
	AlignItemWithTrigger *bool
}

type PickerInputProps struct {
	PickerSourceProps
	Placeholder        string
	InputValue         string
	OnInputValueChange func(string, *native.Event)
}

type ComboboxRootProps struct {
	PickerInputProps
	Value                func() *string
	DefaultValue         string
	OnValueChange        func(*string, *native.Event)
	Multiple             bool
	Values               func() []string
	DefaultValues        []string
	OnValuesChange       func([]string, *native.Event)
	AutoHighlight        *bool
	OpenOnInputClick     *bool
	HighlightItemOnHover *bool
	LoopFocus            *bool
	ReadOnly             bool
	Required             bool
}

type AutocompleteRootProps struct {
	PickerInputProps
}

type PickerPositionerProps struct {
	PartProps
	Side       string
	Align      string
	SideOffset any
}

type ComboboxChipProps struct {
	PartProps
	Index *int
}

type OptionProps struct {
	PartProps
	Value     string
	Label     string
	ValueText string
	Group     string
}

// SelectPartState is everything the core decided about one select.
type SelectPartState struct {
	PopupOpen   bool
	PopupSide   string
	Pressed     bool
	Placeholder bool
	Valid       bool
	Invalid     bool
	Dirty       bool
	Touched     bool
	Filled      bool
	Focused     bool
	ReadOnly    bool
	Required    bool
}

func settledSelectState() SelectPartState {
	return SelectPartState{PopupSide: "bottom", Placeholder: true, Valid: true}
}

// ComboboxPartState is everything the core decided about one combobox.
type ComboboxPartState struct {
	PopupOpen   bool
	Pressed     bool
	Placeholder bool
	Valid       bool
	Invalid     bool
	Dirty       bool
	Touched     bool
	Filled      bool
	Focused     bool
	ReadOnly    bool
	Required    bool
	Status      string
	Empty       bool
	ResultCount int
}

func settledComboboxState() ComboboxPartState {
	return ComboboxPartState{Placeholder: true, Valid: true}
}

// ComboboxChip is one chip a multiple combobox holds.
type ComboboxChip struct {
	Value string
	Label string
}

func selectStateFromPart(part PickerPartState) SelectPartState {
	side := part.PopupSide
	if side == "" {
		side = "bottom"
	}
	return SelectPartState{
		PopupOpen: part.PopupOpen != nil && *part.PopupOpen, PopupSide: side,
		Pressed: part.Pressed != nil && *part.Pressed, Placeholder: part.Placeholder != nil && *part.Placeholder,
		Valid: part.Valid == nil || *part.Valid, Invalid: part.Invalid != nil && *part.Invalid,
		Dirty: part.Dirty != nil && *part.Dirty, Touched: part.Touched != nil && *part.Touched,
		Filled: part.Filled != nil && *part.Filled, Focused: part.Focused != nil && *part.Focused,
		ReadOnly: part.ReadOnly != nil && *part.ReadOnly, Required: part.Required != nil && *part.Required,
	}
}

func comboboxStateFromPart(part PickerPartState) ComboboxPartState {
	count := 0
	if part.ResultCount != nil {
		count = *part.ResultCount
	}
	return ComboboxPartState{
		PopupOpen: part.PopupOpen != nil && *part.PopupOpen, Pressed: part.Pressed != nil && *part.Pressed,
		Placeholder: part.Placeholder != nil && *part.Placeholder, Valid: part.Valid == nil || *part.Valid,
		Invalid: part.Invalid != nil && *part.Invalid, Dirty: part.Dirty != nil && *part.Dirty,
		Touched: part.Touched != nil && *part.Touched, Filled: part.Filled != nil && *part.Filled,
		Focused: part.Focused != nil && *part.Focused, ReadOnly: part.ReadOnly != nil && *part.ReadOnly,
		Required: part.Required != nil && *part.Required, Status: part.Status,
		Empty: part.Empty != nil && *part.Empty, ResultCount: count,
	}
}

type pickerState struct {
	scope     string
	select_   *reactive.Signal[SelectPartState]
	combobox  *reactive.Signal[ComboboxPartState]
	valueText *reactive.Signal[*string]
	chips     *reactive.Signal[[]ComboboxChip]
}

func newPickerState(prefix string) *pickerState {
	return &pickerState{
		scope:     createComponentScope(prefix),
		select_:   reactive.NewSignal(settledSelectState()),
		combobox:  reactive.NewSignal(settledComboboxState()),
		valueText: reactive.NewSignal[*string](nil),
		chips:     reactive.NewSignal([]ComboboxChip{}),
	}
}

var pickerContext = createPartContext[pickerState]()

func pickerScope(part string) string {
	context := pickerContext.Use()
	if context == nil {
		panic(part + " must be used inside its picker root")
	}
	return context.scope
}

// UseSelectState reads what the core decided about the enclosing select.
func UseSelectState() func() SelectPartState {
	context := pickerContext.Use()
	if context == nil {
		return settledSelectState
	}
	return func() SelectPartState { return context.select_.Read() }
}

// UseComboboxState reads what the core decided about the enclosing combobox or autocomplete.
func UseComboboxState() func() ComboboxPartState {
	context := pickerContext.Use()
	if context == nil {
		return settledComboboxState
	}
	return func() ComboboxPartState { return context.combobox.Read() }
}

// UseSelectValueText reads the joined label text a select's Value part renders.
func UseSelectValueText() func() *string {
	context := pickerContext.Use()
	if context == nil {
		return func() *string { return nil }
	}
	return func() *string { return context.valueText.Read() }
}

// UseComboboxChips reads the chips a multiple combobox holds, in chip order.
func UseComboboxChips() func() []ComboboxChip {
	context := pickerContext.Use()
	if context == nil {
		return func() []ComboboxChip { return nil }
	}
	return func() []ComboboxChip { return context.chips.Read() }
}

func applyPickerSource(node *native.Node, props PickerSourceProps) {
	if props.Items != nil {
		reactive.CreateRenderEffect(func() {
			setJson(node, protocol.Options, 524288, props.Items())
		})
	}
	if props.Appearance != nil {
		setJson(node, protocol.Appearance, 65536, encodePickerAppearance(*props.Appearance))
	}
	if props.FilterMode != "" {
		setString(node, protocol.FilterMode, string(props.FilterMode))
	}
	if props.OnCommit != nil {
		setListener(node, protocol.EventCommit, func(event *native.Event) {
			details := CommitFromEvent(event)
			if details != nil {
				props.OnCommit(*details, event)
			}
		})
	}
}

func createPickerPart(name, part string, props PartProps, tag uint8) *native.Node {
	node := createPart(tag, props)
	setPart(node, part, pickerScope(name), "")
	return finishPart(node, props)
}

func createPositionerPart(name, part string, props PickerPositionerProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, part, pickerScope(name), "")
	setString(node, protocol.Side, props.Side)
	setString(node, protocol.Align, props.Align)
	setNumber(node, protocol.SideOffset, props.SideOffset)
	return finishPart(node, props.PartProps)
}

func createOptionPart(name, part string, props OptionProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, part, pickerScope(name), props.Value)
	if props.Label != "" {
		setString(node, protocol.Value, props.Label)
	}
	if props.ValueText != "" {
		setString(node, protocol.ValueText, props.ValueText)
	}
	if props.Group != "" {
		setComponentValue(node, protocol.Group, props.Group)
	}
	return finishPart(node, props.PartProps)
}

func createChipPart(name, part string, props ComboboxChipProps, tag uint8) *native.Node {
	node := createPart(tag, props.PartProps)
	setPart(node, part, pickerScope(name), "")
	index := 0
	if props.Index != nil {
		index = *props.Index
	}
	setNumber(node, protocol.ItemIndex, index)
	return finishPart(node, props.PartProps)
}

func optionalStringSignal(defaultValue string) *reactive.Signal[*string] {
	var initial *string
	if defaultValue != "" {
		value := defaultValue
		initial = &value
	}
	return reactive.NewSignal(initial)
}

func controlledString(controlled func() *string, fallback *reactive.Signal[*string]) func() *string {
	return func() *string {
		if controlled != nil {
			return controlled()
		}
		return fallback.Read()
	}
}

func controlledStrings(controlled func() []string, fallback *reactive.Signal[[]string]) func() []string {
	return func() []string {
		if controlled != nil {
			return controlled()
		}
		return fallback.Read()
	}
}

func setActiveValue(node *native.Node, value *string) {
	if value == nil {
		setComponentValue(node, protocol.ActiveValue, "")
		return
	}
	setComponentValue(node, protocol.ActiveValue, *value)
}

// Select is a controlled select whose option list the core paints in its own window.
var Select = selectAPI{}

type selectAPI struct{}

func (selectAPI) Root(props SelectRootProps) *native.Node {
	state := newPickerState("qg-select")
	uncontrolled := optionalStringSignal(props.DefaultValue)
	uncontrolledValues := reactive.NewSignal(append([]string(nil), props.DefaultValues...))
	value := controlledString(props.Value, uncontrolled)
	values := controlledStrings(props.Values, uncontrolledValues)
	node := createButtonPart(props.PartProps)
	setPart(node, protocol.PartSelect, state.scope, "")
	applyPickerSource(node, props.PickerSourceProps)
	reactive.CreateRenderEffect(func() {
		if props.Multiple {
			setJson(node, protocol.Values, 65536, values())
			return
		}
		setActiveValue(node, value())
	})
	if props.Multiple {
		setExplicitBool(node, protocol.Multiple, true)
	}
	if props.Required {
		setExplicitBool(node, protocol.Required, true)
	}
	if props.ReadOnly {
		setExplicitBool(node, protocol.ReadOnly, true)
	}
	setExplicitBool(node, protocol.Modal, props.Modal)
	setExplicitBool(node, protocol.AlignItemWithTrigger, props.AlignItemWithTrigger)
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.State != nil {
			state.select_.Write(selectStateFromPart(*details.State))
		}
		if details.ValueText.present() {
			state.valueText.Write(details.ValueText.ptr())
		}
		if props.Multiple {
			if details.SelectedValues != nil {
				if props.Values == nil {
					uncontrolledValues.Write(details.SelectedValues)
				}
				if props.OnValuesChange != nil {
					props.OnValuesChange(details.SelectedValues, event)
				}
			}
		} else if details.Value.present() {
			next := details.Value.ptr()
			if props.Value == nil {
				uncontrolled.Write(next)
			}
			if props.OnValueChange != nil {
				props.OnValueChange(next, event)
			}
		}
		if details.Open != nil && props.OnOpenChange != nil {
			props.OnOpenChange(*details.Open, event)
		}
	})
	return reactive.Provide(pickerContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (selectAPI) Trigger(props SelectRootProps) *native.Node { return Select.Root(props) }

func (selectAPI) Option(props OptionProps) *native.Node {
	return createOptionPart("Select.Option", protocol.PartOption, props)
}

func (selectAPI) Label(props PartProps) *native.Node {
	return createPickerPart("Select.Label", protocol.PartSelectLabel, props, protocol.TagView)
}

func (selectAPI) Value(props PartProps) *native.Node {
	return createPickerPart("Select.Value", protocol.PartSelectValue, props, protocol.TagView)
}

func (selectAPI) Icon(props PartProps) *native.Node {
	return createPickerPart("Select.Icon", protocol.PartSelectIcon, props, protocol.TagView)
}

func (selectAPI) Backdrop(props PartProps) *native.Node {
	return createPickerPart("Select.Backdrop", protocol.PartSelectBackdrop, props, protocol.TagView)
}

func (selectAPI) Portal(props PickerPositionerProps) *native.Node {
	return createPositionerPart("Select.Portal", protocol.PartSelectPortal, props)
}

func (selectAPI) Positioner(props PickerPositionerProps) *native.Node {
	return createPositionerPart("Select.Positioner", protocol.PartSelectPositioner, props)
}

func (selectAPI) Popup(props PartProps) *native.Node {
	return createPickerPart("Select.Popup", protocol.PartSelectPopup, props, protocol.TagView)
}

func (selectAPI) Arrow(props PartProps) *native.Node {
	return createPickerPart("Select.Arrow", protocol.PartSelectArrow, props, protocol.TagView)
}

func (selectAPI) List(props PartProps) *native.Node {
	return createPickerPart("Select.List", protocol.PartSelectList, props, protocol.TagView)
}

func (selectAPI) Item(props OptionProps) *native.Node {
	return createOptionPart("Select.Item", protocol.PartSelectItem, props)
}

func (selectAPI) ItemText(props PartProps) *native.Node {
	return createPickerPart("Select.ItemText", protocol.PartSelectItemText, props, protocol.TagView)
}

func (selectAPI) ItemIndicator(props PartProps) *native.Node {
	return createPickerPart("Select.ItemIndicator", protocol.PartSelectItemIndicator, props, protocol.TagView)
}

func (selectAPI) Group(props OptionProps) *native.Node {
	return createOptionPart("Select.Group", protocol.PartSelectGroup, props)
}

func (selectAPI) GroupLabel(props PartProps) *native.Node {
	return createPickerPart("Select.GroupLabel", protocol.PartSelectGroupLabel, props, protocol.TagView)
}

func (selectAPI) Separator(props PartProps) *native.Node {
	return createPickerPart("Select.Separator", protocol.PartSelectSeparator, props, protocol.TagView)
}

func (selectAPI) ScrollUpArrow(props PartProps) *native.Node {
	return createPickerPart("Select.ScrollUpArrow", protocol.PartSelectScrollUpArrow, props, protocol.TagView)
}

func (selectAPI) ScrollDownArrow(props PartProps) *native.Node {
	return createPickerPart("Select.ScrollDownArrow", protocol.PartSelectScrollDownArrow, props, protocol.TagView)
}

func comboboxInputProps(props PickerInputProps) InputPartProps {
	return InputPartProps{
		PartProps: PartProps{
			Ref: props.Ref, Style: props.Style, Disabled: props.Disabled, Role: props.Role,
			AriaLabel: props.AriaLabel, TabIndex: props.TabIndex, OnClick: props.OnClick,
			OnMouseEnter: props.OnMouseEnter, OnMouseLeave: props.OnMouseLeave,
			OnMouseDown: props.OnMouseDown, OnMouseUp: props.OnMouseUp,
			OnKeyDown: props.OnKeyDown, OnKeyUp: props.OnKeyUp, OnFocus: props.OnFocus, OnBlur: props.OnBlur,
		},
		Placeholder: props.Placeholder,
	}
}

func providePickerInput(state *pickerState, input *native.Node, children PartChildren) *native.Node {
	return reactive.Provide(pickerContext, state, func() *native.Node {
		if children == nil {
			return Fragment([]*native.Node{input})
		}
		if child := children(); child != nil {
			return Fragment([]*native.Node{input, child})
		}
		return Fragment([]*native.Node{input})
	})
}

// Combobox is a constrained combobox. Arbitrary text is a query, not a committable value.
var Combobox = comboboxAPI{}

type comboboxAPI struct{}

func (comboboxAPI) Root(props ComboboxRootProps) *native.Node {
	state := newPickerState("qg-combobox")
	uncontrolled := optionalStringSignal(props.DefaultValue)
	uncontrolledValues := reactive.NewSignal(append([]string(nil), props.DefaultValues...))
	value := controlledString(props.Value, uncontrolled)
	values := controlledStrings(props.Values, uncontrolledValues)
	inputProps := comboboxInputProps(props.PickerInputProps)
	input := createPart(protocol.TagInput, inputProps.PartProps)
	applyInputPart(input, inputProps)
	setPart(input, protocol.PartCombobox, state.scope, "")
	applyPickerSource(input, props.PickerSourceProps)
	reactive.CreateRenderEffect(func() {
		setActiveValue(input, value())
	})
	if props.Multiple {
		reactive.CreateRenderEffect(func() {
			setJson(input, protocol.Values, 65536, values())
		})
		setExplicitBool(input, protocol.Multiple, true)
	}
	if props.InputValue != "" {
		setString(input, protocol.InputValue, props.InputValue)
	}
	setExplicitBool(input, protocol.AutoHighlight, props.AutoHighlight)
	setExplicitBool(input, protocol.OpenOnInputClick, props.OpenOnInputClick)
	setExplicitBool(input, protocol.HighlightItemOnHover, props.HighlightItemOnHover)
	setExplicitBool(input, protocol.LoopFocus, props.LoopFocus)
	if props.ReadOnly {
		setExplicitBool(input, protocol.ReadOnly, true)
	}
	if props.Required {
		setExplicitBool(input, protocol.Required, true)
	}
	setListener(input, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.State != nil {
			state.combobox.Write(comboboxStateFromPart(*details.State))
		}
		if details.ChipValues != nil {
			labels := details.ChipLabels
			chips := make([]ComboboxChip, 0, len(details.ChipValues))
			for index, chip := range details.ChipValues {
				label := chip
				if index < len(labels) {
					label = labels[index]
				}
				chips = append(chips, ComboboxChip{Value: chip, Label: label})
			}
			state.chips.Write(chips)
			if props.Multiple {
				if props.Values == nil {
					uncontrolledValues.Write(details.ChipValues)
				}
				if props.OnValuesChange != nil {
					props.OnValuesChange(details.ChipValues, event)
				}
			}
		}
		if details.Value.present() {
			next := details.Value.ptr()
			if props.Value == nil {
				uncontrolled.Write(next)
			}
			if props.OnValueChange != nil {
				props.OnValueChange(next, event)
			}
		}
		if details.InputValue != "" && props.OnInputValueChange != nil {
			props.OnInputValueChange(details.InputValue, event)
		}
		if details.Open != nil && props.OnOpenChange != nil {
			props.OnOpenChange(*details.Open, event)
		}
	})
	if props.Ref != nil {
		props.Ref(input)
	}
	return providePickerInput(state, input, props.Children)
}

func (comboboxAPI) Input(props ComboboxRootProps) *native.Node { return Combobox.Root(props) }

func (comboboxAPI) Option(props OptionProps) *native.Node {
	return createOptionPart("Combobox.Option", protocol.PartOption, props)
}

func (comboboxAPI) Label(props PartProps) *native.Node {
	return createPickerPart("Combobox.Label", protocol.PartComboboxLabel, props, protocol.TagView)
}

func (comboboxAPI) Value(props PartProps) *native.Node {
	return createPickerPart("Combobox.Value", protocol.PartComboboxValue, props, protocol.TagView)
}

func (comboboxAPI) Icon(props PartProps) *native.Node {
	return createPickerPart("Combobox.Icon", protocol.PartComboboxIcon, props, protocol.TagView)
}

func (comboboxAPI) InputGroup(props PartProps) *native.Node {
	return createPickerPart("Combobox.InputGroup", protocol.PartComboboxInputGroup, props, protocol.TagView)
}

func (comboboxAPI) Clear(props PartProps) *native.Node {
	return createPickerPart("Combobox.Clear", protocol.PartComboboxClear, props, protocol.TagButton)
}

func (comboboxAPI) Trigger(props PartProps) *native.Node {
	return createPickerPart("Combobox.Trigger", protocol.PartComboboxTrigger, props, protocol.TagButton)
}

func (comboboxAPI) Chips(props PartProps) *native.Node {
	return createPickerPart("Combobox.Chips", protocol.PartComboboxChips, props, protocol.TagView)
}

func (comboboxAPI) Chip(props ComboboxChipProps) *native.Node {
	return createChipPart("Combobox.Chip", protocol.PartComboboxChip, props, protocol.TagView)
}

func (comboboxAPI) ChipRemove(props ComboboxChipProps) *native.Node {
	return createChipPart("Combobox.ChipRemove", protocol.PartComboboxChipRemove, props, protocol.TagButton)
}

func (comboboxAPI) Backdrop(props PartProps) *native.Node {
	return createPickerPart("Combobox.Backdrop", protocol.PartComboboxBackdrop, props, protocol.TagView)
}

func (comboboxAPI) Portal(props PickerPositionerProps) *native.Node {
	return createPositionerPart("Combobox.Portal", protocol.PartComboboxPortal, props)
}

func (comboboxAPI) Positioner(props PickerPositionerProps) *native.Node {
	return createPositionerPart("Combobox.Positioner", protocol.PartComboboxPositioner, props)
}

func (comboboxAPI) Popup(props PartProps) *native.Node {
	return createPickerPart("Combobox.Popup", protocol.PartComboboxPopup, props, protocol.TagView)
}

func (comboboxAPI) Arrow(props PartProps) *native.Node {
	return createPickerPart("Combobox.Arrow", protocol.PartComboboxArrow, props, protocol.TagView)
}

func (comboboxAPI) Status(props PartProps) *native.Node {
	return createPickerPart("Combobox.Status", protocol.PartComboboxStatus, props, protocol.TagView)
}

func (comboboxAPI) Empty(props PartProps) *native.Node {
	return createPickerPart("Combobox.Empty", protocol.PartComboboxEmpty, props, protocol.TagView)
}

func (comboboxAPI) List(props PartProps) *native.Node {
	return createPickerPart("Combobox.List", protocol.PartComboboxList, props, protocol.TagView)
}

func (comboboxAPI) Row(props PartProps) *native.Node {
	return createPickerPart("Combobox.Row", protocol.PartComboboxRow, props, protocol.TagView)
}

func (comboboxAPI) Item(props OptionProps) *native.Node {
	return createOptionPart("Combobox.Item", protocol.PartComboboxItem, props)
}

func (comboboxAPI) ItemIndicator(props PartProps) *native.Node {
	return createPickerPart("Combobox.ItemIndicator", protocol.PartComboboxItemIndicator, props, protocol.TagView)
}

func (comboboxAPI) Group(props OptionProps) *native.Node {
	return createOptionPart("Combobox.Group", protocol.PartComboboxGroup, props)
}

func (comboboxAPI) GroupLabel(props PartProps) *native.Node {
	return createPickerPart("Combobox.GroupLabel", protocol.PartComboboxGroupLabel, props, protocol.TagView)
}

func (comboboxAPI) Collection(props PartProps) *native.Node {
	return createPickerPart("Combobox.Collection", protocol.PartComboboxCollection, props, protocol.TagView)
}

func (comboboxAPI) Separator(props PartProps) *native.Node {
	return createPickerPart("Combobox.Separator", protocol.PartComboboxSeparator, props, protocol.TagView)
}

// Autocomplete is a free-form autocomplete sharing the combobox's owner-window parts.
var Autocomplete = autocompleteAPI{}

type autocompleteAPI struct{}

func (autocompleteAPI) Root(props AutocompleteRootProps) *native.Node {
	state := newPickerState("qg-autocomplete")
	inputProps := comboboxInputProps(props.PickerInputProps)
	input := createPart(protocol.TagInput, inputProps.PartProps)
	applyInputPart(input, inputProps)
	setPart(input, protocol.PartAutocomplete, state.scope, "")
	applyPickerSource(input, props.PickerSourceProps)
	if props.InputValue != "" {
		setString(input, protocol.InputValue, props.InputValue)
	}
	setListener(input, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.State != nil {
			state.combobox.Write(comboboxStateFromPart(*details.State))
		}
		if details.InputValue != "" && props.OnInputValueChange != nil {
			props.OnInputValueChange(details.InputValue, event)
		}
		if details.Open != nil && props.OnOpenChange != nil {
			props.OnOpenChange(*details.Open, event)
		}
	})
	if props.Ref != nil {
		props.Ref(input)
	}
	return providePickerInput(state, input, props.Children)
}

func (autocompleteAPI) Input(props AutocompleteRootProps) *native.Node {
	return Autocomplete.Root(props)
}

func (autocompleteAPI) Option(props OptionProps) *native.Node {
	return createOptionPart("Autocomplete.Option", protocol.PartOption, props)
}

func (autocompleteAPI) Label(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Label", protocol.PartComboboxLabel, props, protocol.TagView)
}

func (autocompleteAPI) Value(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Value", protocol.PartComboboxValue, props, protocol.TagView)
}

func (autocompleteAPI) Icon(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Icon", protocol.PartComboboxIcon, props, protocol.TagView)
}

func (autocompleteAPI) InputGroup(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.InputGroup", protocol.PartComboboxInputGroup, props, protocol.TagView)
}

func (autocompleteAPI) Clear(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Clear", protocol.PartComboboxClear, props, protocol.TagButton)
}

func (autocompleteAPI) Portal(props PickerPositionerProps) *native.Node {
	return createPositionerPart("Autocomplete.Portal", protocol.PartComboboxPortal, props)
}

func (autocompleteAPI) Positioner(props PickerPositionerProps) *native.Node {
	return createPositionerPart("Autocomplete.Positioner", protocol.PartComboboxPositioner, props)
}

func (autocompleteAPI) Popup(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Popup", protocol.PartComboboxPopup, props, protocol.TagView)
}

func (autocompleteAPI) Arrow(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Arrow", protocol.PartComboboxArrow, props, protocol.TagView)
}

func (autocompleteAPI) Status(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Status", protocol.PartComboboxStatus, props, protocol.TagView)
}

func (autocompleteAPI) Empty(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Empty", protocol.PartComboboxEmpty, props, protocol.TagView)
}

func (autocompleteAPI) List(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.List", protocol.PartComboboxList, props, protocol.TagView)
}

func (autocompleteAPI) Item(props OptionProps) *native.Node {
	return createOptionPart("Autocomplete.Item", protocol.PartComboboxItem, props)
}

func (autocompleteAPI) ItemIndicator(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.ItemIndicator", protocol.PartComboboxItemIndicator, props, protocol.TagView)
}

func (autocompleteAPI) Group(props OptionProps) *native.Node {
	return createOptionPart("Autocomplete.Group", protocol.PartComboboxGroup, props)
}

func (autocompleteAPI) GroupLabel(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.GroupLabel", protocol.PartComboboxGroupLabel, props, protocol.TagView)
}

func (autocompleteAPI) Collection(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Collection", protocol.PartComboboxCollection, props, protocol.TagView)
}

func (autocompleteAPI) Separator(props PartProps) *native.Node {
	return createPickerPart("Autocomplete.Separator", protocol.PartComboboxSeparator, props, protocol.TagView)
}
