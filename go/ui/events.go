package ui

import (
	"encoding/json"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
)

// VisibleRange is the range of rows a collection is virtualizing.
type VisibleRange struct {
	Start int `json:"start"`
	End   int `json:"end"`
}

// CapturedPointerDetails describe a drag captured by the native pointer handler.
// Position and Origin stay in window coordinates even when the node moves.
type CapturedPointerDetails struct {
	Phase         string       `json:"phase"`
	Button        string       `json:"button"`
	Position      native.Point `json:"position"`
	Origin        native.Point `json:"origin"`
	LocalPosition native.Point `json:"localPosition"`
	LocalOrigin   native.Point `json:"localOrigin"`
	Delta         native.Point `json:"delta"`
}

// CapturedPointerFromEvent decodes the payload received by OnPointer.
func CapturedPointerFromEvent(event *native.Event) *CapturedPointerDetails {
	if event == nil || event.Type != protocol.EventPointer {
		return nil
	}
	return decodeEventJSON[CapturedPointerDetails](event)
}

// TableSortState is the table's current sort column and direction.
type TableSortState struct {
	Column    string `json:"column"`
	Direction string `json:"direction"`
}

// TableCell is one table cell coordinate.
type TableCell struct {
	Row    int `json:"row"`
	Column int `json:"column"`
}

// ColumnWidth is one resizable column's retained width.
type ColumnWidth struct {
	ID    string  `json:"id"`
	Width float64 `json:"width"`
}

// TabsIndicatorGeometry is the active tab's laid-out box.
type TabsIndicatorGeometry struct {
	Left   float64 `json:"left"`
	Top    float64 `json:"top"`
	Width  float64 `json:"width"`
	Height float64 `json:"height"`
}

// FieldValidationTriggers names which triggers the core's validation mode answers.
type FieldValidationTriggers struct {
	Change bool `json:"change"`
	Blur   bool `json:"blur"`
	Submit bool `json:"submit"`
}

// FieldValidationDelays is how long the core waits before validating on each trigger.
type FieldValidationDelays struct {
	Change *float64 `json:"change"`
	Blur   *float64 `json:"blur"`
	Submit *float64 `json:"submit"`
}

// FieldValidationDetails is which triggers validate a field and how long the core waits.
type FieldValidationDetails struct {
	Triggers FieldValidationTriggers `json:"triggers"`
	Delay    FieldValidationDelays   `json:"delay"`
}

// TableEditEndDetails is the end of one inline table edit.
type TableEditEndDetails struct {
	Row       int  `json:"row"`
	Column    int  `json:"column"`
	Committed bool `json:"committed"`
}

// ToastStackEntry is one toast's place in the core's own stack.
type ToastStackEntry struct {
	ID            string  `json:"id"`
	Index         int     `json:"index"`
	Type          string  `json:"type"`
	Limited       bool    `json:"limited"`
	Expanded      bool    `json:"expanded"`
	Swiping       bool    `json:"swiping"`
	SwipeMovement float64 `json:"swipeMovement"`
	Offset        float64 `json:"offset"`
}

// ScrollOffset is a scroll area's clamped offset.
type ScrollOffset struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
}

// Extent is a declared width and height the core's arithmetic needs.
type Extent struct {
	Width  float64
	Height float64
}

// MenuSelectDetails is the payload of a native menuselect event.
type MenuSelectDetails struct {
	ID      string `json:"id"`
	Checked *bool  `json:"checked,omitempty"`
	Submenu bool   `json:"submenu,omitempty"`
}

// PickerPartState is the core's own select or combobox part state.
type PickerPartState struct {
	PopupOpen   *bool  `json:"popupOpen,omitempty"`
	PopupSide   string `json:"popupSide,omitempty"`
	Pressed     *bool  `json:"pressed,omitempty"`
	Placeholder *bool  `json:"placeholder,omitempty"`
	Valid       *bool  `json:"valid,omitempty"`
	Invalid     *bool  `json:"invalid,omitempty"`
	Dirty       *bool  `json:"dirty,omitempty"`
	Touched     *bool  `json:"touched,omitempty"`
	Filled      *bool  `json:"filled,omitempty"`
	Focused     *bool  `json:"focused,omitempty"`
	ReadOnly    *bool  `json:"readOnly,omitempty"`
	Required    *bool  `json:"required,omitempty"`
	Status      string `json:"status,omitempty"`
	Empty       *bool  `json:"empty,omitempty"`
	ResultCount *int   `json:"resultCount,omitempty"`
}

