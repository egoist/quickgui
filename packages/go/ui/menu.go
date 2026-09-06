package ui

import (
	"encoding/json"
	"fmt"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// MenuItemKind is one entry kind of a declared menu model.
type MenuItemKind string

const (
	MenuItemAction    MenuItemKind = "action"
	MenuItemCheckbox  MenuItemKind = "checkbox"
	MenuItemRadio     MenuItemKind = "radio"
	MenuItemSubmenu   MenuItemKind = "submenu"
	MenuItemSeparator MenuItemKind = "separator"
	MenuItemGroup     MenuItemKind = "group"
)

// MenuItemDeclaration is one entry of the bounded JSON model a PopoverMenu or ContextMenu declares.
type MenuItemDeclaration struct {
	Type           MenuItemKind          `json:"type,omitempty"`
	ID             string                `json:"id,omitempty"`
	Label          string                `json:"label,omitempty"`
	Shortcut       string                `json:"shortcut,omitempty"`
	Group          string                `json:"group,omitempty"`
	Checked        *bool                 `json:"checked,omitempty"`
	Disabled       bool                  `json:"disabled,omitempty"`
	CloseOnSelect  *bool                 `json:"closeOnSelect,omitempty"`
	TypeaheadLabel string                `json:"typeaheadLabel,omitempty"`
	Items          []MenuItemDeclaration `json:"items,omitempty"`
}

// MenuAppearance is structural geometry and appearance for a declared menu surface.
type MenuAppearance struct {
	Width               *float64
	ItemHeight          *float64
	SeparatorHeight     *float64
	GroupLabelHeight    *float64
	VerticalPadding     *float64
	FontSize            *float64
	Radius              *float64
	Padding             *float64
	Background          any
	Color               any
	HighlightBackground any
	HighlightColor      any
	MutedColor          any
	Loop                *bool
}

type encodedMenu struct {
	Items               []MenuItemDeclaration `json:"items"`
	Width               *float64              `json:"width,omitempty"`
	ItemHeight          *float64              `json:"itemHeight,omitempty"`
	SeparatorHeight     *float64              `json:"separatorHeight,omitempty"`
	GroupLabelHeight    *float64              `json:"groupLabelHeight,omitempty"`
	VerticalPadding     *float64              `json:"verticalPadding,omitempty"`
	FontSize            *float64              `json:"fontSize,omitempty"`
	Radius              *float64              `json:"radius,omitempty"`
	Padding             *float64              `json:"padding,omitempty"`
	Background          *uint32               `json:"background,omitempty"`
	Color               *uint32               `json:"color,omitempty"`
	HighlightBackground *uint32               `json:"highlightBackground,omitempty"`
	HighlightColor      *uint32               `json:"highlightColor,omitempty"`
	MutedColor          *uint32               `json:"mutedColor,omitempty"`
	LoopFocus           *bool                 `json:"loopFocus,omitempty"`
}

// MaxMenuJSONBytes is the byte bound a declared menu may not exceed.
const MaxMenuJSONBytes = 256 * 1024

// EncodeMenu encodes one bounded menu declaration for the Rust binding.
func EncodeMenu(items []MenuItemDeclaration, appearance *MenuAppearance) string {
	if items == nil {
		items = []MenuItemDeclaration{}
	}
	declaration := encodedMenu{Items: items}
	if appearance != nil {
		declaration.Width = appearance.Width
		declaration.ItemHeight = appearance.ItemHeight
		declaration.SeparatorHeight = appearance.SeparatorHeight
		declaration.GroupLabelHeight = appearance.GroupLabelHeight
		declaration.VerticalPadding = appearance.VerticalPadding
		declaration.FontSize = appearance.FontSize
		declaration.Radius = appearance.Radius
		declaration.Padding = appearance.Padding
		declaration.Background = packedColor(appearance.Background)
		declaration.Color = packedColor(appearance.Color)
		declaration.HighlightBackground = packedColor(appearance.HighlightBackground)
		declaration.HighlightColor = packedColor(appearance.HighlightColor)
		declaration.MutedColor = packedColor(appearance.MutedColor)
		declaration.LoopFocus = appearance.Loop
	}
	payload, err := json.Marshal(declaration)
	if err != nil {
		panic(err)
	}
	if len(payload) > MaxMenuJSONBytes {
		panic(fmt.Sprintf("QuickGUI menu declarations are bounded to %d bytes", MaxMenuJSONBytes))
	}
	return string(payload)
}

type PopoverMenuOpenChangeDetails struct {
	Reason PopoverOpenChangeReason
	Event  *native.Event
}

type PopoverMenuRootProps struct {
	Children                func() *native.Node
	Items                   func() []MenuItemDeclaration
	Appearance              *MenuAppearance
	Open                    func() bool
	DefaultOpen             bool
	OnOpenChange            func(bool, PopoverMenuOpenChangeDetails)
	OnSelect                func(MenuSelectDetails, *native.Event)
	Placement               string
	Gap                     *float64
	ViewportMargin          *float64
	DismissOnEscape         *bool
	DismissOnPointerOutside *bool
}

type PopoverMenuPopupProps struct {
	PartProps
	Width     *float64
	OnSelect  func(*native.Event)
	OnDismiss func(*native.Event)
}

type popoverMenuState struct {
	props   PopoverMenuRootProps
	open    *reactive.Signal[bool]
	trigger *reactive.Signal[*native.Node]
}

func newPopoverMenuState(props PopoverMenuRootProps) *popoverMenuState {
	return &popoverMenuState{
		props:   props,
		open:    reactive.NewSignal(props.DefaultOpen),
		trigger: reactive.NewSignal[*native.Node](nil),
	}
}

func (state *popoverMenuState) isOpen() bool {
	if state.props.Open != nil {
		return state.props.Open()
	}
	return state.open.Read()
}

func (state *popoverMenuState) items() []MenuItemDeclaration {
	if state.props.Items == nil {
		return nil
	}
	return state.props.Items()
}

func (state *popoverMenuState) menu() string {
	return EncodeMenu(state.items(), state.props.Appearance)
}

func (state *popoverMenuState) setOpen(next bool, reason PopoverOpenChangeReason, event *native.Event) {
	if state.props.Open == nil {
		state.open.Write(next)
	}
	if state.props.OnOpenChange != nil {
		state.props.OnOpenChange(next, PopoverMenuOpenChangeDetails{Reason: reason, Event: event})
	}
}

func (state *popoverMenuState) selectItem(details MenuSelectDetails, event *native.Event) {
	if state.props.OnSelect != nil {
		state.props.OnSelect(details, event)
	}
	if details.Submenu {
		return
	}
	state.setOpen(false, PopoverDismiss, event)
}

var popoverMenuContext = createPartContext[popoverMenuState]()

// PopoverMenu is a declared popover menu whose rows the core paints itself.
var PopoverMenu = popoverMenuAPI{}

type popoverMenuAPI struct{}

func (popoverMenuAPI) Root(props PopoverMenuRootProps) *native.Node {
	state := newPopoverMenuState(props)
	return reactive.Provide(popoverMenuContext, state, func() *native.Node {
		if props.Children == nil {
			return Fragment(nil)
		}
		return props.Children()
	})
}

func (popoverMenuAPI) Trigger(props PartProps) *native.Node {
	state := requireContext(popoverMenuContext, "PopoverMenu.Trigger", "PopoverMenu.Root")
	node := createButtonPart(props)
	setPart(node, protocol.PartPopoverMenuTrigger, "", "")
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Open, state.isOpen())
	})
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(event *native.Event) {
		state.trigger.Write(node)
		state.setOpen(!state.isOpen(), PopoverTriggerPress, event)
	}))
	if reactive.Untrack(func() *native.Node { return state.trigger.Read() }) == nil {
		state.trigger.Write(node)
	}
	return finishPart(node, props)
}

