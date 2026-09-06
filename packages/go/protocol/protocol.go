// Package protocol is the binary mutation protocol shared with the Rust host.
//
// Values here must stay aligned with packages/native/protocol.ts.
package protocol

const (
	Version    uint32 = 31
	RootNodeID uint32 = 0
	NoAnchor   uint32 = 0xffff_ffff
)

// Retained node kinds the host decodes.
const (
	TagView                  uint8 = 1
	TagButton                uint8 = 2
	TagText                  uint8 = 3
	TagSentinel              uint8 = 4
	TagInput                 uint8 = 5
	TagMarkdown              uint8 = 6
	TagVirtualList           uint8 = 7
	TagTerminal              uint8 = 8
	TagSvg                   uint8 = 9
	TagSwiftUIHost           uint8 = 10
	TagSwiftUIButton         uint8 = 11
	TagSwiftUIQuickGUIHost   uint8 = 12
	TagSwiftUIPopover        uint8 = 13
	TagSwiftUIPopoverTrigger uint8 = 14
	TagSwiftUIPopoverContent uint8 = 15
	TagImage                 uint8 = 16
	TagShader                uint8 = 17
	TagSwiftUISlider         uint8 = 18
	TagSwiftUIToggle         uint8 = 19
	TagSwiftUIProgressView   uint8 = 20
	TagSwiftUIStepper        uint8 = 21
	TagSwiftUITextField      uint8 = 22
	TagSwiftUIPicker         uint8 = 23
	TagSwiftUIDatePicker     uint8 = 24
	TagSwiftUIColorPicker    uint8 = 25
	TagSwiftUIGauge          uint8 = 26
)

