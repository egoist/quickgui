// Package host binds the QuickGUI C ABI without cgo.
//
// The TypeScript frontend links the host statically through scriptc. Go loads a
// prebuilt shared library at runtime so `go build` stays a plain, fast compile.
// Call [Load] once before any other host function, typically from [github.com/egoist/quickgui/packages/go/native.Run].
package host

import "fmt"

// EventHandler receives one host event. flags bit 0 marks a present value, bit 1 present extra,
// bit 2 present data. The implementation copies every span before returning.
type EventHandler func(kind string, window, target, flags uint32, value, extra string, data []byte)

// API is the C ABI a frontend calls. Implementations must not block on the native main thread.
type API interface {
	ProtocolVersion() uint32
	SetEventCallback(handler EventHandler)
	ClearEventCallback()
	CreateApp(options string) uint32
	PrepareApp(app, request uint32)
	AllocateWindow() uint32
	CreateWindow(app, window uint32, options string, batch []byte)
	CreateSystemPopover(app, window, parent, anchor uint32, options string, batch []byte)
	CreateEmbeddedView(app, window, parent uint32, matchHorizontal, matchVertical bool, options string, batch []byte)
	ApplyBatch(app, window uint32, batch []byte)
	CloseWindow(app, window uint32)
	FocusNode(app, window, node uint32)
	ShowDialog(app, window, request, kind uint32, options string)
	Command(app, request uint32, json string)
	Invoke(request uint32, method, params string)
	Call(method, params string) string
	DestroyApp(app uint32)
	RunHost() int
}

// Current is the process-wide host binding. It is an erroring placeholder until [Load] succeeds.
var Current API = unloaded{}

type unloaded struct{}

func (unloaded) ProtocolVersion() uint32 { return 0 }

func (unloaded) SetEventCallback(EventHandler) {}

func (unloaded) ClearEventCallback() {}

func (unloaded) CreateApp(string) uint32 { return 0 }

func (unloaded) PrepareApp(uint32, uint32) {}

func (unloaded) AllocateWindow() uint32 { return 0 }

func (unloaded) CreateWindow(uint32, uint32, string, []byte) {}

func (unloaded) CreateSystemPopover(uint32, uint32, uint32, uint32, string, []byte) {}

func (unloaded) CreateEmbeddedView(uint32, uint32, uint32, bool, bool, string, []byte) {}

func (unloaded) ApplyBatch(uint32, uint32, []byte) {}

func (unloaded) CloseWindow(uint32, uint32) {}

func (unloaded) FocusNode(uint32, uint32, uint32) {}

func (unloaded) ShowDialog(uint32, uint32, uint32, uint32, string) {}

func (unloaded) Command(uint32, uint32, string) {}

func (unloaded) Invoke(uint32, string, string) {}

func (unloaded) Call(string, string) string {
	return `{"ok":false,"error":"QuickGUI host library is not loaded"}`
}

func (unloaded) DestroyApp(uint32) {}

func (unloaded) RunHost() int {
	fmt.Println("quickgui: host library is not loaded; build it with bun run build:native")
	return 1
}