func (popoverMenuAPI) Popup(props PopoverMenuPopupProps) *native.Node {
	state := requireContext(popoverMenuContext, "PopoverMenu.Popup", "PopoverMenu.Root")
	return Show(func() bool { return state.isOpen() && state.trigger.Read() != nil }, func() *native.Node {
		return createPopoverMenuPopup(props, state)
	})
}

func createPopoverMenuPopup(props PopoverMenuPopupProps, state *popoverMenuState) *native.Node {
	anchor := reactive.Untrack(func() *native.Node { return state.trigger.Read() })
	if anchor == nil {
		return Fragment(nil)
	}
	rootProps := state.props
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartPopoverMenuPopup, "", "")
	setString(node, protocol.AnchorTarget, fmt.Sprintf("%d", anchor.ID))
	reactive.CreateRenderEffect(func() {
		setString(node, protocol.Menu, state.menu())
	})
	width := any(224)
	if props.Width != nil {
		width = *props.Width
	} else if rootProps.Appearance != nil && rootProps.Appearance.Width != nil {
		width = *rootProps.Appearance.Width
	}
	applyStyle(node, Style{Width: width})
	placement := rootProps.Placement
	if placement == "" {
		placement = "bottom-start"
	}
	setString(node, protocol.AnchorPlacement, placement)
	gap := 4.0
	if rootProps.Gap != nil {
		gap = *rootProps.Gap
	}
	margin := 8.0
	if rootProps.ViewportMargin != nil {
		margin = *rootProps.ViewportMargin
	}
	setNumber(node, protocol.AnchorGap, gap)
	setNumber(node, protocol.ViewportMargin, margin)
	escape := true
	if rootProps.DismissOnEscape != nil {
		escape = *rootProps.DismissOnEscape
	}
	outside := true
	if rootProps.DismissOnPointerOutside != nil {
		outside = *rootProps.DismissOnPointerOutside
	}
	setExplicitBool(node, protocol.DismissOnEscape, escape)
	setExplicitBool(node, protocol.DismissOnPointerOutside, outside)
	setListener(node, protocol.EventMenuSelect, func(event *native.Event) {
		if props.OnSelect != nil {
			props.OnSelect(event)
		}
		if details := MenuSelectionFromEvent(event); details != nil {
			state.selectItem(*details, event)
		}
	})
	setListener(node, protocol.EventDismiss, func(event *native.Event) {
		if props.OnDismiss != nil {
			props.OnDismiss(event)
		}
		if !event.DefaultPrevented {
			state.setOpen(false, PopoverDismiss, event)
		}
	})
	return finishPart(node, props.PartProps)
}

