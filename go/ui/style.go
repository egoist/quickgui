package ui

import (
	"encoding/json"
	"fmt"
	"math"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
)

// styleData stores a composed style, including nested interaction states.
// StyleBuilder exposes fluent composition over this internal record.
type styleData struct {
	ObjectFit                string
	WordWrap                 string
	TransitionTimingFunction any
	PaddingInlineStart       any
	PaddingInlineEnd         any
	MarginInlineStart        any
	MarginInlineEnd          any
	BorderInlineStartWidth   any
	BorderInlineEndWidth     any
	AspectRatio              any
	GridColumn               any
	GridRow                  any
	BorderStartWidth         any
	BorderEndWidth           any
	Background               any
	BackgroundGradient       any
	TransitionProperty       any
	TransitionDuration       any
	TransitionEasing         any
	TransitionMaxFps         any
	ScrollToEndRevision      any
	TextDecoration           string
	Invalid                  *styleData
	Dragging                 *styleData
	DragOver                 *styleData
	FocusWithin              *styleData
	GroupActive              *styleData
	groupActiveRules         []groupHoverRule
	GridTemplateColumns      any
	GridTemplateRows         any
	GridAutoFlow             string
	GridColumnStart          any
	GridColumnEnd            any
	GridColumnSpan           any
	GridRowStart             any
	GridRowEnd               any
	GridRowSpan              any
	PaddingStart             any
	PaddingEnd               any
	MarginStart              any
	MarginEnd                any
	BorderTopWidth           any
	BorderRightWidth         any
	BorderBottomWidth        any
	BorderLeftWidth          any
	BorderTopLeftRadius      any
	BorderTopRightRadius     any
	BorderBottomLeftRadius   any
	BorderBottomRightRadius  any
	BorderStyle              string
	BoxShadow                any
	TextShadow               any
	TextDecorationLine       string
	TextDecorationColor      any
	TextDecorationStyle      string
	TextDecorationThickness  any
	WordSpacing              any
	WordBreak                string
	OverflowWrap             string
	Hyphens                  string
	TextDirection            string
	Direction                string
	BackgroundImage          string
	BackgroundSize           string
	BackgroundRepeat         string
	BackgroundPosition       string
	Filter                   any
	BackdropFilter           any
	MixBlendMode             string
	Transition               any
	ScrollSnapType           string
	ScrollSnapX              string
	ScrollSnapY              string
	ScrollSnapAlign          string
	ScrollSnapStop           string

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
	TextColor       any
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
	Outline         any
	OutlineWidth    any
	OutlineColor    any
	OutlineOffset   any
	OutlineStyle    string
	Transform       any
	TransformOrigin string
	Hover           *styleData
	Active          *styleData
	Focus           *styleData
	Disabled        *styleData
	Selected        *styleData
	GroupHover      *styleData
	groupHoverRules []groupHoverRule
}

func setLength(node *native.Node, code uint16, value any) {
	if bindStyleAccessor(node, code, value, setLength) {
		return
	}
	if value == nil {
		native.ClearProperty(node, code)
		return
	}
	if _, clear := value.(clearStyleValue); clear {
		native.ClearProperty(node, code)
		return
	}
	if unwrapped, ok := unwrapNumericPointer(value); ok {
		setLength(node, code, unwrapped)
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
	if bindStyleAccessor(node, code, value, setNumber) {
		return
	}
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
	if bindStyleAccessor(node, code, value, setColor) {
		return
	}
	if value == nil || value == "" {
		native.ClearProperty(node, code)
		return
	}
	native.SetColor(node, code, native.ParseColor(value))
}

func setFlex(node *native.Node, value any) {
	if bindStyleAccessor(node, 0, value, func(node *native.Node, _ uint16, value any) { setFlex(node, value) }) {
		return
	}
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
		native.SetNumber(node, protocol.FlexBasis, 0)
	case string:
		native.SetString(node, protocol.FlexGrow, typed)
	default:
		panic(fmt.Sprintf("QuickGUI flex %v is not a number or string", value))
	}
}

