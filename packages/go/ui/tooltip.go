package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// TooltipCursorAxis is which cursor axis a tooltip follows.
type TooltipCursorAxis string

// TooltipSide is a tooltip's preferred side.
type TooltipSide string

// TooltipAlign is a tooltip's preferred alignment.
type TooltipAlign string

// TooltipPositioning is the placement a positioner declares.
type TooltipPositioning struct {
	Side             TooltipSide
	Align            TooltipAlign
	SideOffset       *float64
	CollisionPadding *float64
}

func emptyTooltipPositioning() TooltipPositioning {
	return TooltipPositioning{}
}

type TooltipProviderProps struct {
	PartProps
	Delay      any
	CloseDelay any
	Timeout    any
}

type TooltipRootProps struct {
	Children          func() *native.Node
	Open              func() bool
	DefaultOpen       bool
	OnOpenChange      func(bool, *native.Event)
	Disabled          bool
	Hoverable         *bool
	TrackCursorAxis   TooltipCursorAxis
	OnPlacementChange func(AnchorPlacementDetails, *native.Event)
	Side              TooltipSide
	Align             TooltipAlign
	SideOffset        *float64
	CollisionPadding  *float64
}

type TooltipTriggerProps struct {
	PartProps
	Element      string
	Delay        any
	CloseDelay   any
	CloseOnClick *bool
}

type TooltipPositionerProps struct {
	PartProps
	Side             TooltipSide
	Align            TooltipAlign
	SideOffset       *float64
	CollisionPadding *float64
}

type tooltipProviderState struct {
	scope string
}

type tooltipState struct {
	scope     string
	provider  string
	props     TooltipRootProps
	open      *reactive.Signal[bool]
	placement *reactive.Signal[AnchorPlacementDetails]
	declared  *reactive.Signal[TooltipPositioning]
}

func newTooltipState(props TooltipRootProps, provider string) *tooltipState {
	return &tooltipState{
		scope:     createComponentScope("qg-tooltip"),
		provider:  provider,
		props:     props,
		open:      reactive.NewSignal(props.DefaultOpen),
		placement: reactive.NewSignal(unresolvedPlacement()),
		declared:  reactive.NewSignal(emptyTooltipPositioning()),
	}
}

func (state *tooltipState) isOpen() bool {
	if state.props.Open != nil {
		return state.props.Open()
	}
	return state.open.Read()
}

func (state *tooltipState) currentPlacement() AnchorPlacementDetails {
	return state.placement.Read()
}

func (state *tooltipState) positioning() TooltipPositioning {
	override := state.declared.Read()
	props := state.props
	side := override.Side
	if side == "" {
		side = props.Side
	}
	align := override.Align
	if align == "" {
		align = props.Align
	}
	sideOffset := override.SideOffset
	if sideOffset == nil {
		sideOffset = props.SideOffset
	}
	collision := override.CollisionPadding
	if collision == nil {
		collision = props.CollisionPadding
	}
	return TooltipPositioning{Side: side, Align: align, SideOffset: sideOffset, CollisionPadding: collision}
}

func (state *tooltipState) reportPlacement(next AnchorPlacementDetails, event *native.Event) {
	state.placement.Write(next)
	if state.props.OnPlacementChange != nil {
		state.props.OnPlacementChange(next, event)
	}
}

func (state *tooltipState) adoptOpen(next bool, event *native.Event) {
	if next == state.isOpen() {
		return
	}
	if state.props.Open == nil {
		state.open.Write(next)
	}
	if state.props.OnOpenChange != nil {
		state.props.OnOpenChange(next, event)
	}
}

var tooltipProviderContext = createPartContext[tooltipProviderState]()
var tooltipContext = createPartContext[tooltipState]()

func requireTooltip(part string) *tooltipState {
	return requireContext(tooltipContext, part, "Tooltip.Root")
}

