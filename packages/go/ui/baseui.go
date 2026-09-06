package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

type AvatarRootProps struct {
	PartProps
	OnLoadingStatusChange func(string, *native.Event)
}

type AvatarImageProps struct {
	PartProps
	Src string
	Fit string
}

type AvatarFallbackProps struct {
	PartProps
	Delay any
}

type avatarState struct {
	scope string
}

var avatarContext = createPartContext[avatarState]()

// Avatar is a compound whose image and fallback mount policy belongs to the core.
var Avatar = avatarAPI{}

type avatarAPI struct{}

func (avatarAPI) Root(props AvatarRootProps) *native.Node {
	state := &avatarState{scope: createComponentScope("qg-avatar")}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartAvatar, state.scope, "")
	if props.OnLoadingStatusChange != nil {
		setListener(node, protocol.EventComponentChange, func(event *native.Event) {
			details := ComponentChangeFromEvent(event)
			if details == nil || details.LoadingStatus == "" {
				return
			}
			props.OnLoadingStatusChange(details.LoadingStatus, event)
		})
	}
	return reactive.Provide(avatarContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (avatarAPI) Image(props AvatarImageProps) *native.Node {
	state := requireContext(avatarContext, "Avatar.Image", "Avatar.Root")
	node := createPart(protocol.TagImage, props.PartProps)
	setPart(node, protocol.PartAvatarImage, state.scope, "")
	setString(node, protocol.Value, props.Src)
	if props.Fit != "" {
		setString(node, protocol.ObjectFit, props.Fit)
	}
	return finishPart(node, props.PartProps)
}

func (avatarAPI) Fallback(props AvatarFallbackProps) *native.Node {
	state := requireContext(avatarContext, "Avatar.Fallback", "Avatar.Root")
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartAvatarFallback, state.scope, "")
	setMilliseconds(node, protocol.Delay, props.Delay)
	return finishPart(node, props.PartProps)
}

// ScrollAreaState is the data-* like render state one scroll area reports.
type ScrollAreaState struct {
	Offset         ScrollOffset
	Scrolling      bool
	Hovering       bool
	HasOverflowX   bool
	HasOverflowY   bool
	OverflowXStart bool
	OverflowXEnd   bool
	OverflowYStart bool
	OverflowYEnd   bool
}

func idleScrollArea() ScrollAreaState {
	return ScrollAreaState{}
}

type ScrollAreaRootProps struct {
	PartProps
	ViewportSize          func() Extent
	ContentSize           func() Extent
	OverflowEdgeThreshold any
	OnScrollStateChange   func(ScrollAreaState, *native.Event)
}

type ScrollAreaScrollbarProps struct {
	PartProps
	Orientation string
	KeepMounted *bool
}

type ScrollAreaThumbProps struct {
	PartProps
	Orientation string
}

type scrollAreaRootState struct {
	scope string
	state *reactive.Signal[ScrollAreaState]
}

func (state *scrollAreaRootState) live() ScrollAreaState {
	return state.state.Read()
}

type scrollbarState struct {
	orientation string
}

var scrollAreaContext = createPartContext[scrollAreaRootState]()
var scrollbarContext = createPartContext[scrollbarState]()

// UseScrollAreaState reads the live scroll state inside a ScrollArea.Root subtree.
func UseScrollAreaState() func() ScrollAreaState {
	state := requireContext(scrollAreaContext, "useScrollAreaState", "ScrollArea.Root")
	return state.live
}

func createScrollAreaPart(name, part string, props PartProps) *native.Node {
	state := requireContext(scrollAreaContext, name, "ScrollArea.Root")
	node := createViewPart(props)
	setPart(node, part, state.scope, "")
	return finishPart(node, props)
}

func extentPtr(value Extent) *Extent {
	copy := value
	return &copy
}

// ScrollArea is a scroll area with caller-drawn scrollbars.
var ScrollArea = scrollAreaAPI{}

type scrollAreaAPI struct{}