type ContextMenuRootProps struct {
	Children   func() *native.Node
	Items      func() []MenuItemDeclaration
	Appearance *MenuAppearance
	OnSelect   func(MenuSelectDetails, *native.Event)
	Loop       *bool
}

type ContextMenuTriggerProps struct {
	PartProps
	OnSelect func(*native.Event)
}

type contextMenuState struct {
	props ContextMenuRootProps
}

func (state *contextMenuState) menu() string {
	var items []MenuItemDeclaration
	if state.props.Items != nil {
		items = state.props.Items()
	}
	return EncodeMenu(items, state.props.Appearance)
}

var contextMenuContext = createPartContext[contextMenuState]()

// ContextMenu is a declared cursor-point context menu.
var ContextMenu = contextMenuAPI{}

type contextMenuAPI struct{}

func (contextMenuAPI) Root(props ContextMenuRootProps) *native.Node {
	state := &contextMenuState{props: props}
	compound := newMenuRootState(false, MenuRootProps{})
	if props.Loop != nil {
		compound.props.LoopFocus = props.Loop
	}
	return reactive.Provide(contextMenuContext, state, func() *native.Node {
		return reactive.Provide(menuContext, compound, func() *native.Node {
			if props.Children == nil {
				return Fragment(nil)
			}
			return props.Children()
		})
	})
}

func (contextMenuAPI) Trigger(props ContextMenuTriggerProps) *native.Node {
	state := requireContext(contextMenuContext, "ContextMenu.Trigger", "ContextMenu.Root")
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartContextMenuTrigger, "", "")
	reactive.CreateRenderEffect(func() {
		setString(node, protocol.Menu, state.menu())
	})
	setListener(node, protocol.EventMenuSelect, func(event *native.Event) {
		if props.OnSelect != nil {
			props.OnSelect(event)
		}
		details := MenuSelectionFromEvent(event)
		if details == nil {
			return
		}
		if state.props.OnSelect != nil {
			state.props.OnSelect(*details, event)
		}
	})
	return finishPart(node, props.PartProps)
}

// MenuSurfaceState is everything the core decided about one menu surface.
type MenuSurfaceState struct {
	Open         bool
	Side         string
	Align        string
	AnchorHidden bool
}

func settledMenu() MenuSurfaceState {
	return MenuSurfaceState{Side: "bottom", Align: "start"}
}

// MenuItemState is everything the core decided about one menu row.
type MenuItemState struct {
	Highlighted bool
	Disabled    bool
	Checked     *bool
	Open        bool
}

