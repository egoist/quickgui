package ui

import (
	"fmt"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// ToastType is Base UI's name for the toast kind.
type ToastType string

const (
	ToastInfo    ToastType = "info"
	ToastSuccess ToastType = "success"
	ToastWarning ToastType = "warning"
	ToastError   ToastType = "error"
	ToastLoading ToastType = "loading"
)

// ToastSwipeDirection is which way a swipe dismisses a toast.
type ToastSwipeDirection string

// ToastDeclaration is one queued toast.
type ToastDeclaration struct {
	ID          string    `json:"id"`
	Title       string    `json:"title"`
	Description string    `json:"description,omitempty"`
	Action      string    `json:"action,omitempty"`
	Type        ToastType `json:"type,omitempty"`
	Duration    *float64  `json:"duration,omitempty"`
}

type ToastRequest struct {
	ID          string
	Title       string
	Description string
	Action      string
	Type        ToastType
	Duration    *float64
}

type ToastUpdate struct {
	Title       *string
	Description *string
	Action      *string
	Type        *ToastType
	Duration    *float64
}

type ToastProviderProps struct {
	Children       func() *native.Node
	Timeout        any
	Limit          any
	Expanded       *bool
	SwipeDirection ToastSwipeDirection
	Pitch          any
}

type ToastViewportProps struct {
	PartProps
	Toasts         func() []ToastDeclaration
	Timeout        any
	Limit          any
	Expanded       *bool
	SwipeDirection ToastSwipeDirection
	Pitch          any
	OnDismiss      func([]string, *native.Event)
	OnStackChange  func([]ToastStackEntry, *native.Event)
}

type ToastPartProps struct {
	PartProps
	ToastID string
}

var nextToastID = 1

// ToastManager is a Base UI-shaped manager over the declared toast list.
type ToastManager struct {
	Scope string
	Props ToastProviderProps
	queue *reactive.Signal[[]ToastDeclaration]
	stack *reactive.Signal[[]ToastStackEntry]
}

func newToastManager(props ToastProviderProps) *ToastManager {
	return &ToastManager{
		Scope: createComponentScope("qg-toast"),
		Props: props,
		queue: reactive.NewSignal([]ToastDeclaration{}),
		stack: reactive.NewSignal([]ToastStackEntry{}),
	}
}

func (m *ToastManager) Toasts() []ToastDeclaration { return m.queue.Read() }

func (m *ToastManager) Stack() []ToastStackEntry { return m.stack.Read() }

func (m *ToastManager) Add(toast ToastRequest) string {
	id := toast.ID
	if id == "" {
		id = fmt.Sprintf("qg-toast-%d", nextToastID)
		nextToastID++
	}
	entry := ToastDeclaration{
		ID:          id,
		Title:       toast.Title,
		Description: toast.Description,
		Action:      toast.Action,
		Type:        toast.Type,
		Duration:    toast.Duration,
	}
	current := reactive.Untrack(func() []ToastDeclaration { return m.queue.Read() })
	m.queue.Write(append(append([]ToastDeclaration{}, current...), entry))
	return id
}

func (m *ToastManager) Update(id string, toast ToastUpdate) {
	current := reactive.Untrack(func() []ToastDeclaration { return m.queue.Read() })
	next := make([]ToastDeclaration, 0, len(current))
	for _, entry := range current {
		if entry.ID != id {
			next = append(next, entry)
			continue
		}
		updated := entry
		if toast.Title != nil {
			updated.Title = *toast.Title
		}
		if toast.Description != nil {
			updated.Description = *toast.Description
		}
		if toast.Action != nil {
			updated.Action = *toast.Action
		}
		if toast.Type != nil {
			updated.Type = *toast.Type
		}
		if toast.Duration != nil {
			updated.Duration = toast.Duration
		}
		next = append(next, updated)
	}
	m.queue.Write(next)
}

func (m *ToastManager) Close(id string) {
	current := reactive.Untrack(func() []ToastDeclaration { return m.queue.Read() })
	next := make([]ToastDeclaration, 0, len(current))
	for _, entry := range current {
		if entry.ID != id {
			next = append(next, entry)
		}
	}
	m.queue.Write(next)
}

func (m *ToastManager) CloseAll() {
	m.queue.Write(nil)
}

var toastContext = createPartContext[ToastManager]()

// UseToastManager reads the toast manager inside a Toast.Provider subtree.
func UseToastManager() *ToastManager {
	return requireContext(toastContext, "useToastManager", "Toast.Provider")
}

func createToastPart(part string, props ToastPartProps, button bool) *native.Node {
	manager := toastContext.Use()
	var node *native.Node
	if button {
		node = createButtonPart(props.PartProps)
	} else {
		node = createViewPart(props.PartProps)
	}
	scope := ""
	if manager != nil {
		scope = manager.Scope
	}
	setPart(node, part, scope, props.ToastID)
	return finishPart(node, props.PartProps)
}

// Toast is a toast viewport whose queue bound and swipe arithmetic belong to the core.
var Toast = toastAPI{}

type toastAPI struct{}

func (toastAPI) Provider(props ToastProviderProps) *native.Node {
	manager := newToastManager(props)
	return reactive.Provide(toastContext, manager, func() *native.Node {
		if props.Children == nil {
			return Fragment(nil)
		}
		return props.Children()
	})
}

func (toastAPI) Portal(props PartProps) *native.Node {
	manager := toastContext.Use()
	node := createViewPart(props)
	scope := ""
	if manager != nil {
		scope = manager.Scope
	}
	setPart(node, protocol.PartToastPortal, scope, "")
	return finishPart(node, props)
}

func (toastAPI) Viewport(props ToastViewportProps) *native.Node {
	manager := toastContext.Use()
	node := createViewPart(props.PartProps)
	scope := ""
	if manager != nil {
		scope = manager.Scope
	}
	setPart(node, protocol.PartToastViewport, scope, "")
	reactive.CreateRenderEffect(func() {
		var toasts any
		if props.Toasts != nil {
			toasts = props.Toasts()
		} else if manager != nil {
			toasts = manager.Toasts()
		}
		setJson(node, protocol.Toasts, 65536, toasts)
	})
	var provider *ToastProviderProps
	if manager != nil {
		provider = &manager.Props
	}
	timeout := props.Timeout
	if timeout == nil && provider != nil {
		timeout = provider.Timeout
	}
	limit := props.Limit
	if limit == nil && provider != nil {
		limit = provider.Limit
	}
	expanded := props.Expanded
	if expanded == nil && provider != nil {
		expanded = provider.Expanded
	}
	swipe := props.SwipeDirection
	if swipe == "" && provider != nil {
		swipe = provider.SwipeDirection
	}
	pitch := props.Pitch
	if pitch == nil && provider != nil {
		pitch = provider.Pitch
	}
	setMilliseconds(node, protocol.Timeout, timeout)
	setNumber(node, protocol.Limit, limit)
	if expanded != nil {
		setExplicitBool(node, protocol.StackExpanded, *expanded)
	}
	setString(node, protocol.SwipeDirection, string(swipe))
	setNumber(node, protocol.Pitch, pitch)
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.Dismissed != nil {
			if manager != nil {
				for _, id := range details.Dismissed {
					manager.Close(id)
				}
			}
			if props.OnDismiss != nil {
				props.OnDismiss(details.Dismissed, event)
			}
		}
		if details.Toasts != nil {
			if manager != nil {
				manager.stack.Write(details.Toasts)
			}
			if props.OnStackChange != nil {
				props.OnStackChange(details.Toasts, event)
			}
		}
	})
	return finishPart(node, props.PartProps)
}

func (toastAPI) Positioner(props ToastPartProps) *native.Node {
	return createToastPart(protocol.PartToastPositioner, props, false)
}

func (toastAPI) Root(props ToastPartProps) *native.Node {
	return createToastPart(protocol.PartToast, props, false)
}

func (toastAPI) Content(props ToastPartProps) *native.Node {
	return createToastPart(protocol.PartToastContent, props, false)
}

func (toastAPI) Title(props ToastPartProps) *native.Node {
	return createToastPart(protocol.PartToastTitle, props, false)
}

func (toastAPI) Description(props ToastPartProps) *native.Node {
	return createToastPart(protocol.PartToastDescription, props, false)
}

func (toastAPI) Action(props ToastPartProps) *native.Node {
	return createToastPart(protocol.PartToastAction, props, true)
}

func (toastAPI) Close(props ToastPartProps) *native.Node {
	return createToastPart(protocol.PartToastClose, props, true)
}