// Property codes of the binary mutation protocol.
const (
	Display                     uint16 = 1
	FlexDirection               uint16 = 2
	FlexWrap                    uint16 = 3
	FlexGrow                    uint16 = 4
	FlexShrink                  uint16 = 5
	FlexBasis                   uint16 = 6
	AlignItems                  uint16 = 7
	AlignSelf                   uint16 = 8
	JustifyContent              uint16 = 9
	AlignContent                uint16 = 10
	Gap                         uint16 = 11
	ColumnGap                   uint16 = 12
	RowGap                      uint16 = 13
	Width                       uint16 = 14
	Height                      uint16 = 15
	MinWidth                    uint16 = 16
	MinHeight                   uint16 = 17
	MaxWidth                    uint16 = 18
	MaxHeight                   uint16 = 19
	Padding                     uint16 = 20
	PaddingTop                  uint16 = 21
	PaddingRight                uint16 = 22
	PaddingBottom               uint16 = 23
	PaddingLeft                 uint16 = 24
	Margin                      uint16 = 25
	MarginTop                   uint16 = 26
	MarginRight                 uint16 = 27
	MarginBottom                uint16 = 28
	MarginLeft                  uint16 = 29
	BackgroundColor             uint16 = 30
	Color                       uint16 = 31
	Opacity                     uint16 = 32
	BorderWidth                 uint16 = 33
	BorderColor                 uint16 = 34
	BorderRadius                uint16 = 35
	FontSize                    uint16 = 36
	FontWeight                  uint16 = 37
	LineHeight                  uint16 = 38
	TextAlign                   uint16 = 39
	WhiteSpace                  uint16 = 40
	TextOverflow                uint16 = 41
	LineClamp                   uint16 = 42
	Overflow                    uint16 = 43
	OverflowX                   uint16 = 44
	OverflowY                   uint16 = 45
	Cursor                      uint16 = 46
	AppRegion                   uint16 = 47
	Disabled                    uint16 = 48
	AccessibilityLabel          uint16 = 49
	Role                        uint16 = 50
	TabIndex                    uint16 = 51
	Position                    uint16 = 52
	Top                         uint16 = 53
	Right                       uint16 = 54
	Bottom                      uint16 = 55
	Left                        uint16 = 56
	UserSelect                  uint16 = 57
	ClickListener               uint16 = 58
	HoverListener               uint16 = 59
	Visibility                  uint16 = 60
	AspectRatio                 uint16 = 61
	Value                       uint16 = 62
	Placeholder                 uint16 = 63
	Multiline                   uint16 = 64
	InputListener               uint16 = 65
	SubmitListener              uint16 = 66
	Streaming                   uint16 = 67
	HoverBackgroundColor        uint16 = 94
	HoverColor                  uint16 = 95
	ActiveBackgroundColor       uint16 = 96
	ActiveColor                 uint16 = 97
	Transition                  uint16 = 98
	PointerListener             uint16 = 99
	FontFamily                  uint16 = 101
	Part                        uint16 = 134
	Checked                     uint16 = 135
	Scope                       uint16 = 137
	PartValue                   uint16 = 138
	Open                        uint16 = 144
	KeyDownListener             uint16 = 184
	KeyUpListener               uint16 = 185
	MouseDownListener           uint16 = 186
	MouseUpListener             uint16 = 187
	MouseMoveListener           uint16 = 188
	DoubleClickListener         uint16 = 189
	ScrollListener              uint16 = 190
	ContextMenuListener         uint16 = 191
	PinchListener               uint16 = 192
	RotationListener            uint16 = 193
	SmartMagnifyListener        uint16 = 194
	PressureListener            uint16 = 195
	FocusListener               uint16 = 196
	ActionListener              uint16 = 198
	DragListener                uint16 = 201
	DropListener                uint16 = 202
	ComponentChangeListener     uint16 = 207
	CommitListener              uint16 = 239
	BackgroundGradient          uint16 = 259
	HoverBackgroundGradient     uint16 = 278
	ActiveBackgroundGradient    uint16 = 281
	FocusBackgroundColor        uint16 = 284
	FocusColor                  uint16 = 285
	FocusBackgroundGradient     uint16 = 286
	OutlineWidth                uint16 = 265
	OutlineColor                uint16 = 266
	OutlineOffset               uint16 = 267
	OutlineStyle                uint16 = 268
	Transform                   uint16 = 275
	TransformOrigin             uint16 = 276
	HoverStyle                  uint16 = 346
	ActiveStyle                 uint16 = 347
	FocusStyle                  uint16 = 348
	DisabledStyle               uint16 = 349
	GroupHoverStyle             uint16 = 353
	SelectedStyle               uint16 = 357
	Selected                    uint16 = 358
	DismissListener             uint16 = 87
	TerminalStatusListener      uint16 = 93
	SwiftUIPresentationListener uint16 = 128
	SelectListener              uint16 = 160
	Password                    uint16 = 76
	FocusOnPointer              uint16 = 100
	Indeterminate               uint16 = 136
	ActiveValue                 uint16 = 139
	Orientation                 uint16 = 140
	KeepMounted                 uint16 = 143
	ItemIndex                   uint16 = 145
	Required                    uint16 = 147
	Invalid                     uint16 = 148
	Minimum                     uint16 = 175
	Maximum                     uint16 = 176
	Pressed                     uint16 = 181
	Values                      uint16 = 203
	Step                        uint16 = 204
	LargeStep                   uint16 = 205
	Items                       uint16 = 206
	Options                     uint16 = 208
	InputValue                  uint16 = 209
	Group                       uint16 = 235
	Delay                       uint16 = 292
	CloseDelay                  uint16 = 293
	ReadOnly                    uint16 = 296
	Side                        uint16 = 303
	Align                       uint16 = 304
	SideOffset                  uint16 = 305
	AlignOffset                 uint16 = 306
	CollisionPadding            uint16 = 307
	Sticky                      uint16 = 308
	Modal                       uint16 = 310
	OpenOnHover                 uint16 = 311
	MinStepsBetweenValues       uint16 = 317
	ThumbAlignment              uint16 = 318
	Href                        uint16 = 333
	HoverGroup                  uint16 = 354
	LetterSpacing               uint16 = 240
	TextTransform               uint16 = 242
	AnchorTarget                uint16 = 81
	AnchorPlacement             uint16 = 82
	AnchorGap                   uint16 = 83
	ViewportMargin              uint16 = 84
	DismissOnEscape             uint16 = 85
	DismissOnPointerOutside     uint16 = 86
	ActivateOnFocus             uint16 = 141
	LoopFocus                   uint16 = 142
	HeadingLevel                uint16 = 146
	ValidationMessage           uint16 = 149
	Touched                     uint16 = 150
	Dirty                       uint16 = 151
	Filled                      uint16 = 152
	Variant                     uint16 = 158
	Menu                        uint16 = 159
	Low                         uint16 = 177
	High                        uint16 = 178
	Optimum                     uint16 = 179
	ValueText                   uint16 = 180
	ObjectFit                   uint16 = 182
	FilterMode                  uint16 = 210
	Appearance                  uint16 = 211
	Columns                     uint16 = 212
	RowCount                    uint16 = 213
	SortColumn                  uint16 = 214
	SortDirection               uint16 = 215
	SelectionMode               uint16 = 216
	Selection                   uint16 = 217
	RowIndex                    uint16 = 218
	ColumnIndex                 uint16 = 219
	Nodes                       uint16 = 220
	Expanded                    uint16 = 221
	SelectedValue               uint16 = 222
	SetChildren                 uint16 = 223
	Precision                   uint16 = 224
	Toasts                      uint16 = 225
	SegmentOrder                uint16 = 226
	Segment                     uint16 = 227
	CivilValue                  uint16 = 228
	CivilMinimum                uint16 = 229
	CivilMaximum                uint16 = 230
	MenuCount                   uint16 = 231
	FirstWeekday                uint16 = 232
	RowHeight                   uint16 = 233
	HeaderHeight                uint16 = 234
	Editing                     uint16 = 236
	Disclosure                  uint16 = 237
	LoadingLabel                uint16 = 238
	SwipeDirection              uint16 = 298
	ViewportSize                uint16 = 299
	ContentSize                 uint16 = 300
	OverflowEdgeThreshold       uint16 = 301
	DisablePointerDismissal     uint16 = 302
	AnchorPoint                 uint16 = 309
	Provider                    uint16 = 312
	Timeout                     uint16 = 313
	Hoverable                   uint16 = 314
	TrackCursorAxis             uint16 = 315
	CloseOnClick                uint16 = 316
	Format                      uint16 = 319
	SmallStep                   uint16 = 320
	AllowWheelScrub             uint16 = 321
	SnapOnStep                  uint16 = 322
	Limit                       uint16 = 323
	Pitch                       uint16 = 324
	FocusableWhenDisabled       uint16 = 325
	ValidationMode              uint16 = 326
	ValidationDebounceTime      uint16 = 327
	Parent                      uint16 = 328
	EnterDuration               uint16 = 329
	ExitDuration                uint16 = 330
	StackExpanded               uint16 = 331
	CloseParentOnEsc            uint16 = 332
	Multiple                    uint16 = 334
	AlignItemWithTrigger        uint16 = 335
	AutoHighlight               uint16 = 336
	OpenOnInputClick            uint16 = 337
	HighlightItemOnHover        uint16 = 338
	Length                      uint16 = 294
	Mask                        uint16 = 295
	AutoSubmit                  uint16 = 297
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
	EventTerminal        = 7
	EventPointer         = 8
	EventPresentation    = 9
	EventMenuSelect      = 10
	EventKeyDown         = 11
	EventKeyUp           = 12
	EventMouseDown       = 13
	EventMouseUp         = 14
	EventMouseMove       = 15
	EventDoubleClick     = 16
	EventWheel           = 17
	EventContextMenu     = 18
	EventPinch           = 19
	EventRotate          = 20
	EventSmartMagnify    = 21
	EventPressure        = 22
	EventFocus           = 23
	EventBlur            = 24
	EventAction          = 25
	EventDragStart       = 26
	EventDragEnd         = 27
	EventDrop            = 28
	EventFilesDropped    = 29
	EventComponentChange = 30
	EventCommit          = 31
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
	case "terminal":
		return EventTerminal
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
	case EventTerminal:
		return TerminalStatusListener
	case EventPointer:
		return PointerListener
	case EventPresentation:
		return SwiftUIPresentationListener
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