// optional distinguishes a missing JSON key from an explicit null, matching the TypeScript types.
type optional[T any] struct {
	Value T
	Set   bool
	Null  bool
}

func (o *optional[T]) UnmarshalJSON(data []byte) error {
	o.Set = true
	if string(data) == "null" {
		o.Null = true
		var zero T
		o.Value = zero
		return nil
	}
	return json.Unmarshal(data, &o.Value)
}

func (o optional[T]) present() bool { return o.Set }

func (o optional[T]) ptr() *T {
	if !o.Set || o.Null {
		return nil
	}
	value := o.Value
	return &value
}

// AnchorPlacementDetails is the side and alignment an anchored surface really resolved to.
type AnchorPlacementDetails struct {
	Side            string  `json:"side"`
	Align           string  `json:"align"`
	AnchorHidden    bool    `json:"anchorHidden"`
	AnchorWidth     float64 `json:"anchorWidth"`
	AnchorHeight    float64 `json:"anchorHeight"`
	AvailableWidth  float64 `json:"availableWidth"`
	AvailableHeight float64 `json:"availableHeight"`
}

// ComponentChangeDetails is the payload of a native componentchange event.
type ComponentChangeDetails struct {
	Placement           *AnchorPlacementDetails         `json:"placement,omitempty"`
	ActivationDirection optional[string]                `json:"activationDirection"`
	Indicator           optional[TabsIndicatorGeometry] `json:"indicator"`
	OpenChangeComplete  *bool                           `json:"openChangeComplete,omitempty"`
	CheckedValues       []string                        `json:"checkedValues,omitempty"`
	Validation          *FieldValidationTriggers        `json:"validation,omitempty"`
	ValidationDelay     *FieldValidationDelays          `json:"validationDelay,omitempty"`
	Status              string                          `json:"status,omitempty"`
	DisplayValue        optional[string]                `json:"displayValue"`
	Completion          optional[float64]               `json:"completion"`
	Dragging            *bool                           `json:"dragging,omitempty"`
	Committed           *bool                           `json:"committed,omitempty"`
	Values              []float64                       `json:"values,omitempty"`
	Sizes               []float64                       `json:"sizes,omitempty"`
	Active              optional[string]                `json:"active"`
	Pressed             []string                        `json:"pressed,omitempty"`
	Value               optional[string]                `json:"value"`
	NumberValue         optional[float64]               `json:"numberValue"`
	InputValue          string                          `json:"inputValue,omitempty"`
	Open                *bool                           `json:"open,omitempty"`
	OpenIndex           optional[int]                   `json:"openIndex"`
	FocusedIndex        *int                            `json:"focusedIndex,omitempty"`
	Focused             optional[string]                `json:"focused"`
	Month               string                          `json:"month,omitempty"`
	Text                string                          `json:"text,omitempty"`
	Valid               *bool                           `json:"valid,omitempty"`
	Scrubbing           *bool                           `json:"scrubbing,omitempty"`
	ReadOnly            *bool                           `json:"readOnly,omitempty"`
	Required            *bool                           `json:"required,omitempty"`
	VisibleRange        *VisibleRange                   `json:"visibleRange,omitempty"`
	SelectedRanges      [][]int                         `json:"selectedRanges,omitempty"`
	Sort                optional[TableSortState]        `json:"sort"`
	ActiveCell          optional[TableCell]             `json:"activeCell"`
	ColumnWidths        []ColumnWidth                   `json:"columnWidths,omitempty"`
	ColumnOrder         []string                        `json:"columnOrder,omitempty"`
	EditEnded           *TableEditEndDetails            `json:"editEnded,omitempty"`
	Expanded            []string                        `json:"expanded,omitempty"`
	LoadChildren        string                          `json:"loadChildren,omitempty"`
	Dismissed           []string                        `json:"dismissed,omitempty"`
	Toasts              []ToastStackEntry               `json:"toasts,omitempty"`
	LoadingStatus       string                          `json:"loadingStatus,omitempty"`
	Complete            string                          `json:"complete,omitempty"`
	SnapPoint           *int                            `json:"snapPoint,omitempty"`
	Swiping             *bool                           `json:"swiping,omitempty"`
	SwipeOffset         *float64                        `json:"swipeOffset,omitempty"`
	Offset              *ScrollOffset                   `json:"offset,omitempty"`
	Scrolling           *bool                           `json:"scrolling,omitempty"`
	Hovering            *bool                           `json:"hovering,omitempty"`
	HasOverflowX        *bool                           `json:"hasOverflowX,omitempty"`
	HasOverflowY        *bool                           `json:"hasOverflowY,omitempty"`
	OverflowXStart      *bool                           `json:"overflowXStart,omitempty"`
	OverflowXEnd        *bool                           `json:"overflowXEnd,omitempty"`
	OverflowYStart      *bool                           `json:"overflowYStart,omitempty"`
	OverflowYEnd        *bool                           `json:"overflowYEnd,omitempty"`
	Side                string                          `json:"side,omitempty"`
	Align               string                          `json:"align,omitempty"`
	AnchorHidden        *bool                           `json:"anchorHidden,omitempty"`
	Highlighted         *bool                           `json:"highlighted,omitempty"`
	Disabled            *bool                           `json:"disabled,omitempty"`
	Checked             optional[bool]                  `json:"checked"`
	Activated           string                          `json:"activated,omitempty"`
	Href                string                          `json:"href,omitempty"`
	SelectedValues      []string                        `json:"selectedValues,omitempty"`
	ChipValues          []string                        `json:"chipValues,omitempty"`
	ChipLabels          []string                        `json:"chipLabels,omitempty"`
	ValueText           optional[string]                `json:"valueText"`
	State               *PickerPartState                `json:"state,omitempty"`
}