func settledMenuItem() MenuItemState {
	return MenuItemState{}
}

type MenuSide string
type MenuAlign string

type MenuPositioning struct {
	Side             MenuSide
	Align            MenuAlign
	SideOffset       *float64
	AlignOffset      *float64
	CollisionPadding *float64
	Sticky           *bool
}

func emptyMenuPositioning() MenuPositioning {
	return MenuPositioning{}
}

type MenuRootProps struct {
	Children         func() *native.Node
	Open             func() bool
	DefaultOpen      bool
	OnOpenChange     func(bool, *native.Event)
	Modal            *bool
	Orientation      string
	LoopFocus        *bool
	CloseParentOnEsc *bool
	Disabled         bool
	OpenOnHover      *bool
	Delay            any
	CloseDelay       any
	Side             MenuSide
	Align            MenuAlign
	SideOffset       *float64
	AlignOffset      *float64
	CollisionPadding *float64
	Sticky           *bool
}

type MenuTriggerProps struct {
	PartProps
	OpenOnHover *bool
	Delay       any
	CloseDelay  any
}

type MenuPositionerProps struct {
	PartProps
	Side             MenuSide
	Align            MenuAlign
	SideOffset       *float64
	AlignOffset      *float64
	CollisionPadding *float64
	Sticky           *bool
}

type MenuItemProps struct {
	PartProps
	Value        string
	Label        string
	CloseOnClick *bool
}

type MenuLinkItemProps struct {
	MenuItemProps
	Href       string
	OnNavigate func(string, *native.Event)
}

type MenuCheckboxItemProps struct {
	MenuItemProps
	Checked         func() bool
	OnCheckedChange func(bool, *native.Event)
}

type MenuRadioItemProps struct {
	MenuItemProps
	Checked func() bool
}

type MenuRadioGroupProps struct {
	PartProps
	Name          string
	Value         func() *string
	DefaultValue  string
	OnValueChange func(string, *native.Event)
}

type MenuGroupLabelProps struct {
	PartProps
	Value string
	Label string
}

type MenuSubmenuTriggerProps struct {
	MenuTriggerProps
	Value        string
	Label        string
	CloseOnClick *bool
}

type menuRootState struct {
	submenu  bool
	scope    string
	props    MenuRootProps
	open     *reactive.Signal[bool]
	state    *reactive.Signal[MenuSurfaceState]
	declared *reactive.Signal[MenuPositioning]
}

func newMenuRootState(submenu bool, props MenuRootProps) *menuRootState {
	return &menuRootState{
		submenu:  submenu,
		scope:    createComponentScope("qg-menu"),
		props:    props,
		open:     reactive.NewSignal(props.DefaultOpen),
		state:    reactive.NewSignal(settledMenu()),
		declared: reactive.NewSignal(emptyMenuPositioning()),
	}
}

func (state *menuRootState) isOpen() bool {
	if state.props.Open != nil {
		return state.props.Open()
	}
	return state.open.Read()
}

func (state *menuRootState) live() MenuSurfaceState {
	return state.state.Read()
}

func (state *menuRootState) setOpen(next bool, event *native.Event) {
	if state.props.Open == nil {
		state.open.Write(next)
	}
	if state.props.OnOpenChange != nil {
		state.props.OnOpenChange(next, event)
	}
}

func (state *menuRootState) positioning() MenuPositioning {
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
	return MenuPositioning{
		Side: side, Align: align, SideOffset: sideOffset, AlignOffset: alignOffset,
		CollisionPadding: collision, Sticky: sticky,
	}
}

func (state *menuRootState) reportSurface(details *ComponentChangeDetails) {
	current := reactive.Untrack(func() MenuSurfaceState { return state.state.Read() })
	side := details.Side
	if side == "" {
		side = current.Side
	}
	align := details.Align
	if align == "" {
		align = current.Align
	}
	state.state.Write(MenuSurfaceState{
		Open: details.Open != nil && *details.Open, Side: side, Align: align,
		AnchorHidden: details.AnchorHidden != nil && *details.AnchorHidden,
	})
}

type menuItemStateHolder struct {
	state *reactive.Signal[MenuItemState]
}

func newMenuItemStateHolder() *menuItemStateHolder {
	return &menuItemStateHolder{state: reactive.NewSignal(settledMenuItem())}
}