// Copy before merging so reusable themes and their nested states are never
// mutated by a node's later or conditional styles.
func mergeStateStyles(base, override *styleData) *styleData {
	merged := &styleData{}
	if base != nil {
		mergeStyle(merged, *base)
	}
	if override != nil {
		mergeStyle(merged, *override)
	}
	return merged
}

func mergeStyle(target *styleData, source styleData) {
	*target = normalizeStyleAliases(*target)
	source = normalizeStyleAliases(source)
	if source.ObjectFit != "" {
		target.ObjectFit = source.ObjectFit
	}
	if source.AspectRatio != nil {
		target.AspectRatio = source.AspectRatio
	}
	if source.GridColumn != nil {
		target.GridColumn = source.GridColumn
	}
	if source.GridRow != nil {
		target.GridRow = source.GridRow
	}
	if source.BorderStartWidth != nil {
		target.BorderStartWidth = source.BorderStartWidth
	}
	if source.BorderEndWidth != nil {
		target.BorderEndWidth = source.BorderEndWidth
	}
	if source.Background != nil {
		target.Background = source.Background
	}
	if source.BackgroundGradient != nil {
		target.BackgroundGradient = source.BackgroundGradient
	}
	if source.TransitionProperty != nil {
		target.TransitionProperty = source.TransitionProperty
	}
	if source.TransitionDuration != nil {
		target.TransitionDuration = source.TransitionDuration
	}
	if source.TransitionEasing != nil {
		target.TransitionEasing = source.TransitionEasing
	}
	if source.TransitionMaxFps != nil {
		target.TransitionMaxFps = source.TransitionMaxFps
	}
	if source.ScrollToEndRevision != nil {
		target.ScrollToEndRevision = source.ScrollToEndRevision
	}
	if source.TextDecoration != "" {
		target.TextDecoration = source.TextDecoration
	}
	if source.Invalid != nil {
		target.Invalid = mergeStateStyles(target.Invalid, source.Invalid)
	}
	if source.Dragging != nil {
		target.Dragging = mergeStateStyles(target.Dragging, source.Dragging)
	}
	if source.DragOver != nil {
		target.DragOver = mergeStateStyles(target.DragOver, source.DragOver)
	}
	if source.FocusWithin != nil {
		target.FocusWithin = mergeStateStyles(target.FocusWithin, source.FocusWithin)
	}
	if rules := groupActiveRules(source); len(rules) != 0 {
		target.groupActiveRules = append(groupActiveRules(*target), rules...)
		target.GroupActive = nil
	}
	if source.GridTemplateColumns != nil {
		target.GridTemplateColumns = source.GridTemplateColumns
	}
	if source.GridTemplateRows != nil {
		target.GridTemplateRows = source.GridTemplateRows
	}
	if source.GridAutoFlow != "" {
		target.GridAutoFlow = source.GridAutoFlow
	}
	if source.GridColumnStart != nil {
		target.GridColumnStart = source.GridColumnStart
	}
	if source.GridColumnEnd != nil {
		target.GridColumnEnd = source.GridColumnEnd
	}
	if source.GridColumnSpan != nil {
		target.GridColumnSpan = source.GridColumnSpan
	}
	if source.GridRowStart != nil {
		target.GridRowStart = source.GridRowStart
	}
	if source.GridRowEnd != nil {
		target.GridRowEnd = source.GridRowEnd
	}
	if source.GridRowSpan != nil {
		target.GridRowSpan = source.GridRowSpan
	}
	if source.PaddingStart != nil {
		target.PaddingStart = source.PaddingStart
	}
	if source.PaddingEnd != nil {
		target.PaddingEnd = source.PaddingEnd
	}
	if source.MarginStart != nil {
		target.MarginStart = source.MarginStart
	}
	if source.MarginEnd != nil {
		target.MarginEnd = source.MarginEnd
	}
	if source.BorderTopWidth != nil {
		target.BorderTopWidth = source.BorderTopWidth
	}
	if source.BorderRightWidth != nil {
		target.BorderRightWidth = source.BorderRightWidth
	}
	if source.BorderBottomWidth != nil {
		target.BorderBottomWidth = source.BorderBottomWidth
	}
	if source.BorderLeftWidth != nil {
		target.BorderLeftWidth = source.BorderLeftWidth
	}
	if source.BorderTopLeftRadius != nil {
		target.BorderTopLeftRadius = source.BorderTopLeftRadius
	}
	if source.BorderTopRightRadius != nil {
		target.BorderTopRightRadius = source.BorderTopRightRadius
	}
	if source.BorderBottomLeftRadius != nil {
		target.BorderBottomLeftRadius = source.BorderBottomLeftRadius
	}
	if source.BorderBottomRightRadius != nil {
		target.BorderBottomRightRadius = source.BorderBottomRightRadius
	}
	if source.BorderStyle != "" {
		target.BorderStyle = source.BorderStyle
	}
	if source.BoxShadow != nil {
		target.BoxShadow = source.BoxShadow
	}
	if source.TextShadow != nil {
		target.TextShadow = source.TextShadow
	}
	if source.TextDecorationLine != "" {
		target.TextDecorationLine = source.TextDecorationLine
	}
	if source.TextDecorationColor != nil {
		target.TextDecorationColor = source.TextDecorationColor
	}
	if source.TextDecorationStyle != "" {
		target.TextDecorationStyle = source.TextDecorationStyle
	}
	if source.TextDecorationThickness != nil {
		target.TextDecorationThickness = source.TextDecorationThickness
	}
	if source.WordSpacing != nil {
		target.WordSpacing = source.WordSpacing
	}
	if source.WordBreak != "" {
		target.WordBreak = source.WordBreak
	}
	if source.OverflowWrap != "" {
		target.OverflowWrap = source.OverflowWrap
	}
	if source.Hyphens != "" {
		target.Hyphens = source.Hyphens
	}
	if source.TextDirection != "" {
		target.TextDirection = source.TextDirection
	}
	if source.Direction != "" {
		target.Direction = source.Direction
	}
	if source.BackgroundImage != "" {
		target.BackgroundImage = source.BackgroundImage
	}
	if source.BackgroundSize != "" {
		target.BackgroundSize = source.BackgroundSize
	}
	if source.BackgroundRepeat != "" {
		target.BackgroundRepeat = source.BackgroundRepeat
	}
	if source.BackgroundPosition != "" {
		target.BackgroundPosition = source.BackgroundPosition
	}
	if source.Filter != nil {
		target.Filter = source.Filter
	}
	if source.BackdropFilter != nil {
		target.BackdropFilter = source.BackdropFilter
	}
	if source.MixBlendMode != "" {
		target.MixBlendMode = source.MixBlendMode
	}
	if source.Transition != nil {
		target.Transition = source.Transition
	}
	if source.ScrollSnapType != "" {
		target.ScrollSnapType = source.ScrollSnapType
	}
	if source.ScrollSnapX != "" {
		target.ScrollSnapX = source.ScrollSnapX
	}
	if source.ScrollSnapY != "" {
		target.ScrollSnapY = source.ScrollSnapY
	}
	if source.ScrollSnapAlign != "" {
		target.ScrollSnapAlign = source.ScrollSnapAlign
	}
	if source.ScrollSnapStop != "" {
		target.ScrollSnapStop = source.ScrollSnapStop
	}

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
	if source.TextColor != nil {
		target.TextColor = source.TextColor
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
		target.Hover = mergeStateStyles(target.Hover, source.Hover)
	}
	if source.Active != nil {
		target.Active = mergeStateStyles(target.Active, source.Active)
	}
	if source.Focus != nil {
		target.Focus = mergeStateStyles(target.Focus, source.Focus)
	}
	if source.Disabled != nil {
		target.Disabled = mergeStateStyles(target.Disabled, source.Disabled)
	}
	if source.Selected != nil {
		target.Selected = mergeStateStyles(target.Selected, source.Selected)
	}
	if rules := groupHoverRules(source); len(rules) != 0 {
		target.groupHoverRules = append(groupHoverRules(*target), rules...)
		target.GroupHover = nil
	}
	if source.Outline != nil {
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
	if source.Transform != nil {
		target.Transform = source.Transform
	}
	if source.TransformOrigin != "" {
		target.TransformOrigin = source.TransformOrigin
	}
}

func applyStyleList(node *native.Node, styles []styleData) styleData {
	merged := styleData{}
	for _, style := range styles {
		mergeStyle(&merged, style)
	}
	applyStyle(node, merged)
	return merged
}

func bindStyleList(node *native.Node, styles func() []styleData) {
	node.BindProperties(func() {
		applyStyleList(node, styles())
	})
}

func applyStyle(node *native.Node, style styleData) {
	style = normalizeStyleAliases(style)
	if style.ObjectFit != "" {
		setString(node, protocol.ObjectFit, style.ObjectFit)
	}
	if style.AspectRatio != nil {
		setNumber(node, protocol.AspectRatio, style.AspectRatio)
	}
	if style.BorderStartWidth != nil {
		setLength(node, protocol.BorderStartWidth, style.BorderStartWidth)
	}
	if style.BorderEndWidth != nil {
		setLength(node, protocol.BorderEndWidth, style.BorderEndWidth)
	}
	if style.ScrollToEndRevision != nil {
		setNumber(node, protocol.ScrollToEndRevision, style.ScrollToEndRevision)
	}
	if style.Invalid != nil {
		setStateStyle(node, protocol.InvalidStyle, "invalid", style.Invalid)
	}
	if style.Dragging != nil {
		setStateStyle(node, protocol.DraggingStyle, "dragging", style.Dragging)
	}
	if style.DragOver != nil {
		setStateStyle(node, protocol.DragOverStyle, "dragOver", style.DragOver)
	}
	if style.FocusWithin != nil {
		setStateStyle(node, protocol.FocusWithinStyle, "focusWithin", style.FocusWithin)
	}
	if rules := groupActiveRules(style); len(rules) != 0 {
		setGroupStyles(node, protocol.GroupActiveStyle, "groupActive", rules)
	}
	if style.Background != nil {
		bindDeclaration(node, style.Background, func(value any) { setBackground(node, value) })
	}
	if style.BackgroundGradient != nil {
		bindDeclaration(node, style.BackgroundGradient, func(value any) { setBackground(node, value) })
	}
	if style.GridColumn != nil {
		bindDeclaration(node, style.GridColumn, func(value any) { setGridPlacement(node, true, value) })
	}
	if style.GridRow != nil {
		bindDeclaration(node, style.GridRow, func(value any) { setGridPlacement(node, false, value) })
	}
	if style.TextDecoration != "" {
		applyTextDecoration(node, style.TextDecoration)
	}
	if style.GridTemplateColumns != nil {
		bindDeclaration(node, style.GridTemplateColumns, func(value any) { setGridTemplate(node, protocol.GridTemplateColumns, value) })
	}
	if style.GridTemplateRows != nil {
		bindDeclaration(node, style.GridTemplateRows, func(value any) { setGridTemplate(node, protocol.GridTemplateRows, value) })
	}
	if style.GridAutoFlow != "" {
		setString(node, protocol.GridAutoFlow, style.GridAutoFlow)
	}
	if style.GridColumnStart != nil {
		setLength(node, protocol.GridColumnStart, style.GridColumnStart)
	}
	if style.GridColumnEnd != nil {
		setLength(node, protocol.GridColumnEnd, style.GridColumnEnd)
	}
	if style.GridColumnSpan != nil {
		setLength(node, protocol.GridColumnSpan, style.GridColumnSpan)
	}
	if style.GridRowStart != nil {
		setLength(node, protocol.GridRowStart, style.GridRowStart)
	}
	if style.GridRowEnd != nil {
		setLength(node, protocol.GridRowEnd, style.GridRowEnd)
	}
	if style.GridRowSpan != nil {
		setLength(node, protocol.GridRowSpan, style.GridRowSpan)
	}
	if style.PaddingStart != nil {
		setLength(node, protocol.PaddingStart, style.PaddingStart)
	}
	if style.PaddingEnd != nil {
		setLength(node, protocol.PaddingEnd, style.PaddingEnd)
	}
	if style.MarginStart != nil {
		setLength(node, protocol.MarginStart, style.MarginStart)
	}
	if style.MarginEnd != nil {
		setLength(node, protocol.MarginEnd, style.MarginEnd)
	}
	if style.BorderTopWidth != nil {
		setLength(node, protocol.BorderTopWidth, style.BorderTopWidth)
	}
	if style.BorderRightWidth != nil {
		setLength(node, protocol.BorderRightWidth, style.BorderRightWidth)
	}
	if style.BorderBottomWidth != nil {
		setLength(node, protocol.BorderBottomWidth, style.BorderBottomWidth)
	}
	if style.BorderLeftWidth != nil {
		setLength(node, protocol.BorderLeftWidth, style.BorderLeftWidth)
	}
	if style.BorderTopLeftRadius != nil {
		setLength(node, protocol.BorderTopLeftRadius, style.BorderTopLeftRadius)
	}
	if style.BorderTopRightRadius != nil {
		setLength(node, protocol.BorderTopRightRadius, style.BorderTopRightRadius)
	}
	if style.BorderBottomLeftRadius != nil {
		setLength(node, protocol.BorderBottomLeftRadius, style.BorderBottomLeftRadius)
	}
	if style.BorderBottomRightRadius != nil {
		setLength(node, protocol.BorderBottomRightRadius, style.BorderBottomRightRadius)
	}
	if style.BorderStyle != "" {
		setString(node, protocol.BorderStyle, style.BorderStyle)
	}
	if style.BoxShadow != nil {
		bindDeclaration(node, style.BoxShadow, func(value any) { setBoxShadow(node, value) })
	}
	if style.TextShadow != nil {
		bindDeclaration(node, style.TextShadow, func(value any) { setDeclaration(node, protocol.TextShadow, value) })
	}
	if style.TextDecorationLine != "" {
		setString(node, protocol.TextDecorationLine, style.TextDecorationLine)
	}
	if style.TextDecorationColor != nil {
		setColor(node, protocol.TextDecorationColor, style.TextDecorationColor)
	}
	if style.TextDecorationStyle != "" {
		setString(node, protocol.TextDecorationStyle, style.TextDecorationStyle)
	}
	if style.TextDecorationThickness != nil {
		setLength(node, protocol.TextDecorationThickness, style.TextDecorationThickness)
	}
	if style.WordSpacing != nil {
		setLength(node, protocol.WordSpacing, style.WordSpacing)
	}
	if style.WordBreak != "" {
		setString(node, protocol.WordBreak, style.WordBreak)
	}
	if style.OverflowWrap != "" {
		setString(node, protocol.OverflowWrap, style.OverflowWrap)
	}
	if style.Hyphens != "" {
		setString(node, protocol.Hyphens, style.Hyphens)
	}
	if style.TextDirection != "" {
		setString(node, protocol.TextDirection, style.TextDirection)
	}
	if style.Direction != "" {
		setString(node, protocol.Direction, style.Direction)
	}
	if style.BackgroundImage != "" {
		setString(node, protocol.BackgroundImage, style.BackgroundImage)
	}
	if style.BackgroundSize != "" {
		setString(node, protocol.BackgroundSize, style.BackgroundSize)
	}
	if style.BackgroundRepeat != "" {
		setString(node, protocol.BackgroundRepeat, style.BackgroundRepeat)
	}
	if style.BackgroundPosition != "" {
		setString(node, protocol.BackgroundPosition, style.BackgroundPosition)
	}
	if style.Filter != nil {
		bindDeclaration(node, style.Filter, func(value any) { setDeclaration(node, protocol.Filter, value) })
	}
	if style.BackdropFilter != nil {
		bindDeclaration(node, style.BackdropFilter, func(value any) { setDeclaration(node, protocol.BackdropFilter, value) })
	}
	if style.MixBlendMode != "" {
		setString(node, protocol.MixBlendMode, style.MixBlendMode)
	}
	if style.Transition != nil {
		bindDeclaration(node, style.Transition, func(value any) { setTransition(node, value) })
	}
	if style.TransitionProperty != nil {
		bindDeclaration(node, style.TransitionProperty, func(value any) { setTransitionProperties(node, value) })
	}
	if style.TransitionDuration != nil {
		bindDeclaration(node, style.TransitionDuration, func(value any) { setMilliseconds(node, protocol.TransitionDuration, value) })
	}
	if style.TransitionEasing != nil {
		bindDeclaration(node, style.TransitionEasing, func(value any) { setTransitionEasing(node, value) })
	}
	if style.TransitionMaxFps != nil {
		setNumber(node, protocol.TransitionMaxFps, style.TransitionMaxFps)
	}

	if style.ScrollSnapType != "" {
		setString(node, protocol.ScrollSnapType, style.ScrollSnapType)
	}
	if style.ScrollSnapX != "" {
		setString(node, protocol.ScrollSnapX, style.ScrollSnapX)
	}
	if style.ScrollSnapY != "" {
		setString(node, protocol.ScrollSnapY, style.ScrollSnapY)
	}
	if style.ScrollSnapAlign != "" {
		setString(node, protocol.ScrollSnapAlign, style.ScrollSnapAlign)
	}
	if style.ScrollSnapStop != "" {
		setString(node, protocol.ScrollSnapStop, style.ScrollSnapStop)
	}

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
	if style.TextColor != nil {
		setColor(node, protocol.Color, style.TextColor)
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
	if rules := groupHoverRules(style); len(rules) != 0 {
		setGroupHoverStyles(node, rules)
	}
	if style.Outline != nil {
		bindDeclaration(node, style.Outline, func(value any) { setOutline(node, value) })
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
	if style.Transform != nil {
		bindDeclaration(node, style.Transform, func(value any) { setDeclaration(node, protocol.Transform, value) })
	}
	if style.TransformOrigin != "" {
		setString(node, protocol.TransformOrigin, style.TransformOrigin)
	}
}

type encodedStateStyle struct {
	Background      string              `json:"background,omitempty"`
	BoxShadow       *[]encodedBoxShadow `json:"boxShadow,omitempty"`
	BackgroundColor *uint32             `json:"backgroundColor,omitempty"`
	Color           *uint32             `json:"color,omitempty"`
	BorderColor     *uint32             `json:"borderColor,omitempty"`
	BorderWidth     *float64            `json:"borderWidth,omitempty"`
	BorderRadius    *float64            `json:"borderRadius,omitempty"`
	Opacity         *float64            `json:"opacity,omitempty"`
	Cursor          string              `json:"cursor,omitempty"`
	Outline         string              `json:"outline,omitempty"`
	Transform       string              `json:"transform,omitempty"`
	TransformOrigin string              `json:"transformOrigin,omitempty"`
}

func setStateStyle(node *native.Node, code uint16, state string, style *styleData) {
	node.Bind(func() {
		encoded, anyField := encodeStateStyle(state, style)
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

	})
}

func encodeStateStyle(state string, style *styleData) (encodedStateStyle, bool) {
	encoded := encodedStateStyle{}
	anyField := false
	if style.Background != nil {
		encoded.BackgroundColor, encoded.Background = encodeBackground(resolveDeclaration(style.Background))
		anyField = encoded.BackgroundColor != nil || encoded.Background != ""
	}
	if style.BackgroundGradient != nil {
		encoded.BackgroundColor, encoded.Background = encodeBackground(resolveDeclaration(style.BackgroundGradient))
		anyField = encoded.BackgroundColor != nil || encoded.Background != ""
	}
	if value := resolveDeclaration(style.BoxShadow); value != nil {
		shadows := encodeBoxShadows(value)
		encoded.BoxShadow = &shadows
		anyField = true
	}
	if value := resolveDeclaration(style.BackgroundColor); value != nil {
		color := native.ParseColor(value)
		encoded.BackgroundColor = &color
		anyField = true
	}
	if value := resolveDeclaration(style.TextColor); value != nil {
		color := native.ParseColor(value)
		encoded.Color = &color
		anyField = true
	}
	if value := resolveDeclaration(style.BorderColor); value != nil {
		color := native.ParseColor(value)
		encoded.BorderColor = &color
		anyField = true
	}
	if value := resolveDeclaration(style.BorderWidth); value != nil {
		width := stateLength(state, "borderWidth", value)
		encoded.BorderWidth = &width
		anyField = true
	}
	if value := resolveDeclaration(style.BorderRadius); value != nil {
		radius := stateLength(state, "borderRadius", value)
		encoded.BorderRadius = &radius
		anyField = true
	}
	if value := resolveDeclaration(style.Opacity); value != nil {
		opacity := toFloat(value)
		if !isFinite(opacity) {
			panic("QuickGUI state opacity must be finite")
		}
		encoded.Opacity = &opacity
		anyField = true
	}
	if style.Cursor != "" {
		if state == "groupHover" || state == "groupActive" || state == "focusWithin" {
			panic("QuickGUI " + state + " cannot declare a cursor because the pointer rests on another element")
		}
		encoded.Cursor = style.Cursor
		anyField = true
	}
	if style.Outline != nil {
		encoded.Outline = outlineText(resolveDeclaration(style.Outline))
	}
	if encoded.Outline == "" && (style.OutlineWidth != nil || style.OutlineColor != nil || style.OutlineStyle != "") {
		parts := []string{}
		if value := resolveDeclaration(style.OutlineWidth); value != nil {
			parts = append(parts, fmt.Sprintf("%gpx", stateLength(state, "outlineWidth", value)))
		}
		if style.OutlineStyle != "" {
			parts = append(parts, style.OutlineStyle)
		}
		if value := resolveDeclaration(style.OutlineColor); value != nil {
			color := native.ParseColor(value)
			parts = append(parts, fmt.Sprintf("#%02x%02x%02x%02x", byte(color), byte(color>>8), byte(color>>16), byte(color>>24)))
		}
		encoded.Outline = strings.Join(parts, " ")
	}
	if style.Transform != nil {
		encoded.Transform = declarationText(resolveDeclaration(style.Transform))
	}
	encoded.TransformOrigin = style.TransformOrigin
	anyField = anyField || encoded.Outline != "" || encoded.Transform != "" || encoded.TransformOrigin != ""
	return encoded, anyField
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

func unwrapNumericPointer(value any) (any, bool) {
	switch typed := value.(type) {
	case *int:
		if typed == nil {
			return nil, true
		}
		return *typed, true
	case *int32:
		if typed == nil {
			return nil, true
		}
		return *typed, true
	case *int64:
		if typed == nil {
			return nil, true
		}
		return *typed, true
	case *float32:
		if typed == nil {
			return nil, true
		}
		return *typed, true
	case *float64:
		if typed == nil {
			return nil, true
		}
		return *typed, true
	default:
		return nil, false
	}
}

func toFloat(value any) float64 {
	if unwrapped, ok := unwrapNumericPointer(value); ok {
		if unwrapped == nil {
			return math.NaN()
		}
		return toFloat(unwrapped)
	}
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

// Normalize aliases before merging so later records override the same property.
func normalizeStyleAliases(style styleData) styleData {
	if style.WordWrap != "" {
		if style.OverflowWrap == "" {
			style.OverflowWrap = style.WordWrap
		}
		style.WordWrap = ""
	}
	if style.TransitionTimingFunction != nil {
		if style.TransitionEasing == nil {
			style.TransitionEasing = style.TransitionTimingFunction
		}
		style.TransitionTimingFunction = nil
	}
	if style.PaddingInlineStart != nil {
		if style.PaddingStart == nil {
			style.PaddingStart = style.PaddingInlineStart
		}
		style.PaddingInlineStart = nil
	}
	if style.PaddingInlineEnd != nil {
		if style.PaddingEnd == nil {
			style.PaddingEnd = style.PaddingInlineEnd
		}
		style.PaddingInlineEnd = nil
	}
	if style.MarginInlineStart != nil {
		if style.MarginStart == nil {
			style.MarginStart = style.MarginInlineStart
		}
		style.MarginInlineStart = nil
	}
	if style.MarginInlineEnd != nil {
		if style.MarginEnd == nil {
			style.MarginEnd = style.MarginInlineEnd
		}
		style.MarginInlineEnd = nil
	}
	if style.BorderInlineStartWidth != nil {
		if style.BorderStartWidth == nil {
			style.BorderStartWidth = style.BorderInlineStartWidth
		}
		style.BorderInlineStartWidth = nil
	}
	if style.BorderInlineEndWidth != nil {
		if style.BorderEndWidth == nil {
			style.BorderEndWidth = style.BorderInlineEndWidth
		}
		style.BorderInlineEndWidth = nil
	}
	return style
}