// UseTooltipPlacement reads the placement the core really resolved to inside a Tooltip.Root subtree.
func UseTooltipPlacement() func() AnchorPlacementDetails {
	state := requireTooltip("useTooltipPlacement")
	return state.currentPlacement
}

func triggerTag(element string) uint8 {
	switch element {
	case "view":
		return protocol.TagView
	case "text":
		return protocol.TagText
	case "input":
		return protocol.TagInput
	default:
		return protocol.TagButton
	}
}

func createTooltipPart(name, part string, props PartProps) *native.Node {
	state := requireTooltip(name)
	node := createViewPart(props)
	setPart(node, part, state.scope, "")
	return finishPart(node, props)
}

// Tooltip is a hover-opened surface whose deadlines belong to the core.
var Tooltip = tooltipAPI{}

type tooltipAPI struct{}

func (tooltipAPI) Provider(props TooltipProviderProps) *native.Node {
	state := &tooltipProviderState{scope: createComponentScope("qg-tooltip-provider")}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartTooltipProvider, state.scope, "")
	setMilliseconds(node, protocol.Delay, props.Delay)
	setMilliseconds(node, protocol.CloseDelay, props.CloseDelay)
	setMilliseconds(node, protocol.Timeout, props.Timeout)
	return reactive.Provide(tooltipProviderContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (tooltipAPI) Root(props TooltipRootProps) *native.Node {
	provider := ""
	if current := tooltipProviderContext.Use(); current != nil {
		provider = current.scope
	}
	state := newTooltipState(props, provider)
	return reactive.Provide(tooltipContext, state, func() *native.Node {
		if props.Children == nil {
			return Fragment(nil)
		}
		return props.Children()
	})
}

func (tooltipAPI) Trigger(props TooltipTriggerProps) *native.Node {
	state := requireTooltip("Tooltip.Trigger")
	root := state.props
	node := createPart(triggerTag(props.Element), props.PartProps)
	setPart(node, protocol.PartTooltipTrigger, state.scope, "")
	if state.provider != "" {
		setComponentValue(node, protocol.Provider, state.provider)
	}
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Open, state.isOpen())
	})
	if props.Disabled == nil && root.Disabled {
		setExplicitBool(node, protocol.Disabled, true)
	}
	setExplicitBool(node, protocol.Hoverable, root.Hoverable)
	setString(node, protocol.TrackCursorAxis, string(root.TrackCursorAxis))
	setMilliseconds(node, protocol.Delay, props.Delay)
	setMilliseconds(node, protocol.CloseDelay, props.CloseDelay)
	setExplicitBool(node, protocol.CloseOnClick, props.CloseOnClick)
	reactive.CreateRenderEffect(func() {
		positioning := state.positioning()
		setString(node, protocol.Side, string(positioning.Side))
		setString(node, protocol.Align, string(positioning.Align))
		setNumber(node, protocol.SideOffset, anyPtr(positioning.SideOffset))
		setNumber(node, protocol.CollisionPadding, anyPtr(positioning.CollisionPadding))
	})
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.Placement != nil {
			state.reportPlacement(*details.Placement, event)
		}
		if details.Open != nil {
			state.adoptOpen(*details.Open, event)
		}
	})
	return finishPart(node, props.PartProps)
}

func (tooltipAPI) Positioner(props TooltipPositionerProps) *native.Node {
	state := requireTooltip("Tooltip.Positioner")
	state.declared.Write(TooltipPositioning{
		Side: props.Side, Align: props.Align, SideOffset: props.SideOffset, CollisionPadding: props.CollisionPadding,
	})
	reactive.OnCleanup(func() { state.declared.Write(emptyTooltipPositioning()) })
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartTooltipPositioner, state.scope, "")
	return finishPart(node, props.PartProps)
}

func (tooltipAPI) Portal(props PartProps) *native.Node {
	return createTooltipPart("Tooltip.Portal", protocol.PartTooltipPortal, props)
}

