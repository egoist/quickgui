package ui

import (
	"fmt"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// PopoverOpenChangeReason is why a popover opened or closed.
type PopoverOpenChangeReason string

const (
	PopoverTriggerPress PopoverOpenChangeReason = "trigger-press"
	PopoverDismiss      PopoverOpenChangeReason = "dismiss"
	PopoverHover        PopoverOpenChangeReason = "hover"
)

type PopoverOpenChangeDetails struct {
	Reason PopoverOpenChangeReason
	Event  *native.Event
}

type PopoverSide string
type PopoverAlign string

type AnchorPositioning struct {
	Side             PopoverSide
	Align            PopoverAlign
	SideOffset       *float64
	AlignOffset      *float64
	CollisionPadding *float64
	Sticky           *bool
	Anchor           *native.Node
}

func emptyPositioning() AnchorPositioning {
	return AnchorPositioning{}
}

type PopoverRootProps struct {
	Children                func() *native.Node
	Open                    func() bool
	DefaultOpen             bool
	OnOpenChange            func(bool, PopoverOpenChangeDetails)
	DismissOnEscape         *bool
	DismissOnPointerOutside *bool
	Modal                   *bool
	OpenOnHover             *bool
	Delay                   any
	CloseDelay              any
	OnPlacementChange       func(AnchorPlacementDetails, *native.Event)
	Side                    PopoverSide
	Align                   PopoverAlign
	SideOffset              *float64
	AlignOffset             *float64
	CollisionPadding        *float64
	Sticky                  *bool
	Anchor                  *native.Node
}

type PopoverTriggerProps struct {
	PartProps
	OpenOnHover *bool
	Delay       any
	CloseDelay  any
}

type PopoverPositionerProps struct {
	PartProps
	Side             PopoverSide
	Align            PopoverAlign
	SideOffset       *float64
	AlignOffset      *float64
	CollisionPadding *float64
	Sticky           *bool
	Anchor           *native.Node
}

type PopoverPopupProps struct {
	PartProps
	OnDismiss func(*native.Event)
}

type PopoverContentProps struct {
	PartProps
	Width          float64
	Height         float64
	Placement      string
	Gap            *float64
	ViewportMargin *float64
}

type popoverSurface string

const (
	popoverSurfaceInWindow popoverSurface = "popover"
	popoverSurfaceSystem   popoverSurface = "system-popover"
)

func unresolvedPlacement() AnchorPlacementDetails {
	return AnchorPlacementDetails{Side: "bottom", Align: "start"}
}

type popoverState struct {
	surface   popoverSurface
	scope     string
	props     PopoverRootProps
	open      *reactive.Signal[bool]
	anchor    *reactive.Signal[*native.Node]
	placement *reactive.Signal[AnchorPlacementDetails]
	declared  *reactive.Signal[AnchorPositioning]
	triggers  []*native.Node
}

func newPopoverState(surface popoverSurface, props PopoverRootProps) *popoverState {
	return &popoverState{
		surface:   surface,
		scope:     createComponentScope("qg-popover"),
		props:     props,
		open:      reactive.NewSignal(props.DefaultOpen),
		anchor:    reactive.NewSignal[*native.Node](nil),
		placement: reactive.NewSignal(unresolvedPlacement()),
		declared:  reactive.NewSignal(emptyPositioning()),
	}
}

func (state *popoverState) isOpen() bool {
	if state.props.Open != nil {
		return state.props.Open()
	}
	return state.open.Read()
}

func (state *popoverState) currentAnchor() *native.Node {
	return state.anchor.Read()
}

func (state *popoverState) currentPlacement() AnchorPlacementDetails {
	return state.placement.Read()
}

func (state *popoverState) positioning() AnchorPositioning {
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
	alignOffset := override.AlignOffset
	if alignOffset == nil {
		alignOffset = props.AlignOffset
	}
	collision := override.CollisionPadding
	if collision == nil {
		collision = props.CollisionPadding
	}
	sticky := override.Sticky
	if sticky == nil {
		sticky = props.Sticky
	}
	anchor := override.Anchor
	if anchor == nil {
		anchor = props.Anchor
	}
	return AnchorPositioning{
		Side: side, Align: align, SideOffset: sideOffset, AlignOffset: alignOffset,
		CollisionPadding: collision, Sticky: sticky, Anchor: anchor,
	}
}

func (state *popoverState) declarePositioning(next AnchorPositioning) {
	state.declared.Write(next)
}

func (state *popoverState) dismissOnEscape() bool {
	if state.props.DismissOnEscape != nil {
		return *state.props.DismissOnEscape
	}
	return true
}

func (state *popoverState) dismissOnPointerOutside() bool {
	if state.props.DismissOnPointerOutside != nil {
		return *state.props.DismissOnPointerOutside
	}
	return true
}

func (state *popoverState) registerTrigger(node *native.Node) {
	state.triggers = append(state.triggers, node)
	if reactive.Untrack(func() *native.Node { return state.anchor.Read() }) == nil {
		state.anchor.Write(node)
	}
}

func (state *popoverState) unregisterTrigger(node *native.Node) {
	next := state.triggers[:0]
	for _, trigger := range state.triggers {
		if trigger != node {
			next = append(next, trigger)
		}
	}
	state.triggers = next
	if reactive.Untrack(func() *native.Node { return state.anchor.Read() }) == node {
		var replacement *native.Node
		if len(state.triggers) > 0 {
			replacement = state.triggers[0]
		}
		state.anchor.Write(replacement)
	}
}

func (state *popoverState) toggleFromTrigger(node *native.Node, event *native.Event) {
	state.anchor.Write(node)
	state.changeOpen(!state.isOpen(), PopoverTriggerPress, event)
}

func (state *popoverState) dismiss(event *native.Event) {
	state.changeOpen(false, PopoverDismiss, event)
}

func (state *popoverState) adoptOpen(next bool, event *native.Event) {
	if next == state.isOpen() {
		return
	}
	state.changeOpen(next, PopoverHover, event)
}

func (state *popoverState) reportPlacement(next AnchorPlacementDetails, event *native.Event) {
	state.placement.Write(next)
	if state.props.OnPlacementChange != nil {
		state.props.OnPlacementChange(next, event)
	}
}

func (state *popoverState) changeOpen(next bool, reason PopoverOpenChangeReason, event *native.Event) {
	if state.props.Open == nil {
		state.open.Write(next)
	}
	if state.props.OnOpenChange != nil {
		state.props.OnOpenChange(next, PopoverOpenChangeDetails{Reason: reason, Event: event})
	}
}

var popoverContext = createPartContext[popoverState]()

func requirePopover(surface popoverSurface, part string) *popoverState {
	root := "Popover.Root"
	if surface == popoverSurfaceSystem {
		root = "SystemPopover.Root"
	}
	state := requireContext(popoverContext, part, root)
	if state.surface != surface {
		panic(part + " must be used inside " + root)
	}
	return state
}

func createPopoverRoot(surface popoverSurface, props PopoverRootProps) *native.Node {
	state := newPopoverState(surface, props)
	return reactive.Provide(popoverContext, state, func() *native.Node {
		if props.Children == nil {
			return Fragment(nil)
		}
		return props.Children()
	})
}

func createPopoverTrigger(props PopoverTriggerProps) *native.Node {
	state := requireContext(popoverContext, "Popover.Trigger", "Popover.Root")
	node := createButtonPart(props.PartProps)
	if state.surface == popoverSurfaceInWindow {
		setPart(node, protocol.PartPopoverTrigger, state.scope, "")
		reactive.CreateRenderEffect(func() {
			setExplicitBool(node, protocol.Open, state.isOpen())
		})
		setExplicitBool(node, protocol.Modal, state.props.Modal)
		hover := props.OpenOnHover
		if hover == nil {
			hover = state.props.OpenOnHover
		}
		setExplicitBool(node, protocol.OpenOnHover, hover)
		delay := props.Delay
		if delay == nil {
			delay = state.props.Delay
		}
		closeDelay := props.CloseDelay
		if closeDelay == nil {
			closeDelay = state.props.CloseDelay
		}
		setMilliseconds(node, protocol.Delay, delay)
		setMilliseconds(node, protocol.CloseDelay, closeDelay)
		reactive.CreateRenderEffect(func() {
			positioning := state.positioning()
			setString(node, protocol.Side, string(positioning.Side))
			setString(node, protocol.Align, string(positioning.Align))
			setNumber(node, protocol.SideOffset, anyPtr(positioning.SideOffset))
			setNumber(node, protocol.AlignOffset, anyPtr(positioning.AlignOffset))
			setNumber(node, protocol.CollisionPadding, anyPtr(positioning.CollisionPadding))
			setExplicitBool(node, protocol.Sticky, positioning.Sticky)
			if positioning.Anchor == nil {
				setString(node, protocol.AnchorTarget, "")
			} else {
				setString(node, protocol.AnchorTarget, fmt.Sprintf("%d", positioning.Anchor.ID))
			}
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
	}
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		state.toggleFromTrigger(node, event)
	}))
	state.registerTrigger(node)
	reactive.OnCleanup(func() { state.unregisterTrigger(node) })
	return finishPart(node, props.PartProps)
}

