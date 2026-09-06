package ui

import (
	"encoding/json"
	"fmt"
	"math"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
)

// Style is the host style record, including nested interaction states.
type Style struct {
	Display         string
	Flex            any
	FlexDirection   string
	FlexWrap        string
	FlexGrow        any
	FlexShrink      any
	FlexBasis       any
	AlignItems      string
	AlignSelf       string
	JustifyContent  string
	AlignContent    string
	Gap             any
	ColumnGap       any
	RowGap          any
	Width           any
	Height          any
	MinWidth        any
	MinHeight       any
	MaxWidth        any
	MaxHeight       any
	Padding         any
	PaddingTop      any
	PaddingRight    any
	PaddingBottom   any
	PaddingLeft     any
	Margin          any
	MarginTop       any
	MarginRight     any
	MarginBottom    any
	MarginLeft      any
	BackgroundColor any
	Color           any
	Opacity         any
	BorderWidth     any
	BorderColor     any
	BorderRadius    any
	FontSize        any
	FontFamily      string
	FontWeight      any
	LineHeight      any
	TextAlign       string
	Cursor          string
	AppRegion       string
	UserSelect      string
	Position        string
	Top             any
	Right           any
	Bottom          any
	Left            any
	Hover           *Style
	Active          *Style
	Focus           *Style
	Disabled        *Style
}

func setLength(node *native.Node, code uint16, value any) {
	if value == nil {
		native.ClearProperty(node, code)
		return
	}
	switch typed := value.(type) {
	case int:
		native.SetNumber(node, code, float32(typed))
	case int32:
		native.SetNumber(node, code, float32(typed))
	case int64:
		native.SetNumber(node, code, float32(typed))
	case float32:
		if !isFinite(float64(typed)) {
			native.ClearProperty(node, code)
			return
		}
		native.SetNumber(node, code, typed)
	case float64:
		if !isFinite(typed) {
			native.ClearProperty(node, code)
			return
		}
		native.SetNumber(node, code, float32(typed))
	case string:
		normalized := normalizeLength(typed)
		if number, ok := normalized.(float64); ok {
			native.SetNumber(node, code, float32(number))
		} else if text := normalized.(string); text == "" {
			native.ClearProperty(node, code)
		} else {
			native.SetString(node, code, text)
		}
	default:
		panic(fmt.Sprintf("QuickGUI length %v is not a number or string", value))
	}
}

func setNumber(node *native.Node, code uint16, value any) {
	if value == nil {
		native.ClearProperty(node, code)
		return
	}
	number := toFloat(value)
	if !isFinite(number) {
		native.ClearProperty(node, code)
		return
	}
	native.SetNumber(node, code, float32(number))
}

func setString(node *native.Node, code uint16, value string) {
	if value == "" {
		native.ClearProperty(node, code)
		return
	}
	native.SetString(node, code, value)
}

func setColor(node *native.Node, code uint16, value any) {
	if value == nil || value == "" {
		native.ClearProperty(node, code)
		return
	}
	native.SetColor(node, code, native.ParseColor(value))
}

func setFlex(node *native.Node, value any) {
	if value == nil {
		native.ClearProperty(node, protocol.FlexGrow)
		native.ClearProperty(node, protocol.FlexShrink)
		native.ClearProperty(node, protocol.FlexBasis)
		return
	}
	switch typed := value.(type) {
	case int, int32, int64, float32, float64:
		native.SetNumber(node, protocol.FlexGrow, float32(toFloat(typed)))
		native.SetNumber(node, protocol.FlexShrink, 1)
		native.ClearProperty(node, protocol.FlexBasis)
	case string:
		native.SetString(node, protocol.FlexGrow, typed)
	default:
		panic(fmt.Sprintf("QuickGUI flex %v is not a number or string", value))
	}
}

