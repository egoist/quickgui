package native

import (
	"encoding/json"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/reactive"
)

func TestEncodeWindowOptionsSendsEmbeddedChrome(t *testing.T) {
	decorated := false
	shadow := false
	visible := false
	encoded, err := json.Marshal(encodeWindowOptions(WindowOptions{
		Title:                "QuickGUI SwiftUI embedded view",
		Width:                320,
		Height:               200,
		Visible:              &visible,
		Decorated:            &decorated,
		Shadow:               &shadow,
		BackgroundAppearance: "transparent",
	}))
	if err != nil {
		t.Fatal(err)
	}
	var payload map[string]any
	if err := json.Unmarshal(encoded, &payload); err != nil {
		t.Fatal(err)
	}
	if payload["decorated"] != false || payload["shadow"] != false || payload["show"] != false {
		t.Fatalf("%s", encoded)
	}
	if payload["transparent"] != true {
		t.Fatalf("transparent %v", payload["transparent"])
	}
}

func TestWindowOptionsPreserveExplicitZeroFalseAndCompoundDeclarations(t *testing.T) {
	zero, no := 0.0, false
	options := WindowOptions{
		Position: &Point{}, DisableMinimumSize: true, MinimumSize: &Size{Width: 400, Height: 300},
		MaximumSize: &Size{Width: 1000, Height: 800}, Focus: &no, Resizable: &no, Opacity: &zero,
		BackgroundAppearance: "opaque", Blur: &no, CursorPosition: &Point{},
		TaskbarProgress: &TaskbarProgress{State: "normal", Progress: 0},
		TaskbarOverlay:  &TaskbarOverlay{Icon: ImageSource{Path: "/icon.png"}, Description: "Download"},
		Offset:          &Point{X: 0, Y: 8}, AcceptsKeyFocus: &no,
	}
	var payload map[string]any
	if err := json.Unmarshal([]byte(mustString(encodeWindowOptions(options))), &payload); err != nil {
		t.Fatal(err)
	}
	for key, want := range map[string]any{"x": 0.0, "y": 0.0, "minimumSizeEnabled": false, "maximumWidth": 1000.0, "focus": false, "resizable": false, "opacity": 0.0, "cursorX": 0.0, "taskbarProgress": 0.0, "transparent": false, "blur": false, "popoverOffsetX": 0.0, "popoverOffsetY": 8.0, "popoverAcceptsKeyFocus": false} {
		if got, present := payload[key]; !present || got != want {
			t.Fatalf("%s = %v (present %v), want %v", key, got, present, want)
		}
	}
	if _, present := payload["minimumWidth"]; present {
		t.Fatal("disabled minimum size retained a minimum")
	}
	if payload["taskbarOverlayDescription"] != "Download" {
		t.Fatal("overlay description was lost")
	}
}

func TestWindowStateIncludesGeometryCursorAndNativeTabs(t *testing.T) {
	fake := installCommandHost(t)
	window := &Window{NodeHost: NewNodeHost(1, 2)}
	var state WindowState
	window.GetState(func(value WindowState, err error) {
		if err != nil {
			t.Fatal(err)
		}
		state = value
	})
	replyCommand(fake, "command", `{"x":-20,"y":0,"width":800,"height":600,"viewportWidth":800,"viewportHeight":572,"minimumWidth":200,"minimumHeight":100,"cursorX":0,"cursorY":0,"nativeTabCount":2,"nativeSelectedTab":0,"nativeTabBarVisible":true,"scaleFactor":2,"windowLevel":"floating","focusable":false}`, "")
	if state.Bounds.X != -20 || state.ViewportSize.Height != 572 || state.MinimumSize == nil || state.MinimumSize.Width != 200 || state.CursorPosition == nil || *state.CursorPosition != (Point{}) || state.NativeTabs.Count != 2 || state.NativeTabs.SelectedIndex == nil || *state.NativeTabs.SelectedIndex != 0 || !state.NativeTabs.TabBarVisible || state.WindowLevel != "floating" {
		t.Fatalf("incomplete snapshot: %+v", state)
	}
}

func TestFrameMetricsAreReadOnDemandForOneWindow(t *testing.T) {
	fake := installCommandHost(t)
	window := &Window{NodeHost: NewNodeHost(1, 2)}

	called := false
	Metrics.GetFrameMetrics(window, func(metrics *FrameMetrics, err error) {
		called = true
		if err != nil || metrics != nil {
			t.Fatalf("metrics before first frame = %+v, %v", metrics, err)
		}
	})
	var request struct {
		Method string `json:"method"`
		Window uint32 `json:"window"`
	}
	if err := json.Unmarshal([]byte(fake.payload), &request); err != nil {
		t.Fatal(err)
	}
	if request.Method != "get-window-frame-metrics" || request.Window != window.NativeID {
		t.Fatalf("unexpected frame metrics request: %s", fake.payload)
	}
	replyCommand(fake, "command", `{"frameNumber":0}`, "")
	if !called {
		t.Fatal("frame metrics request did not complete")
	}

	var received *FrameMetrics
	Metrics.GetFrameMetrics(window, func(metrics *FrameMetrics, err error) {
		if err != nil {
			t.Fatal(err)
		}
		received = metrics
	})
	replyCommand(fake, "command", `{"frameNumber":42,"cpuMilliseconds":1.25,"smoothedCpuMilliseconds":1.5,"frameMilliseconds":8,"smoothedFrameMilliseconds":10}`, "")
	if received == nil || received.FrameNumber != 42 || received.SmoothedFrameMilliseconds != 10 {
		t.Fatalf("frame metrics = %+v", received)
	}
}