func anyPtr[T any](value *T) any {
	if value == nil {
		return nil
	}
	return *value
}

func createPopoverPart(part, name string, props PartProps) *native.Node {
	state := requirePopover(popoverSurfaceInWindow, name)
	node := createViewPart(props)
	setPart(node, part, state.scope, "")
	return finishPart(node, props)
}

func createInWindowContent(props PopoverContentProps, state *popoverState) *native.Node {
	anchor := reactive.Untrack(func() *native.Node { return state.currentAnchor() })
	if anchor == nil {
		return Fragment(nil)
	}
	node := createViewPart(props.PartProps)
	applyStyle(node, Style{Width: props.Width, Height: props.Height})
	setString(node, protocol.AnchorTarget, fmt.Sprintf("%d", anchor.ID))
	placement := props.Placement
	if placement == "" {
		placement = "bottom-start"
	}
	setString(node, protocol.AnchorPlacement, placement)
	gap := 6.0
	if props.Gap != nil {
		gap = *props.Gap
	}
	margin := 8.0
	if props.ViewportMargin != nil {
		margin = *props.ViewportMargin
	}
	setNumber(node, protocol.AnchorGap, gap)
	setNumber(node, protocol.ViewportMargin, margin)
	setExplicitBool(node, protocol.DismissOnEscape, state.dismissOnEscape())
	setExplicitBool(node, protocol.DismissOnPointerOutside, state.dismissOnPointerOutside())
	setListener(node, protocol.EventDismiss, func(event *native.Event) {
		state.dismiss(event)
	})
	return finishPart(node, props.PartProps)
}