func (holder *menuItemStateHolder) live() MenuItemState {
	return holder.state.Read()
}

func (holder *menuItemStateHolder) adopt(details *ComponentChangeDetails) {
	holder.state.Write(MenuItemState{
		Highlighted: details.Highlighted != nil && *details.Highlighted,
		Disabled:    details.Disabled != nil && *details.Disabled,
		Checked:     details.Checked.ptr(),
		Open:        details.Open != nil && *details.Open,
	})
}

type menuRadioGroupState struct {
	props MenuRadioGroupProps
	value *reactive.Signal[*string]
}

func newMenuRadioGroupState(props MenuRadioGroupProps) *menuRadioGroupState {
	var initial *string
	if props.DefaultValue != "" {
		value := props.DefaultValue
		initial = &value
	}
	return &menuRadioGroupState{props: props, value: reactive.NewSignal(initial)}
}

func (state *menuRadioGroupState) current() *string {
	if state.props.Value != nil {
		return state.props.Value()
	}
	return state.value.Read()
}

func (state *menuRadioGroupState) setValue(next string, event *native.Event) {
	if state.props.Value == nil {
		value := next
		state.value.Write(&value)
	}
	if state.props.OnValueChange != nil {
		state.props.OnValueChange(next, event)
	}
}

var menuContext = createPartContext[menuRootState]()
var menuItemContext = createPartContext[menuItemStateHolder]()
var menuRadioGroupContext = createPartContext[menuRadioGroupState]()

func requireMenu(part string) *menuRootState {
	return requireContext(menuContext, part, "Menu.Root")
}

// UseMenuState reads what the core decided about the enclosing menu surface.
func UseMenuState() func() MenuSurfaceState {
	context := menuContext.Use()
	if context == nil {
		return settledMenu
	}
	return context.live
}

// UseMenuItemState reads what the core decided about the enclosing menu row.
func UseMenuItemState() func() MenuItemState {
	context := menuItemContext.Use()
	if context == nil {
		return settledMenuItem
	}
	return context.live
}

func applyMenuTrigger(node *native.Node, state *menuRootState, props MenuTriggerProps) {
	root := state.props
	reactive.CreateRenderEffect(func() {
		setExplicitBool(node, protocol.Open, state.isOpen())
	})
	setExplicitBool(node, protocol.Modal, root.Modal)
	setString(node, protocol.Orientation, root.Orientation)
	setExplicitBool(node, protocol.LoopFocus, root.LoopFocus)
	setExplicitBool(node, protocol.CloseParentOnEsc, root.CloseParentOnEsc)
	if props.Disabled == nil && root.Disabled {
		setExplicitBool(node, protocol.Disabled, true)
	}
	hover := props.OpenOnHover
	if hover == nil {
		hover = root.OpenOnHover
	}
	setExplicitBool(node, protocol.OpenOnHover, hover)
	delay := props.Delay
	if delay == nil {
		delay = root.Delay
	}
	closeDelay := props.CloseDelay
	if closeDelay == nil {
		closeDelay = root.CloseDelay
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
	})
}

func adoptSurfaceChange(state *menuRootState, event *native.Event) {
	details := ComponentChangeFromEvent(event)
	if details == nil {
		return
	}
	state.reportSurface(details)
	if details.Open != nil && *details.Open != state.isOpen() {
		state.setOpen(*details.Open, event)
	}
}

func createMenuPart(name, part string, props PartProps) *native.Node {
	state := requireMenu(name)
	node := createViewPart(props)
	setPart(node, part, state.scope, "")
	return finishPart(node, props)
}