func applyStyle(node *native.Node, style Style) {
	if style.Display != "" {
		setString(node, protocol.Display, style.Display)
	}
	if style.Flex != nil {
		setFlex(node, style.Flex)
	}
	if style.FlexDirection != "" {
		setString(node, protocol.FlexDirection, style.FlexDirection)
	}
	if style.FlexWrap != "" {
		setString(node, protocol.FlexWrap, style.FlexWrap)
	}
	if style.FlexGrow != nil {
		setNumber(node, protocol.FlexGrow, style.FlexGrow)
	}
	if style.FlexShrink != nil {
		setNumber(node, protocol.FlexShrink, style.FlexShrink)
	}
	if style.FlexBasis != nil {
		setLength(node, protocol.FlexBasis, style.FlexBasis)
	}
	if style.AlignItems != "" {
		setString(node, protocol.AlignItems, style.AlignItems)
	}
	if style.AlignSelf != "" {
		setString(node, protocol.AlignSelf, style.AlignSelf)
	}
	if style.JustifyContent != "" {
		setString(node, protocol.JustifyContent, style.JustifyContent)
	}
	if style.AlignContent != "" {
		setString(node, protocol.AlignContent, style.AlignContent)
	}
	if style.Gap != nil {
		setLength(node, protocol.Gap, style.Gap)
	}
	if style.ColumnGap != nil {
		setLength(node, protocol.ColumnGap, style.ColumnGap)
	}
	if style.RowGap != nil {
		setLength(node, protocol.RowGap, style.RowGap)
	}
	if style.Width != nil {
		setLength(node, protocol.Width, style.Width)
	}
	if style.Height != nil {
		setLength(node, protocol.Height, style.Height)
	}
	if style.MinWidth != nil {
		setLength(node, protocol.MinWidth, style.MinWidth)
	}
	if style.MinHeight != nil {
		setLength(node, protocol.MinHeight, style.MinHeight)
	}
	if style.MaxWidth != nil {
		setLength(node, protocol.MaxWidth, style.MaxWidth)
	}
	if style.MaxHeight != nil {
		setLength(node, protocol.MaxHeight, style.MaxHeight)
	}
	if style.Padding != nil {
		setLength(node, protocol.Padding, style.Padding)
	}
	if style.PaddingTop != nil {
		setLength(node, protocol.PaddingTop, style.PaddingTop)
	}
	if style.PaddingRight != nil {
		setLength(node, protocol.PaddingRight, style.PaddingRight)
	}
	if style.PaddingBottom != nil {
		setLength(node, protocol.PaddingBottom, style.PaddingBottom)
	}
	if style.PaddingLeft != nil {
		setLength(node, protocol.PaddingLeft, style.PaddingLeft)
	}
	if style.Margin != nil {
		setLength(node, protocol.Margin, style.Margin)
	}
	if style.MarginTop != nil {
		setLength(node, protocol.MarginTop, style.MarginTop)
	}
	if style.MarginRight != nil {
		setLength(node, protocol.MarginRight, style.MarginRight)
	}
	if style.MarginBottom != nil {
		setLength(node, protocol.MarginBottom, style.MarginBottom)
	}
	if style.MarginLeft != nil {
		setLength(node, protocol.MarginLeft, style.MarginLeft)
	}
	if style.BackgroundColor != nil {
		setColor(node, protocol.BackgroundColor, style.BackgroundColor)
	}
	if style.Color != nil {
		setColor(node, protocol.Color, style.Color)
	}
	if style.Opacity != nil {
		setNumber(node, protocol.Opacity, style.Opacity)
	}
	if style.BorderWidth != nil {
		setLength(node, protocol.BorderWidth, style.BorderWidth)
	}
	if style.BorderColor != nil {
		setColor(node, protocol.BorderColor, style.BorderColor)
	}
	if style.BorderRadius != nil {
		setLength(node, protocol.BorderRadius, style.BorderRadius)
	}
	if style.FontSize != nil {
		setLength(node, protocol.FontSize, style.FontSize)
	}
	if style.FontFamily != "" {
		setString(node, protocol.FontFamily, style.FontFamily)
	}
	if style.FontWeight != nil {
		setLength(node, protocol.FontWeight, style.FontWeight)
	}
	if style.LineHeight != nil {
		setLength(node, protocol.LineHeight, style.LineHeight)
	}
	if style.TextAlign != "" {
		setString(node, protocol.TextAlign, style.TextAlign)
	}
	if style.Cursor != "" {
		setString(node, protocol.Cursor, style.Cursor)
	}
	if style.AppRegion != "" {
		setString(node, protocol.AppRegion, style.AppRegion)
	}
	if style.UserSelect != "" {
		setString(node, protocol.UserSelect, style.UserSelect)
	}
	if style.Position != "" {
		setString(node, protocol.Position, style.Position)
	}
	if style.Top != nil {
		setLength(node, protocol.Top, style.Top)
	}
	if style.Right != nil {
		setLength(node, protocol.Right, style.Right)
	}
	if style.Bottom != nil {
		setLength(node, protocol.Bottom, style.Bottom)
	}
	if style.Left != nil {
		setLength(node, protocol.Left, style.Left)
	}
	if style.Hover != nil {
		setStateStyle(node, protocol.HoverStyle, "hover", style.Hover)
	}
	if style.Active != nil {
		setStateStyle(node, protocol.ActiveStyle, "active", style.Active)
	}
	if style.Focus != nil {
		setStateStyle(node, protocol.FocusStyle, "focus", style.Focus)
	}
	if style.Disabled != nil {
		setStateStyle(node, protocol.DisabledStyle, "disabled", style.Disabled)
	}
}

