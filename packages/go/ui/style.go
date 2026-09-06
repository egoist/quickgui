package ui

import (
	"encoding/json"
	"fmt"
	"math"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
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
	Overflow        string
	OverflowX       string
	OverflowY       string
	Visibility      string
	LetterSpacing   any
	TextTransform   string
	LineClamp       any
	WhiteSpace      string
	TextOverflow    string
	Top             any
	Right           any
	Bottom          any
	Left            any
	Outline         string
	OutlineWidth    any
	OutlineColor    any
	OutlineOffset   any
	OutlineStyle    string
	Transform       string
	TransformOrigin string
	Hover           *Style
	Active          *Style
	Focus           *Style
	Disabled        *Style
	Selected        *Style
	GroupHover      *Style
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

func mergeStyle(target *Style, source Style) {
	if source.Display != "" {
		target.Display = source.Display
	}
	if source.Flex != nil {
		target.Flex = source.Flex
	}
	if source.FlexDirection != "" {
		target.FlexDirection = source.FlexDirection
	}
	if source.FlexWrap != "" {
		target.FlexWrap = source.FlexWrap
	}
	if source.FlexGrow != nil {
		target.FlexGrow = source.FlexGrow
	}
	if source.FlexShrink != nil {
		target.FlexShrink = source.FlexShrink
	}
	if source.FlexBasis != nil {
		target.FlexBasis = source.FlexBasis
	}
	if source.AlignItems != "" {
		target.AlignItems = source.AlignItems
	}
	if source.AlignSelf != "" {
		target.AlignSelf = source.AlignSelf
	}
	if source.JustifyContent != "" {
		target.JustifyContent = source.JustifyContent
	}
	if source.AlignContent != "" {
		target.AlignContent = source.AlignContent
	}
	if source.Gap != nil {
		target.Gap = source.Gap
	}
	if source.ColumnGap != nil {
		target.ColumnGap = source.ColumnGap
	}
	if source.RowGap != nil {
		target.RowGap = source.RowGap
	}
	if source.Width != nil {
		target.Width = source.Width
	}
	if source.Height != nil {
		target.Height = source.Height
	}
	if source.MinWidth != nil {
		target.MinWidth = source.MinWidth
	}
	if source.MinHeight != nil {
		target.MinHeight = source.MinHeight
	}
	if source.MaxWidth != nil {
		target.MaxWidth = source.MaxWidth
	}
	if source.MaxHeight != nil {
		target.MaxHeight = source.MaxHeight
	}
	if source.Padding != nil {
		target.Padding = source.Padding
	}
	if source.PaddingTop != nil {
		target.PaddingTop = source.PaddingTop
	}
	if source.PaddingRight != nil {
		target.PaddingRight = source.PaddingRight
	}
	if source.PaddingBottom != nil {
		target.PaddingBottom = source.PaddingBottom
	}
	if source.PaddingLeft != nil {
		target.PaddingLeft = source.PaddingLeft
	}
	if source.Margin != nil {
		target.Margin = source.Margin
	}
	if source.MarginTop != nil {
		target.MarginTop = source.MarginTop
	}
	if source.MarginRight != nil {
		target.MarginRight = source.MarginRight
	}
	if source.MarginBottom != nil {
		target.MarginBottom = source.MarginBottom
	}
	if source.MarginLeft != nil {
		target.MarginLeft = source.MarginLeft
	}
	if source.BackgroundColor != nil {
		target.BackgroundColor = source.BackgroundColor
	}
	if source.Color != nil {
		target.Color = source.Color
	}
	if source.Opacity != nil {
		target.Opacity = source.Opacity
	}
	if source.BorderWidth != nil {
		target.BorderWidth = source.BorderWidth
	}
	if source.BorderColor != nil {
		target.BorderColor = source.BorderColor
	}
	if source.BorderRadius != nil {
		target.BorderRadius = source.BorderRadius
	}
	if source.FontSize != nil {
		target.FontSize = source.FontSize
	}
	if source.FontFamily != "" {
		target.FontFamily = source.FontFamily
	}
	if source.FontWeight != nil {
		target.FontWeight = source.FontWeight
	}
	if source.LineHeight != nil {
		target.LineHeight = source.LineHeight
	}
	if source.TextAlign != "" {
		target.TextAlign = source.TextAlign
	}
	if source.Cursor != "" {
		target.Cursor = source.Cursor
	}
	if source.AppRegion != "" {
		target.AppRegion = source.AppRegion
	}
	if source.UserSelect != "" {
		target.UserSelect = source.UserSelect
	}
	if source.Position != "" {
		target.Position = source.Position
	}
	if source.Overflow != "" {
		target.Overflow = source.Overflow
	}
	if source.OverflowX != "" {
		target.OverflowX = source.OverflowX
	}
	if source.OverflowY != "" {
		target.OverflowY = source.OverflowY
	}
	if source.Visibility != "" {
		target.Visibility = source.Visibility
	}
	if source.LetterSpacing != nil {
		target.LetterSpacing = source.LetterSpacing
	}
	if source.TextTransform != "" {
		target.TextTransform = source.TextTransform
	}
	if source.LineClamp != nil {
		target.LineClamp = source.LineClamp
	}
	if source.WhiteSpace != "" {
		target.WhiteSpace = source.WhiteSpace
	}
	if source.TextOverflow != "" {
		target.TextOverflow = source.TextOverflow
	}
	if source.Top != nil {
		target.Top = source.Top
	}
	if source.Right != nil {
		target.Right = source.Right
	}
	if source.Bottom != nil {
		target.Bottom = source.Bottom
	}
	if source.Left != nil {
		target.Left = source.Left
	}
	if source.Hover != nil {
		target.Hover = source.Hover
	}
	if source.Active != nil {
		target.Active = source.Active
	}
	if source.Focus != nil {
		target.Focus = source.Focus
	}
	if source.Disabled != nil {
		target.Disabled = source.Disabled
	}
	if source.Selected != nil {
		target.Selected = source.Selected
	}
	if source.GroupHover != nil {
		target.GroupHover = source.GroupHover
	}
	if source.Outline != "" {
		target.Outline = source.Outline
	}
	if source.OutlineWidth != nil {
		target.OutlineWidth = source.OutlineWidth
	}
	if source.OutlineColor != nil {
		target.OutlineColor = source.OutlineColor
	}
	if source.OutlineOffset != nil {
		target.OutlineOffset = source.OutlineOffset
	}
	if source.OutlineStyle != "" {
		target.OutlineStyle = source.OutlineStyle
	}
	if source.Transform != "" {
		target.Transform = source.Transform
	}
	if source.TransformOrigin != "" {
		target.TransformOrigin = source.TransformOrigin
	}
}

func applyStyleList(node *native.Node, styles []Style) Style {
	merged := Style{}
	for _, style := range styles {
		mergeStyle(&merged, style)
	}
	applyStyle(node, merged)
	return merged
}

func bindStyleList(node *native.Node, styles func() []Style) {
	reactive.CreateRenderEffect(func() {
		applyStyleList(node, styles())
	})
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
	if style.Overflow != "" {
		setString(node, protocol.Overflow, style.Overflow)
	}
	if style.OverflowX != "" {
		setString(node, protocol.OverflowX, style.OverflowX)
	}
	if style.OverflowY != "" {
		setString(node, protocol.OverflowY, style.OverflowY)
	}
	if style.Visibility != "" {
		setString(node, protocol.Visibility, style.Visibility)
	}
	if style.LetterSpacing != nil {
		setLength(node, protocol.LetterSpacing, style.LetterSpacing)
	}
	if style.TextTransform != "" {
		setString(node, protocol.TextTransform, style.TextTransform)
	}
	if style.LineClamp != nil {
		setNumber(node, protocol.LineClamp, style.LineClamp)
	}
	if style.WhiteSpace != "" {
		setString(node, protocol.WhiteSpace, style.WhiteSpace)
	}
	if style.TextOverflow != "" {
		setString(node, protocol.TextOverflow, style.TextOverflow)
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
	if style.Selected != nil {
		setStateStyle(node, protocol.SelectedStyle, "selected", style.Selected)
	}
	if style.GroupHover != nil {
		setStateStyle(node, protocol.GroupHoverStyle, "groupHover", style.GroupHover)
	}
	if style.Outline != "" {
		applyOutlineShorthand(node, style.Outline)
	}
	if style.OutlineWidth != nil {
		setLength(node, protocol.OutlineWidth, style.OutlineWidth)
	}
	if style.OutlineColor != nil {
		setColor(node, protocol.OutlineColor, style.OutlineColor)
	}
	if style.OutlineOffset != nil {
		setLength(node, protocol.OutlineOffset, style.OutlineOffset)
	}
	if style.OutlineStyle != "" {
		setString(node, protocol.OutlineStyle, style.OutlineStyle)
	}
	if style.Transform != "" {
		setString(node, protocol.Transform, style.Transform)
	}
	if style.TransformOrigin != "" {
		setString(node, protocol.TransformOrigin, style.TransformOrigin)
	}
}

func applyOutlineShorthand(node *native.Node, value string) {
	for _, token := range strings.Fields(value) {
		switch token {
		case "solid", "dashed", "dotted", "none":
			setString(node, protocol.OutlineStyle, token)
		default:
			if strings.HasSuffix(token, "px") || token == "0" {
				setLength(node, protocol.OutlineWidth, token)
				continue
			}
			if _, err := strconv.ParseFloat(token, 64); err == nil {
				setLength(node, protocol.OutlineWidth, token)
				continue
			}
			setColor(node, protocol.OutlineColor, token)
		}
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