func createMenuItem(
	name, part string,
	props MenuItemProps,
	declare func(*native.Node),
	report func(*ComponentChangeDetails, *native.Event),
) *native.Node {
	state := requireMenu(name)
	holder := newMenuItemStateHolder()
	node := createViewPart(props.PartProps)
	setPart(node, part, state.scope, props.Value)
	if props.Label != "" {
		setString(node, protocol.AccessibilityLabel, props.Label)
	}
	setExplicitBool(node, protocol.CloseOnClick, props.CloseOnClick)
	declare(node)
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		holder.adopt(details)
		report(details, event)
	})
	return reactive.Provide(menuItemContext, holder, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func ignoreItemReport(*ComponentChangeDetails, *native.Event) {}

func declareNothing(*native.Node) {}

func createMenuRoot(submenu bool, props MenuRootProps) *native.Node {
	state := newMenuRootState(submenu, props)
	return reactive.Provide(menuContext, state, func() *native.Node {
		if props.Children == nil {
			return Fragment(nil)
		}
		return props.Children()
	})
}

// Menu is an in-window menu whose rows are ordinary child nodes.
var Menu = menuAPI{}

type menuAPI struct{}

func (menuAPI) Root(props MenuRootProps) *native.Node {
	return createMenuRoot(false, props)
}

func (menuAPI) SubmenuRoot(props MenuRootProps) *native.Node {
	return createMenuRoot(true, props)
}

func (menuAPI) Trigger(props MenuTriggerProps) *native.Node {
	state := requireMenu("Menu.Trigger")
	node := createButtonPart(props.PartProps)
	setPart(node, protocol.PartMenuTrigger, state.scope, "")
	applyMenuTrigger(node, state, props)
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		adoptSurfaceChange(state, event)
	})
	return finishPart(node, props.PartProps)
}

func (menuAPI) SubmenuTrigger(props MenuSubmenuTriggerProps) *native.Node {
	state := requireMenu("Menu.SubmenuTrigger")
	holder := newMenuItemStateHolder()
	node := createViewPart(props.PartProps)
	value := props.Value
	if value == "" {
		value = state.scope
	}
	setPart(node, protocol.PartMenuSubmenuTrigger, state.scope, value)
	applyMenuTrigger(node, state, props.MenuTriggerProps)
	if props.Label != "" {
		setString(node, protocol.AccessibilityLabel, props.Label)
	}
	setExplicitBool(node, protocol.CloseOnClick, props.CloseOnClick)
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.Highlighted != nil {
			holder.adopt(details)
		}
		if details.Side != "" || details.Align != "" {
			state.reportSurface(details)
		}
		if details.Open != nil && *details.Open != state.isOpen() {
			state.setOpen(*details.Open, event)
		}
	})
	return reactive.Provide(menuItemContext, holder, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (menuAPI) Portal(props PartProps) *native.Node {
	return createMenuPart("Menu.Portal", protocol.PartMenuPortal, props)
}

func (menuAPI) Positioner(props MenuPositionerProps) *native.Node {
	state := requireMenu("Menu.Positioner")
	state.declared.Write(MenuPositioning{
		Side: props.Side, Align: props.Align, SideOffset: props.SideOffset,
		AlignOffset: props.AlignOffset, CollisionPadding: props.CollisionPadding, Sticky: props.Sticky,
	})
	reactive.OnCleanup(func() { state.declared.Write(emptyMenuPositioning()) })
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartMenuPositioner, state.scope, "")
	return finishPart(node, props.PartProps)
}

func (menuAPI) Backdrop(props PartProps) *native.Node {
	return createMenuPart("Menu.Backdrop", protocol.PartMenuBackdrop, props)
}

func (menuAPI) Popup(props PartProps) *native.Node {
	return createMenuPart("Menu.Popup", protocol.PartMenuPopup, props)
}

func (menuAPI) Arrow(props PartProps) *native.Node {
	return createMenuPart("Menu.Arrow", protocol.PartMenuArrow, props)
}

func (menuAPI) Group(props PartProps) *native.Node {
	return createMenuPart("Menu.Group", protocol.PartMenuGroup, props)
}

func (menuAPI) GroupLabel(props MenuGroupLabelProps) *native.Node {
	state := requireMenu("Menu.GroupLabel")
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartMenuGroupLabel, state.scope, props.Value)
	if props.Label != "" {
		setString(node, protocol.AccessibilityLabel, props.Label)
	}
	return finishPart(node, props.PartProps)
}

func (menuAPI) Separator(props PartProps) *native.Node {
	return createMenuPart("Menu.Separator", protocol.PartMenuSeparator, props)
}

func (menuAPI) Item(props MenuItemProps) *native.Node {
	return createMenuItem("Menu.Item", protocol.PartMenuItem, props, declareNothing, ignoreItemReport)
}

func (menuAPI) LinkItem(props MenuLinkItemProps) *native.Node {
	return createMenuItem("Menu.LinkItem", protocol.PartMenuLinkItem, props.MenuItemProps, func(node *native.Node) {
		setMenuLink(node, protocol.Href, props.Href)
	}, func(details *ComponentChangeDetails, event *native.Event) {
		if details.Href == "" || props.OnNavigate == nil {
			return
		}
		props.OnNavigate(details.Href, event)
	})
}