func (tooltipAPI) Popup(props PartProps) *native.Node {
	return createTooltipPart("Tooltip.Popup", protocol.PartTooltipPopup, props)
}

func (tooltipAPI) Arrow(props PartProps) *native.Node {
	return createTooltipPart("Tooltip.Arrow", protocol.PartTooltipArrow, props)
}

type PreviewCardRootProps struct {
	PartProps
	Open           func() bool
	DefaultOpen    bool
	OnOpenChange   func(bool, *native.Event)
	Placement      string
	Gap            any
	ViewportMargin any
}

type PreviewCardTriggerProps struct {
	PartProps
	Delay      any
	CloseDelay any
}

type previewCardState struct {
	scope string
	props PreviewCardRootProps
	open  *reactive.Signal[bool]
}

func newPreviewCardState(props PreviewCardRootProps) *previewCardState {
	return &previewCardState{
		scope: createComponentScope("qg-preview-card"),
		props: props,
		open:  reactive.NewSignal(props.DefaultOpen),
	}
}

func (state *previewCardState) isOpen() bool {
	if state.props.Open != nil {
		return state.props.Open()
	}
	return state.open.Read()
}

func (state *previewCardState) setOpen(next bool, event *native.Event) {
	if state.props.Open == nil {
		state.open.Write(next)
	}
	if state.props.OnOpenChange != nil {
		state.props.OnOpenChange(next, event)
	}
}

var previewCardContext = createPartContext[previewCardState]()

func createPreviewCardPart(name, part string, props PartProps) *native.Node {
	state := requireContext(previewCardContext, name, "PreviewCard.Root")
	node := createViewPart(props)
	setPart(node, part, state.scope, "")
	return finishPart(node, props)
}

// PreviewCard is a hover card whose two deadlines belong to the core.
var PreviewCard = previewCardAPI{}

type previewCardAPI struct{}

func (previewCardAPI) Root(props PreviewCardRootProps) *native.Node {
	state := newPreviewCardState(props)
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartPreviewCard, state.scope, "")
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Open, state.isOpen())
	})
	setString(node, protocol.AnchorPlacement, props.Placement)
	setNumber(node, protocol.AnchorGap, props.Gap)
	setNumber(node, protocol.ViewportMargin, props.ViewportMargin)
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || details.Open == nil {
			return
		}
		state.setOpen(*details.Open, event)
	})
	return reactive.Provide(previewCardContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (previewCardAPI) Trigger(props PreviewCardTriggerProps) *native.Node {
	state := requireContext(previewCardContext, "PreviewCard.Trigger", "PreviewCard.Root")
	node := createButtonPart(props.PartProps)
	setPart(node, protocol.PartPreviewCardTrigger, state.scope, "")
	setMilliseconds(node, protocol.Delay, props.Delay)
	setMilliseconds(node, protocol.CloseDelay, props.CloseDelay)
	return finishPart(node, props.PartProps)
}

func (previewCardAPI) Portal(props PartProps) *native.Node {
	return createPreviewCardPart("PreviewCard.Portal", protocol.PartPreviewCardPortal, props)
}

func (previewCardAPI) Positioner(props PartProps) *native.Node {
	return createPreviewCardPart("PreviewCard.Positioner", protocol.PartPreviewCardPositioner, props)
}

func (previewCardAPI) Popup(props PartProps) *native.Node {
	return createPreviewCardPart("PreviewCard.Popup", protocol.PartPreviewCardPopup, props)
}

func (previewCardAPI) Arrow(props PartProps) *native.Node {
	return createPreviewCardPart("PreviewCard.Arrow", protocol.PartPreviewCardArrow, props)
}

func (previewCardAPI) Backdrop(props PartProps) *native.Node {
	return createPreviewCardPart("PreviewCard.Backdrop", protocol.PartPreviewCardBackdrop, props)
}