// CommitDetails is the payload of a native commit event.
type CommitDetails struct {
	Value       string   `json:"value,omitempty"`
	NumberValue *float64 `json:"numberValue,omitempty"`
	InputValue  string   `json:"inputValue,omitempty"`
	Row         *int     `json:"row,omitempty"`
	Column      *int     `json:"column,omitempty"`
}

func decodeEventJSON[T any](event *native.Event) *T {
	if event == nil {
		return nil
	}
	value, ok := event.ValueOK()
	if !ok {
		return nil
	}
	var decoded *T
	if err := json.Unmarshal([]byte(value), &decoded); err != nil {
		return nil
	}
	return decoded
}

// InputValue is the composed text of an input event, or the empty string.
func InputValue(event *native.Event) string {
	value, ok := event.ValueOK()
	if !ok {
		return ""
	}
	return value
}

// ComponentChangeFromEvent decodes a componentchange payload.
func ComponentChangeFromEvent(event *native.Event) *ComponentChangeDetails {
	return decodeEventJSON[ComponentChangeDetails](event)
}

// CommitFromEvent decodes a commit payload.
func CommitFromEvent(event *native.Event) *CommitDetails {
	return decodeEventJSON[CommitDetails](event)
}

// MenuSelectionFromEvent decodes a menuselect payload.
func MenuSelectionFromEvent(event *native.Event) *MenuSelectDetails {
	return decodeEventJSON[MenuSelectDetails](event)
}

// ActionFromEvent is the binding id an action event carries.
func ActionFromEvent(event *native.Event) string {
	value, ok := event.ValueOK()
	if !ok {
		return ""
	}
	return value
}

// ComponentItem is one ordered item of a toolbar or toggle group.
type ComponentItem struct {
	Value    string `json:"value"`
	Disabled bool   `json:"disabled,omitempty"`
}

// OptionEntry is one select or combobox option.
type OptionEntry struct {
	Value    string `json:"value"`
	Label    string `json:"label"`
	Group    string `json:"group,omitempty"`
	Disabled bool   `json:"disabled,omitempty"`
}