func (menuAPI) CheckboxItem(props MenuCheckboxItemProps) *native.Node {
	return createMenuItem("Menu.CheckboxItem", protocol.PartMenuCheckboxItem, props.MenuItemProps, func(node *native.Node) {
		if props.Checked == nil {
			return
		}
		reactive.CreateRenderEffect(func() {
			setExplicitBool(node, protocol.Checked, props.Checked())
		})
	}, func(details *ComponentChangeDetails, event *native.Event) {
		checked := details.Checked.ptr()
		if checked == nil || props.OnCheckedChange == nil {
			return
		}
		props.OnCheckedChange(*checked, event)
	})
}

func (menuAPI) CheckboxItemIndicator(props PartProps) *native.Node {
	return createMenuPart("Menu.CheckboxItemIndicator", protocol.PartMenuCheckboxItemIndicator, props)
}

func (menuAPI) RadioGroup(props MenuRadioGroupProps) *native.Node {
	state := requireMenu("Menu.RadioGroup")
	group := newMenuRadioGroupState(props)
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartMenuRadioGroup, state.scope, props.Name)
	reactive.CreateRenderEffect(func() {
		setActiveValue(node, group.current())
	})
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if value := details.Value.ptr(); value != nil {
			group.setValue(*value, event)
		}
	})
	return reactive.Provide(menuRadioGroupContext, group, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (menuAPI) RadioItem(props MenuRadioItemProps) *native.Node {
	group := menuRadioGroupContext.Use()
	return createMenuItem("Menu.RadioItem", protocol.PartMenuRadioItem, props.MenuItemProps, func(node *native.Node) {
		reactive.CreateRenderEffect(func() {
			if props.Checked != nil {
				setExplicitBool(node, protocol.Checked, props.Checked())
				return
			}
			if group != nil && props.Value != "" {
				current := group.current()
				setExplicitBool(node, protocol.Checked, current != nil && *current == props.Value)
			}
		})
	}, ignoreItemReport)
}

func (menuAPI) RadioItemIndicator(props PartProps) *native.Node {
	return createMenuPart("Menu.RadioItemIndicator", protocol.PartMenuRadioItemIndicator, props)
}

type MenubarRootProps struct {
	PartProps
	Count          any
	Open           func() *int
	DefaultOpen    *int
	OnOpenChange   func(*int, *native.Event)
	OnActiveChange func(int, *native.Event)
}

type MenubarItemProps struct {
	PartProps
	Index *int
}

var menubarContext = createPartContext[string]()

// Menubar is an in-window menubar that owns which menu is open.
var Menubar = menubarAPI{}

type menubarAPI struct{}

func (menubarAPI) Root(props MenubarRootProps) *native.Node {
	scope := createComponentScope("qg-menubar")
	uncontrolled := reactive.NewSignal(props.DefaultOpen)
	open := func() *int {
		if props.Open != nil {
			return props.Open()
		}
		return uncontrolled.Read()
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartMenubar, scope, "")
	if props.Count != nil {
		setNumber(node, protocol.MenuCount, props.Count)
	}
	reactive.CreateRenderEffect(func() {
		index := open()
		setExplicitBool(node, protocol.Open, index != nil)
		setNumber(node, protocol.ItemIndex, anyPtr(index))
	})
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.OpenIndex.present() {
			next := details.OpenIndex.ptr()
			if props.Open == nil {
				uncontrolled.Write(next)
			}
			if props.OnOpenChange != nil {
				props.OnOpenChange(next, event)
			}
		}
		if details.FocusedIndex != nil && props.OnActiveChange != nil {
			props.OnActiveChange(*details.FocusedIndex, event)
		}
	})
	return reactive.Provide(menubarContext, &scope, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (menubarAPI) Item(props MenubarItemProps) *native.Node {
	node := createButtonPart(props.PartProps)
	scope := ""
	if current := menubarContext.Use(); current != nil {
		scope = *current
	}
	setPart(node, protocol.PartMenubarItem, scope, "")
	if props.Index != nil {
		setNumber(node, protocol.ItemIndex, *props.Index)
	}
	return finishPart(node, props.PartProps)
}
