// Package protocol is the binary mutation protocol shared with the Rust host.
//
// Wire constants are generated from the Rust decoder.
//
//go:generate go run ../internal/cmd/protocolgen
package protocol

const (
	RootNodeID uint32 = 0
	NoAnchor   uint32 = 0xffff_ffff
)

const (
	MaxStyleDeclarationBytes = 4096
	MaxStateStyleJSONBytes   = 16 * 1024
	MaxComponentValueBytes   = 256
	MaxComponentJSONBytes    = 64 * 1024
	MaxHoverGroupNameBytes   = 256
	MaxMenuLinkBytes         = 8 * 1024
)

// Event types, numbered so listeners live in plain slices.
const (
	EventClick           = 1
	EventMouseEnter      = 2
	EventMouseLeave      = 3
	EventInput           = 4
	EventSubmit          = 5
	EventDismiss         = 6
	EventPointer         = 7
	EventPresentation    = 8
	EventMenuSelect      = 9
	EventKeyDown         = 10
	EventKeyUp           = 11
	EventMouseDown       = 12
	EventMouseUp         = 13
	EventMouseMove       = 14
	EventDoubleClick     = 15
	EventWheel           = 16
	EventContextMenu     = 17
	EventPinch           = 18
	EventRotate          = 19
	EventSmartMagnify    = 20
	EventPressure        = 21
	EventFocus           = 22
	EventBlur            = 23
	EventAction          = 24
	EventDragStart       = 25
	EventDragEnd         = 26
	EventDrop            = 27
	EventFilesDropped    = 28
	EventComponentChange = 29
	EventCommit          = 30
)

// EventTypeFromKind maps one native event kind onto its numeric type, or 0.
func EventTypeFromKind(kind string) int {
	switch kind {
	case "click":
		return EventClick
	case "mouseenter":
		return EventMouseEnter
	case "mouseleave":
		return EventMouseLeave
	case "input":
		return EventInput
	case "submit":
		return EventSubmit
	case "dismiss":
		return EventDismiss
	case "pointer":
		return EventPointer
	case "presentationchange":
		return EventPresentation
	case "menuselect":
		return EventMenuSelect
	case "keydown":
		return EventKeyDown
	case "keyup":
		return EventKeyUp
	case "mousedown":
		return EventMouseDown
	case "mouseup":
		return EventMouseUp
	case "mousemove":
		return EventMouseMove
	case "dblclick":
		return EventDoubleClick
	case "wheel":
		return EventWheel
	case "contextmenu":
		return EventContextMenu
	case "pinch":
		return EventPinch
	case "rotate":
		return EventRotate
	case "smartmagnify":
		return EventSmartMagnify
	case "pressure":
		return EventPressure
	case "focus":
		return EventFocus
	case "blur":
		return EventBlur
	case "action":
		return EventAction
	case "dragstart":
		return EventDragStart
	case "dragend":
		return EventDragEnd
	case "drop":
		return EventDrop
	case "filesdropped":
		return EventFilesDropped
	case "componentchange":
		return EventComponentChange
	case "commit":
		return EventCommit
	default:
		return 0
	}
}

// ListenerPropertyFor is the declared-listener property the host reads, or 0 when implicit.
func ListenerPropertyFor(eventType int) uint16 {
	switch eventType {
	case EventClick:
		return ClickListener
	case EventMouseEnter, EventMouseLeave:
		return HoverListener
	case EventInput:
		return InputListener
	case EventSubmit:
		return SubmitListener
	case EventDismiss:
		return DismissListener
	case EventPointer:
		return PointerListener
	case EventPresentation:
		return 0 // Presentation lifecycle events are emitted without a listener property.
	case EventMenuSelect:
		return SelectListener
	case EventKeyDown:
		return KeyDownListener
	case EventKeyUp:
		return KeyUpListener
	case EventMouseDown:
		return MouseDownListener
	case EventMouseUp:
		return MouseUpListener
	case EventMouseMove:
		return MouseMoveListener
	case EventDoubleClick:
		return DoubleClickListener
	case EventWheel:
		return ScrollListener
	case EventContextMenu:
		return ContextMenuListener
	case EventPinch:
		return PinchListener
	case EventRotate:
		return RotationListener
	case EventSmartMagnify:
		return SmartMagnifyListener
	case EventPressure:
		return PressureListener
	case EventFocus, EventBlur:
		return FocusListener
	case EventAction:
		return ActionListener
	case EventDragStart, EventDragEnd:
		return DragListener
	case EventDrop, EventFilesDropped:
		return DropListener
	case EventComponentChange:
		return ComponentChangeListener
	case EventCommit:
		return CommitListener
	default:
		return 0
	}
}

// SharesListenerProperty reports whether two event types share one host listener flag.
func SharesListenerProperty(a, b int) bool {
	return ListenerPropertyFor(a) == ListenerPropertyFor(b)
}