type encodedStateStyle struct {
	BackgroundColor *uint32  `json:"backgroundColor,omitempty"`
	Color           *uint32  `json:"color,omitempty"`
	BorderColor     *uint32  `json:"borderColor,omitempty"`
	BorderWidth     *float64 `json:"borderWidth,omitempty"`
	BorderRadius    *float64 `json:"borderRadius,omitempty"`
	Opacity         *float64 `json:"opacity,omitempty"`
	Cursor          string   `json:"cursor,omitempty"`
}

func setStateStyle(node *native.Node, code uint16, state string, style *Style) {
	encoded := encodedStateStyle{}
	anyField := false
	if style.BackgroundColor != nil {
		color := native.ParseColor(style.BackgroundColor)
		encoded.BackgroundColor = &color
		anyField = true
	}
	if style.Color != nil {
		color := native.ParseColor(style.Color)
		encoded.Color = &color
		anyField = true
	}
	if style.BorderColor != nil {
		color := native.ParseColor(style.BorderColor)
		encoded.BorderColor = &color
		anyField = true
	}
	if style.BorderWidth != nil {
		width := stateLength(state, "borderWidth", style.BorderWidth)
		encoded.BorderWidth = &width
		anyField = true
	}
	if style.BorderRadius != nil {
		radius := stateLength(state, "borderRadius", style.BorderRadius)
		encoded.BorderRadius = &radius
		anyField = true
	}
	if style.Opacity != nil {
		opacity := toFloat(style.Opacity)
		encoded.Opacity = &opacity
		anyField = true
	}
	if style.Cursor != "" {
		encoded.Cursor = style.Cursor
		anyField = true
	}
	if !anyField {
		native.ClearProperty(node, code)
		return
	}
	payload, err := json.Marshal(encoded)
	if err != nil {
		panic(err)
	}
	if len(payload) > protocol.MaxStateStyleJSONBytes {
		panic(fmt.Sprintf("QuickGUI state style declarations are bounded to %d bytes", protocol.MaxStateStyleJSONBytes))
	}
	native.SetString(node, code, string(payload))
}

func stateLength(state, name string, value any) float64 {
	switch typed := value.(type) {
	case int:
		return float64(typed)
	case float64:
		return typed
	case float32:
		return float64(typed)
	case string:
		if number, ok := normalizeLength(typed).(float64); ok {
			return number
		}
	}
	panic(fmt.Sprintf("QuickGUI `%s` %s must be a number of logical pixels", state, name))
}

func normalizeLength(value string) any {
	trimmed := strings.TrimSpace(value)
	if strings.HasSuffix(trimmed, "px") {
		number, err := strconv.ParseFloat(trimmed[:len(trimmed)-2], 64)
		if err == nil && isFinite(number) {
			return number
		}
		return trimmed
	}
	if trimmed == "0" {
		return 0.0
	}
	number, err := strconv.ParseFloat(trimmed, 64)
	if err == nil && isFinite(number) && trimmed != "" {
		return number
	}
	return trimmed
}

func toFloat(value any) float64 {
	switch typed := value.(type) {
	case int:
		return float64(typed)
	case int32:
		return float64(typed)
	case int64:
		return float64(typed)
	case float32:
		return float64(typed)
	case float64:
		return typed
	default:
		panic(fmt.Sprintf("QuickGUI expected a number, got %v", value))
	}
}

func isFinite(value float64) bool {
	return !math.IsNaN(value) && !math.IsInf(value, 0)
}