func TestWindowInterceptionAndEventPayloadsHaveNativeParity(t *testing.T) {
	fake := installCommandHost(t)
	window := &Window{NodeHost: NewNodeHost(1, 2)}
	window.NativeReady = true
	var events []WindowEventName
	stop := window.OnCloseRequested(func(w *Window) {
		if CurrentWindow() != w {
			t.Fatal("close lost window context")
		}
		events = append(events, WindowCloseRequested)
	})
	if !window.closeIntercepting {
		t.Fatal("native close was not intercepted")
	}
	window.didRequestClose()
	stop()
	window.didRequestClose()
	if window.closeIntercepting || !reflect.DeepEqual(events, []WindowEventName{WindowCloseRequested}) {
		t.Fatal("close interception did not follow its subscriptions")
	}
	window.On(WindowResize, func(event WindowEvent) {
		if event.Size == nil || event.Size.Width != 321 {
			t.Fatal("resize dimensions lost")
		}
		events = append(events, event.Type)
	})
	window.On(WindowOcclusionChange, func(event WindowEvent) {
		if event.Occluded == nil || *event.Occluded {
			t.Fatal("false occlusion lost")
		}
		events = append(events, event.Type)
	})
	window.didObserveLifecycle("window-resize", `{"width":321,"height":123}`)
	window.didObserveLifecycle("window-occlusion", "false")
	window.SetTitle("")
	var payload map[string]any
	if err := json.Unmarshal([]byte(fake.payload), &payload); err != nil {
		t.Fatal(err)
	}
	if value, present := payload["value"]; !present || value != "" {
		t.Fatal("empty title was omitted")
	}
	if len(events) != 3 {
		t.Fatal("missing window lifecycle event")
	}
}

func TestFailedComponentConstructionDisposesOwnersAndMenus(t *testing.T) {
	fake := &windowHost{}
	defer host.Install(fake)()
	previous := App
	App = &Application{NativeID: 1, ready: true}
	defer func() { App = previous; pendingFlush = nil }()
	cleanups := 0
	before := len(menuCallbacks)
	func() {
		defer func() {
			if recover() == nil {
				t.Fatal("component panic was swallowed")
			}
		}()
		NewWindow(WindowOptions{
			Menu: []MenuDefinition{{Label: "File", Items: []MenuItem{{Label: "Action", Click: func() {}}}}},
			Component: func() *Node {
				var children []*Node
				reactive.OnCleanup(func() { cleanups++ })
				children = append(children, CreateText("partial"))
				panic("mount failed")
				return testFragment(children)
			},
		})
	}()
	if cleanups != 1 || len(menuCallbacks) != before || fake.created != 0 || len(App.Windows) != 0 {
		t.Fatal("failed window creation leaked resources")
	}
}

type windowHost struct {
	host.Fake
	created int
	closed  int
}

func (h *windowHost) AllocateWindow() uint32                       { return 42 }
func (h *windowHost) CreateWindow(_, _ uint32, _ string, _ []byte) { h.created++ }
func (h *windowHost) CloseWindow(_, _ uint32)                      { h.closed++ }

func TestWindowOwnsComponentLifetime(t *testing.T) {
	fake := &windowHost{}
	defer host.Install(fake)()
	previous := App
	App = &Application{NativeID: 1, ready: true}
	defer func() { App = previous; pendingFlush = nil }()
	value, setValue := reactive.CreateSignal("first")
	mounts, effects, cleanups := 0, 0, 0
	var text *Node
	window := NewWindow(WindowOptions{Component: func() *Node {
		if CurrentWindow() == nil {
			t.Fatal("missing component window")
		}
		mounts++
		text = CreateText("")
		text.Bind(func() { effects++; ReplaceText(text, value()) })
		sibling := CreateText("sibling")
		reactive.OnCleanup(func() { cleanups++ })
		return testFragment([]*Node{text, sibling})
	}})
	setValue("second")
	if mounts != 1 || effects != 2 || text.Text != "second" || fake.created != 1 {
		t.Fatal("the component remounted or its binding did not update")
	}
	if len(window.Root.Children) != 3 || window.Root.Children[0] != text || window.Root.Children[1].Text != "sibling" || len(window.Root.Children[2].Group) != 2 {
		t.Fatal("window components must mount the explicitly returned fragment once")
	}
	App.didCloseWindow(window)
	App.didCloseWindow(window)
	setValue("after close")
	if effects != 2 || cleanups != 1 || window.Root != nil || len(App.Windows) != 0 || fake.closed != 0 {
		t.Fatal("window closure did not dispose the component exactly once")
	}
}

func TestDeferredBindingsKeepTheirOwningWindow(t *testing.T) {
	fake := &windowHost{}
	defer host.Install(fake)()
	previous := App
	App = &Application{NativeID: 1, ready: true}
	defer func() { App = previous; pendingFlush = nil }()
	value, setValue := reactive.CreateSignal(0)
	var observed *Window
	var events subscriptions[struct{}]
	window := NewWindow(WindowOptions{Component: func() *Node {
		node := CreateText("")
		node.Bind(func() {
			value()
			observed = CurrentWindow()
			events.add(func(struct{}) { observed = CurrentWindow() })
		})
		return node

	}})
	defer App.didCloseWindow(window)
	setValue(1)
	if observed != window {
		t.Fatal("deferred binding lost its window")
	}
	other := &Window{NodeHost: NewNodeHost(1, 99)}
	withCurrentWindow(other, func() { setValue(2) })
	if observed != window {
		t.Fatal("cross-window update adopted the event sender's window")
	}
	observed = nil
	events.emit(struct{}{})
	if observed != window || currentWindow != nil {
		t.Fatal("callback lost or leaked its captured window")
	}
}
