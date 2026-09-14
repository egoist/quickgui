package ui

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

// DrawerModality is how an open drawer interacts with the rest of the window.
type DrawerModality string

const (
	// DrawerModalityModal contains focus and projects modal semantics.
	DrawerModalityModal DrawerModality = "modal"
	// DrawerModalityNonModal leaves the page interactive while the sheet is open.
	DrawerModalityNonModal DrawerModality = "non-modal"
	// DrawerModalityTrapFocus contains focus without blocking the page.
	DrawerModalityTrapFocus DrawerModality = "trap-focus"
)

// DrawerSwipeState is the live swipe a drawer reports while a gesture is in flight.
type DrawerSwipeState struct {
	Swiping bool
	// SwipeOffset is the dismissing displacement in logical pixels. It is never negative.
	SwipeOffset float64
}

type DrawerRootProps struct {
	PartProps
	Open         func() bool
	DefaultOpen  bool
	OnOpenChange func(bool, *native.Event)
	// Modal contains focus like a dialog; DrawerModalityTrapFocus only contains focus.
	Modal DrawerModality
	// SwipeDirection is the edge a swipe dismisses toward: "up", "down" (the default),
	// "left", or "right".
	SwipeDirection string
	// SnapPoints are the allowed drawer extents. A point at or below 1 is a fraction of
	// the viewport extent; a larger one is pixels.
	SnapPoints []float64
	// SnapPoint is which declared snap point an opening drawer lands on.
	SnapPoint         *int
	OnSnapPointChange func(int, *native.Event)
	// DisablePointerDismissal refuses a backdrop or swipe dismissal, leaving Escape and
	// the close control.
	DisablePointerDismissal *bool
	// OnSwipeChange reports the live swipe the core decided, for the paint-only
	// transform the application applies.
	OnSwipeChange func(DrawerSwipeState, *native.Event)
}

type drawerState struct {
	scope string
	props DrawerRootProps
	open  *reactive.Signal[bool]
	swipe *reactive.Signal[DrawerSwipeState]
}

func (state *drawerState) isOpen() bool {
	if state.props.Open != nil {
		return state.props.Open()
	}
	return state.open.Read()
}

func (state *drawerState) change(next bool, event *native.Event) {
	if state.props.Open == nil {
		state.open.Write(next)
	}
	if state.props.OnOpenChange != nil {
		state.props.OnOpenChange(next, event)
	}
}

func (state *drawerState) liveSwipe() DrawerSwipeState {
	return state.swipe.Read()
}

func (state *drawerState) applyChange(details *ComponentChangeDetails, event *native.Event) {
	if details.Swiping != nil {
		next := DrawerSwipeState{Swiping: *details.Swiping}
		if details.SwipeOffset != nil {
			next.SwipeOffset = *details.SwipeOffset
		}
		state.swipe.Write(next)
		if state.props.OnSwipeChange != nil {
			state.props.OnSwipeChange(next, event)
		}
	}
	if details.SnapPoint != nil && state.props.OnSnapPointChange != nil {
		state.props.OnSnapPointChange(*details.SnapPoint, event)
	}
	if details.Open != nil && *details.Open != state.isOpen() {
		state.change(*details.Open, event)
	}
}

var drawerContext = createPartContext[drawerState]()

func requireDrawer(part string) *drawerState {
	return requireContext(drawerContext, part, "Drawer.Root")
}

// UseDrawerSwipe reads the live swipe inside a Drawer.Root subtree.
//
// Apply SwipeOffset as a paint-only transform on the popup; the core never animates
// the sheet.
func UseDrawerSwipe() func() DrawerSwipeState {
	return requireDrawer("useDrawerSwipe").liveSwipe
}

func createDrawerPart(name, part string, props PartProps) *native.Node {
	state := requireDrawer(name)
	node := createViewPart(props)
	setPart(node, part, state.scope, "")
	return finishPart(node, props)
}

// Drawer is a controlled sheet with snap points and core-owned swipe dismissal.
// Focus containment, Escape, and backdrop dismissal reuse the core's dialog machinery.
type drawerAPI struct{}

func (drawerAPI) Root(props DrawerRootProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	state := &drawerState{
		scope: createComponentScope("qg-drawer"),
		props: props,
		open:  reactive.NewSignal(props.DefaultOpen),
		swipe: reactive.NewSignal(DrawerSwipeState{}),
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartDrawer, state.scope, "")
	if props.Modal != "" {
		setString(node, protocol.Variant, string(props.Modal))
	}
	if props.SwipeDirection != "" {
		setString(node, protocol.SwipeDirection, props.SwipeDirection)
	}
	if props.SnapPoints != nil {
		setJson(node, protocol.Values, 65536, props.SnapPoints)
	}
	if props.SnapPoint != nil {
		setNumber(node, protocol.ItemIndex, *props.SnapPoint)
	}
	setExplicitBool(node, protocol.DisablePointerDismissal, props.DisablePointerDismissal)
	node.Bind(func() {
		setExplicitBool(node, protocol.Open, state.isOpen())
	})
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		state.applyChange(details, event)
	})
	return reactive.Provide(drawerContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (drawerAPI) Trigger(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	state := requireDrawer("Drawer.Trigger")
	node := createButtonPart(props)
	setPart(node, protocol.PartDrawerTrigger, state.scope, "")
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		state.change(true, event)
	}))
	return finishPart(node, props)
}

func (drawerAPI) Portal(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDrawerPart("Drawer.Portal", protocol.PartDrawerPortal, props)
}

func (drawerAPI) Backdrop(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDrawerPart("Drawer.Backdrop", protocol.PartDrawerBackdrop, props)
}

func (drawerAPI) Viewport(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDrawerPart("Drawer.Viewport", protocol.PartDrawerViewport, props)
}

func (drawerAPI) Popup(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDrawerPart("Drawer.Popup", protocol.PartDrawerPopup, props)
}

func (drawerAPI) Content(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDrawerPart("Drawer.Content", protocol.PartDrawerContent, props)
}

func (drawerAPI) Title(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDrawerPart("Drawer.Title", protocol.PartDrawerTitle, props)
}

func (drawerAPI) Description(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDrawerPart("Drawer.Description", protocol.PartDrawerDescription, props)
}

func (drawerAPI) Close(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	state := requireDrawer("Drawer.Close")
	node := createButtonPart(props)
	setPart(node, protocol.PartDrawerClose, state.scope, "")
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		state.change(false, event)
	}))
	return finishPart(node, props)
}

func (drawerAPI) SwipeArea(props PartProps, children ...any) *native.Node {
	props.Children = withPartChildren(props.Children, children)

	return createDrawerPart("Drawer.SwipeArea", protocol.PartDrawerSwipeArea, props)
}