func createSystemContent(props PopoverContentProps, state *popoverState) *native.Node {
	anchor := reactive.Untrack(func() *native.Node { return state.currentAnchor() })
	if anchor == nil {
		return Fragment(nil)
	}
	owner := reactive.GetOwner()
	placeholder := Fragment(nil)
	var systemWindow *native.Window
	disposing := false
	native.Dispatch(func() {
		if disposing {
			return
		}
		placement := props.Placement
		if placement == "" {
			placement = "bottom-start"
		}
		gap := 6.0
		if props.Gap != nil {
			gap = *props.Gap
		}
		margin := 8.0
		if props.ViewportMargin != nil {
			margin = *props.ViewportMargin
		}
		escape := state.dismissOnEscape()
		outside := state.dismissOnPointerOutside()
		window := native.NewWindow(native.WindowOptions{
			Title:                   "QuickGUI System Popover",
			Anchor:                  anchor,
			Width:                   props.Width,
			Height:                  props.Height,
			Placement:               placement,
			Gap:                     &gap,
			ViewportMargin:          &margin,
			DismissOnEscape:         &escape,
			DismissOnPointerOutside: &outside,
			Renderer: func(target *native.Window) func() {
				return reactive.RunWithOwner(owner, func() func() {
					return CreateRenderer(func() *native.Node {
						node := createViewPart(props.PartProps)
						return finishPart(node, props.PartProps)
					})(target)
				})
			},
		})
		window.OnClose(func(*native.Window) {
			systemWindow = nil
			if disposing {
				return
			}
			native.Dispatch(func() {
				if disposing {
					return
				}
				state.dismiss(&native.Event{Type: protocol.EventDismiss, Target: placeholder})
				reactive.Flush()
			})
		})
		systemWindow = window
	})
	reactive.OnCleanup(func() {
		disposing = true
		if systemWindow != nil {
			systemWindow.Close()
		}
		systemWindow = nil
	})
	return placeholder
}

