package native

import (
	"encoding/json"
	"fmt"
	"os"
	"runtime"

	"github.com/egoist/quickgui/packages/go/host"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// App is the process-wide hosted application, matching the TypeScript `app` singleton.
var App = &Application{}

// Application holds native app identity, windows, and lifecycle listeners.
type Application struct {
	NativeID uint32
	Windows  map[uint32]*Window
	ready    bool
	exited   bool
	onReady  []func()
	onReopen []func(ReopenEvent)
}

// ReopenEvent is a macOS dock-click or equivalent reopen.
type ReopenEvent struct {
	HasVisibleWindows bool
}

// AppOptions configure identity and quit policy before the first readiness turn.
type AppOptions struct {
	Name       string
	Version    string
	Identifier string
	QuitMode   string
	Fonts      []string
}

type nativeAppOptions struct {
	Name       string   `json:"name,omitempty"`
	Version    string   `json:"version,omitempty"`
	Identifier string   `json:"identifier,omitempty"`
	QuitMode   string   `json:"quitMode,omitempty"`
	Fonts      []string `json:"fonts,omitempty"`
}

func (a *Application) IsReady() bool { return a.ready }

func (a *Application) OnReady(listener func()) {
	if a.ready {
		listener()
		return
	}
	a.onReady = append(a.onReady, listener)
}

func (a *Application) OnReopen(listener func(ReopenEvent)) {
	a.onReopen = append(a.onReopen, listener)
}

func (a *Application) registerWindow(window *Window) {
	if a.Windows == nil {
		a.Windows = map[uint32]*Window{}
	}
	a.Windows[window.NativeID] = window
}

func (a *Application) closeWindow(window *Window) {
	if window.Closed {
		return
	}
	host.Current.CloseWindow(a.NativeID, window.NativeID)
}

func (a *Application) didCloseWindow(window *Window) {
	delete(a.Windows, window.NativeID)
	window.didClose()
}

func (a *Application) dispatchHostEvent(ev hostEvent) {
	var extra struct {
		Error string `json:"error"`
	}
	hasValue := ev.flags&1 != 0
	hasExtra := ev.flags&2 != 0
	if hasExtra && ev.extra != "" {
		_ = json.Unmarshal([]byte(ev.extra), &extra)
	}
	value := ev.value
	if !hasValue {
		value = ""
	}
	switch ev.kind {
	case "app-ready":
		if extra.Error != "" {
			fmt.Fprintln(os.Stderr, "quickgui:", extra.Error)
			os.Exit(1)
		}
		a.ready = true
		setAppContext(a.NativeID, true)
		for _, listener := range a.onReady {
			listener()
		}
		return
	case "command", "invoke":
		var err error
		if extra.Error != "" {
			err = fmt.Errorf("%s", extra.Error)
		}
		settleReply(ev.target, value, err)
		return
	case "exit":
		a.exited = true
		rejectAllReplies(fmt.Errorf("the QuickGUI application exited"))
		return
	case "host-error":
		msg := extra.Error
		if msg == "" {
			msg = "the native host failed"
		}
		fmt.Fprintln(os.Stderr, "quickgui:", msg)
		return
	case "reopen":
		event := ReopenEvent{HasVisibleWindows: value == "true"}
		for _, listener := range a.onReopen {
			listener(event)
		}
		return
	}
	owner := a.Windows[ev.window]
	if owner == nil {
		return
	}
	if ev.kind == "close" {
		a.didCloseWindow(owner)
		return
	}
	if ev.kind == "close-requested" {
		owner.didRequestClose()
		return
	}
	if len(ev.kind) >= 7 && ev.kind[:7] == "window-" {
		owner.didObserveLifecycle(ev.kind, value)
		return
	}
	eventType := protocol.EventTypeFromKind(ev.kind)
	if eventType == 0 {
		return
	}
	withCurrentWindow(owner, func() {
		DispatchEvent(owner.NodeHost, eventType, ev.target, value, hasValue)
	})
	owner.Flush()
}

// Run loads the host shared library, starts the application goroutine, and owns the
// process main thread for AppKit/Winit. start runs after the host is ready, the same
// way TypeScript awaits `app.whenReady()`.
func Run(start func()) {
	runtime.LockOSThread()
	if err := host.Load(); err != nil {
		fmt.Fprintln(os.Stderr, "quickgui:", err)
		os.Exit(1)
	}
	go runApplication(start)
	os.Exit(host.Current.RunHost())
}

func runApplication(start func()) {
	defer func() {
		if recovered := recover(); recovered != nil {
			fmt.Fprintln(os.Stderr, "quickgui:", recovered)
			os.Exit(1)
		}
	}()
	if version := host.Current.ProtocolVersion(); version != protocol.Version {
		panic(fmt.Sprintf("QuickGUI native protocol mismatch: the application uses %d, the host uses %d", protocol.Version, version))
	}
	host.Current.SetEventCallback(enqueueHostEvent)
	options, _ := json.Marshal(nativeAppOptions{})
	id := host.Current.CreateApp(string(options))
	if id == 0 {
		panic("the QuickGUI host refused to create the application")
	}
	App.NativeID = id
	setAppContext(id, false)
	pendingReplies[readyRequest] = func(string, error) {}
	host.Current.PrepareApp(id, readyRequest)
	for !App.ready {
		<-workWake
		processTurns()
	}
	start()
	FlushPending()
	for !App.exited {
		<-workWake
		processTurns()
	}
}

func processTurns() {
	for {
		events := drainEvents()
		jobs := drainWork()
		if len(events) == 0 && len(jobs) == 0 {
			return
		}
		for _, ev := range events {
			App.dispatchHostEvent(ev)
		}
		for _, job := range jobs {
			job()
		}
		FlushPending()
		reactive.Flush()
	}
}