func (scrollAreaAPI) Root(props ScrollAreaRootProps) *native.Node {
	state := &scrollAreaRootState{
		scope: createComponentScope("qg-scroll-area"),
		state: reactive.NewSignal(idleScrollArea()),
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartScrollArea, state.scope, "")
	if props.ViewportSize != nil {
		reactive.CreateRenderEffect(func() {
			next := props.ViewportSize()
			setExtent(node, protocol.ViewportSize, extentPtr(next))
		})
	}
	if props.ContentSize != nil {
		reactive.CreateRenderEffect(func() {
			next := props.ContentSize()
			setExtent(node, protocol.ContentSize, extentPtr(next))
		})
	}
	if props.OverflowEdgeThreshold != nil {
		setNumber(node, protocol.OverflowEdgeThreshold, props.OverflowEdgeThreshold)
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || details.Offset == nil || details.HasOverflowY == nil {
			return
		}
		next := ScrollAreaState{
			Offset:         ScrollOffset{X: details.Offset.X, Y: details.Offset.Y},
			Scrolling:      details.Scrolling != nil && *details.Scrolling,
			Hovering:       details.Hovering != nil && *details.Hovering,
			HasOverflowX:   details.HasOverflowX != nil && *details.HasOverflowX,
			HasOverflowY:   *details.HasOverflowY,
			OverflowXStart: details.OverflowXStart != nil && *details.OverflowXStart,
			OverflowXEnd:   details.OverflowXEnd != nil && *details.OverflowXEnd,
			OverflowYStart: details.OverflowYStart != nil && *details.OverflowYStart,
			OverflowYEnd:   details.OverflowYEnd != nil && *details.OverflowYEnd,
		}
		state.state.Write(next)
		if props.OnScrollStateChange != nil {
			props.OnScrollStateChange(next, event)
		}
	})
	return reactive.Provide(scrollAreaContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (scrollAreaAPI) Viewport(props PartProps) *native.Node {
	return createScrollAreaPart("ScrollArea.Viewport", protocol.PartScrollAreaViewport, props)
}

func (scrollAreaAPI) Content(props PartProps) *native.Node {
	return createScrollAreaPart("ScrollArea.Content", protocol.PartScrollAreaContent, props)
}

func (scrollAreaAPI) Scrollbar(props ScrollAreaScrollbarProps) *native.Node {
	state := requireContext(scrollAreaContext, "ScrollArea.Scrollbar", "ScrollArea.Root")
	orientation := props.Orientation
	if orientation == "" {
		orientation = "vertical"
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartScrollAreaScrollbar, state.scope, "")
	setString(node, protocol.Orientation, orientation)
	setExplicitBool(node, protocol.KeepMounted, props.KeepMounted)
	return reactive.Provide(scrollbarContext, &scrollbarState{orientation: orientation}, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (scrollAreaAPI) Thumb(props ScrollAreaThumbProps) *native.Node {
	state := requireContext(scrollAreaContext, "ScrollArea.Thumb", "ScrollArea.Root")
	orientation := props.Orientation
	if orientation == "" {
		if bar := scrollbarContext.Use(); bar != nil {
			orientation = bar.orientation
		} else {
			orientation = "vertical"
		}
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartScrollAreaThumb, state.scope, "")
	setString(node, protocol.Orientation, orientation)
	return finishPart(node, props.PartProps)
}

func (scrollAreaAPI) Corner(props PartProps) *native.Node {
	return createScrollAreaPart("ScrollArea.Corner", protocol.PartScrollAreaCorner, props)
}

type OtpFieldRootProps struct {
	PartProps
	Value          func() string
	DefaultValue   string
	Length         any
	ValidationType string
	Mask           *bool
	ReadOnly       bool
	Required       bool
	AutoSubmit     string
	OnValueChange  func(string, *native.Event)
	OnComplete     func(string, *native.Event)
}

type OtpFieldInputProps struct {
	InputPartProps
	Index int
}

type OtpFieldSeparatorProps struct {
	PartProps
	Index *int
}

type otpFieldState struct {
	scope string
}

var otpFieldContext = createPartContext[otpFieldState]()

// OtpField is a controlled OTP field. Each slot composes the core's own text input.
var OtpField = otpFieldAPI{}

type otpFieldAPI struct{}

func (otpFieldAPI) Root(props OtpFieldRootProps) *native.Node {
	state := &otpFieldState{scope: createComponentScope("qg-otp-field")}
	uncontrolled := reactive.NewSignal(props.DefaultValue)
	value := func() string {
		if props.Value != nil {
			return props.Value()
		}
		return uncontrolled.Read()
	}
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartOtpField, state.scope, "")
	reactive.CreateRenderEffect(func() {
		setString(node, protocol.Value, value())
	})
	if props.Length != nil {
		setNumber(node, protocol.Length, props.Length)
	}
	setString(node, protocol.Variant, props.ValidationType)
	setExplicitBool(node, protocol.Mask, props.Mask)
	if props.ReadOnly {
		setExplicitBool(node, protocol.ReadOnly, true)
	}
	if props.Required {
		setExplicitBool(node, protocol.Required, true)
	}
	if props.AutoSubmit != "" {
		setComponentValue(node, protocol.AutoSubmit, props.AutoSubmit)
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		next := details.Value.ptr()
		if next == nil {
			return
		}
		if props.Value == nil {
			uncontrolled.Write(*next)
		}
		if props.OnValueChange != nil {
			props.OnValueChange(*next, event)
		}
		if details.Complete != "" && props.OnComplete != nil {
			props.OnComplete(details.Complete, event)
		}
	})
	return reactive.Provide(otpFieldContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (otpFieldAPI) Input(props OtpFieldInputProps) *native.Node {
	state := requireContext(otpFieldContext, "OtpField.Input", "OtpField.Root")
	node := createPart(protocol.TagInput, props.PartProps)
	applyInputPart(node, props.InputPartProps)
	setPart(node, protocol.PartOtpFieldInput, state.scope, "")
	setNumber(node, protocol.ItemIndex, props.Index)
	return finishPart(node, props.PartProps)
}

func (otpFieldAPI) Separator(props OtpFieldSeparatorProps) *native.Node {
	state := requireContext(otpFieldContext, "OtpField.Separator", "OtpField.Root")
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartOtpFieldSeparator, state.scope, "")
	if props.Index != nil {
		setNumber(node, protocol.ItemIndex, *props.Index)
	}
	return finishPart(node, props.PartProps)
}

type NavigationMenuRootProps struct {
	PartProps
	Value                       func() *string
	DefaultValue                string
	OnValueChange               func(*string, *native.Event)
	Orientation                 string
	Delay                       any
	CloseDelay                  any
	LoopFocus                   *bool
	Placement                   string
	Items                       []ComponentItem
	OnActivationDirectionChange func(*string, *native.Event)
}

type NavigationMenuItemProps struct {
	PartProps
	Value string
}

type NavigationMenuPartProps struct {
	PartProps
	Value string
}

type NavigationMenuLinkProps struct {
	PartProps
	Value  string
	Active bool
}

type navigationMenuState struct {
	scope string
}

type navigationMenuItemState struct {
	value string
}

var navigationMenuContext = createPartContext[navigationMenuState]()
var navigationMenuItemContext = createPartContext[navigationMenuItemState]()

func navigationMenuPart(name, part string, props NavigationMenuPartProps, button bool) *native.Node {
	state := requireContext(navigationMenuContext, name, "NavigationMenu.Root")
	value := props.Value
	if value == "" {
		if item := navigationMenuItemContext.Use(); item != nil {
			value = item.value
		}
	}
	if value == "" {
		panic(name + " needs a `value`, or a NavigationMenu.Item value ancestor")
	}
	var node *native.Node
	if button {
		node = createButtonPart(props.PartProps)
	} else {
		node = createViewPart(props.PartProps)
	}
	setPart(node, part, state.scope, value)
	return finishPart(node, props.PartProps)
}

// NavigationMenu is a controlled navigation menu whose hover deadlines belong to the core.
var NavigationMenu = navigationMenuAPI{}

type navigationMenuAPI struct{}

func (navigationMenuAPI) Root(props NavigationMenuRootProps) *native.Node {
	state := &navigationMenuState{scope: createComponentScope("qg-navigation-menu")}
	uncontrolled := optionalStringSignal(props.DefaultValue)
	value := controlledString(props.Value, uncontrolled)
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartNavigationMenu, state.scope, "")
	reactive.CreateRenderEffect(func() {
		setActiveValue(node, value())
	})
	setString(node, protocol.Orientation, props.Orientation)
	setMilliseconds(node, protocol.Delay, props.Delay)
	setMilliseconds(node, protocol.CloseDelay, props.CloseDelay)
	setExplicitBool(node, protocol.LoopFocus, props.LoopFocus)
	setString(node, protocol.AnchorPlacement, props.Placement)
	if props.Items != nil {
		setJson(node, protocol.Items, 65536, props.Items)
	}
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || !details.Value.present() {
			return
		}
		next := details.Value.ptr()
		current := value()
		changed := (current == nil) != (next == nil) || (current != nil && next != nil && *current != *next)
		if changed {
			if props.Value == nil {
				uncontrolled.Write(next)
			}
			if props.OnValueChange != nil {
				props.OnValueChange(next, event)
			}
		}
		if props.OnActivationDirectionChange != nil {
			props.OnActivationDirectionChange(details.ActivationDirection.ptr(), event)
		}
	})
	return reactive.Provide(navigationMenuContext, state, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (navigationMenuAPI) List(props PartProps) *native.Node {
	state := requireContext(navigationMenuContext, "NavigationMenu.List", "NavigationMenu.Root")
	node := createViewPart(props)
	setPart(node, protocol.PartNavigationMenuList, state.scope, "")
	return finishPart(node, props)
}

func (navigationMenuAPI) Item(props NavigationMenuItemProps) *native.Node {
	state := requireContext(navigationMenuContext, "NavigationMenu.Item", "NavigationMenu.Root")
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartNavigationMenuItem, state.scope, props.Value)
	return reactive.Provide(navigationMenuItemContext, &navigationMenuItemState{value: props.Value}, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func (navigationMenuAPI) Trigger(props NavigationMenuPartProps) *native.Node {
	return navigationMenuPart("NavigationMenu.Trigger", protocol.PartNavigationMenuTrigger, props, true)
}

func (navigationMenuAPI) Icon(props NavigationMenuPartProps) *native.Node {
	return navigationMenuPart("NavigationMenu.Icon", protocol.PartNavigationMenuIcon, props, false)
}

func (navigationMenuAPI) Portal(props NavigationMenuPartProps) *native.Node {
	return navigationMenuPart("NavigationMenu.Portal", protocol.PartNavigationMenuPortal, props, false)
}

func (navigationMenuAPI) Positioner(props NavigationMenuPartProps) *native.Node {
	return navigationMenuPart("NavigationMenu.Positioner", protocol.PartNavigationMenuPositioner, props, false)
}

func (navigationMenuAPI) Popup(props NavigationMenuPartProps) *native.Node {
	return navigationMenuPart("NavigationMenu.Popup", protocol.PartNavigationMenuPopup, props, false)
}

func (navigationMenuAPI) Viewport(props NavigationMenuPartProps) *native.Node {
	return navigationMenuPart("NavigationMenu.Viewport", protocol.PartNavigationMenuViewport, props, false)
}

func (navigationMenuAPI) Content(props NavigationMenuPartProps) *native.Node {
	return navigationMenuPart("NavigationMenu.Content", protocol.PartNavigationMenuContent, props, false)
}

func (navigationMenuAPI) Arrow(props NavigationMenuPartProps) *native.Node {
	return navigationMenuPart("NavigationMenu.Arrow", protocol.PartNavigationMenuArrow, props, false)
}

func (navigationMenuAPI) Backdrop(props NavigationMenuPartProps) *native.Node {
	return navigationMenuPart("NavigationMenu.Backdrop", protocol.PartNavigationMenuBackdrop, props, false)
}

func (navigationMenuAPI) Link(props NavigationMenuLinkProps) *native.Node {
	state := requireContext(navigationMenuContext, "NavigationMenu.Link", "NavigationMenu.Root")
	node := createButtonPart(props.PartProps)
	setPart(node, protocol.PartNavigationMenuLink, state.scope, props.Value)
	setExplicitBool(node, protocol.Checked, props.Active)
	return finishPart(node, props.PartProps)
}