// Popover is an in-window retained popover.
var Popover = popoverAPI{}

type popoverAPI struct{}

func (popoverAPI) Root(props PopoverRootProps) *native.Node {
	return createPopoverRoot(popoverSurfaceInWindow, props)
}

func (popoverAPI) Trigger(props PopoverTriggerProps) *native.Node {
	return createPopoverTrigger(props)
}

func (popoverAPI) Content(props PopoverContentProps) *native.Node {
	state := requirePopover(popoverSurfaceInWindow, "Popover.Content")
	return Show(func() bool { return state.isOpen() && state.currentAnchor() != nil }, func() *native.Node {
		return createInWindowContent(props, state)
	})
}

func (popoverAPI) Portal(props PartProps) *native.Node {
	return createPopoverPart(protocol.PartPopoverPortal, "Popover.Portal", props)
}

func (popoverAPI) Backdrop(props PartProps) *native.Node {
	return createPopoverPart(protocol.PartPopoverBackdrop, "Popover.Backdrop", props)
}

func (popoverAPI) Positioner(props PopoverPositionerProps) *native.Node {
	state := requirePopover(popoverSurfaceInWindow, "Popover.Positioner")
	state.declarePositioning(AnchorPositioning{
		Side: props.Side, Align: props.Align, SideOffset: props.SideOffset,
		AlignOffset: props.AlignOffset, CollisionPadding: props.CollisionPadding,
		Sticky: props.Sticky, Anchor: props.Anchor,
	})
	reactive.OnCleanup(func() { state.declarePositioning(emptyPositioning()) })
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartPopoverPositioner, state.scope, "")
	return finishPart(node, props.PartProps)
}

func (popoverAPI) Popup(props PopoverPopupProps) *native.Node {
	state := requirePopover(popoverSurfaceInWindow, "Popover.Popup")
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartPopoverPopup, state.scope, "")
	setExplicitBool(node, protocol.DismissOnEscape, state.dismissOnEscape())
	setExplicitBool(node, protocol.DismissOnPointerOutside, state.dismissOnPointerOutside())
	setListener(node, protocol.EventDismiss, func(event *native.Event) {
		if props.OnDismiss != nil {
			props.OnDismiss(event)
		}
		state.dismiss(event)
	})
	return finishPart(node, props.PartProps)
}

func (popoverAPI) Arrow(props PartProps) *native.Node {
	return createPopoverPart(protocol.PartPopoverArrow, "Popover.Arrow", props)
}

func (popoverAPI) Viewport(props PartProps) *native.Node {
	return createPopoverPart(protocol.PartPopoverViewport, "Popover.Viewport", props)
}

func (popoverAPI) Title(props PartProps) *native.Node {
	return createPopoverPart(protocol.PartPopoverTitle, "Popover.Title", props)
}

func (popoverAPI) Description(props PartProps) *native.Node {
	return createPopoverPart(protocol.PartPopoverDescription, "Popover.Description", props)
}

func (popoverAPI) Close(props PartProps) *native.Node {
	state := requirePopover(popoverSurfaceInWindow, "Popover.Close")
	node := createButtonPart(props)
	setPart(node, protocol.PartPopoverClose, state.scope, "")
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		state.dismiss(event)
	}))
	return finishPart(node, props)
}

// SystemPopover opens its content in a native child window.
var SystemPopover = systemPopoverAPI{}

type systemPopoverAPI struct{}

func (systemPopoverAPI) Root(props PopoverRootProps) *native.Node {
	return createPopoverRoot(popoverSurfaceSystem, props)
}

func (systemPopoverAPI) Trigger(props PopoverTriggerProps) *native.Node {
	return createPopoverTrigger(props)
}

func (systemPopoverAPI) Content(props PopoverContentProps) *native.Node {
	state := requirePopover(popoverSurfaceSystem, "SystemPopover.Content")
	return Show(func() bool { return state.isOpen() && state.currentAnchor() != nil }, func() *native.Node {
		return createSystemContent(props, state)
	})
}

// UsePopoverPlacement reads the placement the retained tree really resolved to.
func UsePopoverPlacement() func() AnchorPlacementDetails {
	state := requirePopover(popoverSurfaceInWindow, "usePopoverPlacement")
	return state.currentPlacement
}
