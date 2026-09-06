package native

import (
	"encoding/json"

	"github.com/egoist/quickgui/packages/go/host"
	"github.com/egoist/quickgui/packages/go/protocol"
)

// Renderer mounts a view tree into a window and returns a disposer.
type Renderer func(window *Window) func()

// Point is a logical-pixel coordinate.
type Point struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
}

// WindowOptions match the TypeScript Window constructor.
type WindowOptions struct {
	Renderer             Renderer
	Title                string
	Width                float64
	Height               float64
	MinimumWidth         float64
	MinimumHeight        float64
	Background           any
	TitleBarStyle        string
	TrafficLightPosition *Point
	Visible              *bool
	Appearance           string
	Vibrancy             string
	VisualEffectState    string
}

type nativeWindowOptions struct {
	Title             string   `json:"title,omitempty"`
	Width             *float64 `json:"width,omitempty"`
	Height            *float64 `json:"height,omitempty"`
	MinimumWidth      *float64 `json:"minimumWidth,omitempty"`
	MinimumHeight     *float64 `json:"minimumHeight,omitempty"`
	Background        *uint32  `json:"background,omitempty"`
	TitleBarStyle     string   `json:"titleBarStyle,omitempty"`
	TrafficLightX     *float64 `json:"trafficLightX,omitempty"`
	TrafficLightY     *float64 `json:"trafficLightY,omitempty"`
	Show              *bool    `json:"show,omitempty"`
	Appearance        string   `json:"appearance,omitempty"`
	Vibrancy          string   `json:"vibrancy,omitempty"`
	VisualEffectState string   `json:"visualEffectState,omitempty"`
}

// WindowEventName is one window lifecycle notification.
type WindowEventName string

const (
	WindowClosed         WindowEventName = "closed"
	WindowCloseRequested WindowEventName = "closeRequested"
	WindowFocus          WindowEventName = "focus"
	WindowBlur           WindowEventName = "blur"
	WindowResize         WindowEventName = "resize"
	WindowMove           WindowEventName = "move"
	WindowReadyToShow    WindowEventName = "readyToShow"
	WindowAppearance     WindowEventName = "appearanceChange"
)

type WindowEvent struct {
	Window     *Window
	Type       WindowEventName
	Appearance string
}

type windowListener struct {
	id       int
	Type     WindowEventName
	Listener func(WindowEvent)
}

var nextWindowListenerID int

var currentWindow *Window

func withCurrentWindow(window *Window, fn func()) {
	previous := currentWindow
	currentWindow = window
	defer func() { currentWindow = previous }()
	fn()
}

// CurrentWindow is the Window whose renderer or event callback is running.
func CurrentWindow() *Window {
	if currentWindow == nil {
		panic("Window.Current() must be called while rendering or handling a window event")
	}
	return currentWindow
}

// Window is one native window and its retained tree.
type Window struct {
	*NodeHost
	Root           *Node
	App            *Application
	listeners      []windowListener
	mountDisposers []func()
}

// NewWindow allocates a handle, renders the tree, and queues native creation.
func NewWindow(options WindowOptions) *Window {
	if !App.IsReady() {
		panic("call native.Run before creating a QuickGUI Window")
	}
	window := &Window{
		NodeHost: NewNodeHost(App.NativeID, host.Current.AllocateWindow()),
		App:      App,
	}
	window.Root = CreateRootNode(window.NodeHost, protocol.RootNodeID)
	native := encodeWindowOptions(options)
	encoded, _ := json.Marshal(native)
	var dispose func()
	withCurrentWindow(window, func() {
		if options.Renderer != nil {
			dispose = options.Renderer(window)
		}
	})
	if dispose != nil {
		window.mountDisposers = append(window.mountDisposers, dispose)
	}
	initial := window.TakeBatch()
	host.Current.CreateWindow(window.AppID, window.NativeID, string(encoded), initial)
	window.NativeReady = true
	App.registerWindow(window)
	return window
}

func encodeWindowOptions(options WindowOptions) nativeWindowOptions {
	native := nativeWindowOptions{Title: options.Title, TitleBarStyle: options.TitleBarStyle}
	if options.Width > 0 {
		native.Width = &options.Width
	}
	if options.Height > 0 {
		native.Height = &options.Height
	}
	if options.MinimumWidth > 0 {
		native.MinimumWidth = &options.MinimumWidth
	}
	if options.MinimumHeight > 0 {
		native.MinimumHeight = &options.MinimumHeight
	}
	if options.Background != nil {
		color := ParseColor(options.Background)
		native.Background = &color
	}
	if options.TrafficLightPosition != nil {
		native.TrafficLightX = &options.TrafficLightPosition.X
		native.TrafficLightY = &options.TrafficLightPosition.Y
	}
	native.Show = options.Visible
	native.Appearance = options.Appearance
	native.Vibrancy = options.Vibrancy
	native.VisualEffectState = options.VisualEffectState
	return native
}

func (w *Window) On(eventType WindowEventName, listener func(WindowEvent)) func() {
	if w.Closed {
		return func() {}
	}
	nextWindowListenerID++
	entry := windowListener{id: nextWindowListenerID, Type: eventType, Listener: listener}
	w.listeners = append(w.listeners, entry)
	return func() {
		for i, existing := range w.listeners {
			if existing.id == entry.id {
				w.listeners = append(w.listeners[:i], w.listeners[i+1:]...)
				return
			}
		}
	}
}

func (w *Window) OnClose(listener func(*Window)) func() {
	return w.On(WindowClosed, func(event WindowEvent) { listener(event.Window) })
}

func (w *Window) emit(eventType WindowEventName) {
	w.emitEvent(WindowEvent{Window: w, Type: eventType})
}

func (w *Window) emitEvent(event WindowEvent) {
	snapshot := append([]windowListener(nil), w.listeners...)
	withCurrentWindow(w, func() {
		for _, entry := range snapshot {
			if entry.Type == event.Type {
				entry.Listener(event)
			}
		}
	})
}

func (w *Window) Close() {
	w.App.closeWindow(w)
}

func (w *Window) Focus() {
	w.Action("focus", "")
}

func (w *Window) SetTitle(title string) {
	w.Action("set-title", title)
}

func (w *Window) SetRepresentedFile(path string) {
	w.Action("set-represented-file", path)
}

func (w *Window) Action(action, value string) {
	if w.Closed {
		return
	}
	payload := map[string]any{"method": "window-action", "window": w.NativeID, "action": action}
	if value != "" {
		payload["value"] = value
	}
	encoded, _ := json.Marshal(payload)
	SendMutation(string(encoded))
}

func (w *Window) didClose() {
	if w.Closed {
		return
	}
	w.Closed = true
	for i := len(w.mountDisposers) - 1; i >= 0; i-- {
		w.mountDisposers[i]()
	}
	w.mountDisposers = nil
	w.emit(WindowClosed)
}

func (w *Window) didRequestClose() {
	if w.Closed {
		return
	}
	w.emit(WindowCloseRequested)
}

func (w *Window) didObserveLifecycle(kind, value string) {
	if w.Closed {
		return
	}
	switch kind {
	case "window-focus":
		if value == "true" {
			w.emit(WindowFocus)
		} else {
			w.emit(WindowBlur)
		}
	case "window-resize":
		w.emit(WindowResize)
	case "window-move":
		w.emit(WindowMove)
	case "window-ready-to-show":
		w.emit(WindowReadyToShow)
	case "window-appearance":
		appearance := "light"
		if value == "dark" {
			appearance = "dark"
		}
		w.emitEvent(WindowEvent{Window: w, Type: WindowAppearance, Appearance: appearance})
	}
}
