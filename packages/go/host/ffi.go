package host

import (
	"fmt"
	"unsafe"

	"github.com/ebitengine/purego"
)

type loaded struct {
	protocolVersion     func() uint32
	setEventCallback    func(callback, context uintptr)
	clearEventCallback  func(callback, context uintptr)
	createApp           func(options *byte, length uintptr) uint32
	prepareApp          func(app, request uint32) int32
	allocateWindow      func() uint32
	createWindow        func(app, window uint32, options *byte, optionsLen uintptr, batch *byte, batchLen uintptr) int32
	createSystemPopover func(app, window, parent, anchor uint32, options *byte, optionsLen uintptr, batch *byte, batchLen uintptr) int32
	createEmbeddedView  func(app, window, parent uint32, matchH, matchV uint8, options *byte, optionsLen uintptr, batch *byte, batchLen uintptr) int32
	applyBatch          func(app, window uint32, batch *byte, length uintptr) int32
	closeWindow         func(app, window uint32) int32
	focusNode           func(app, window, node uint32) int32
	showDialog          func(app, window, request, kind uint32, options *byte, length uintptr) int32
	command             func(app, request uint32, json *byte, length uintptr) int32
	invoke              func(request uint32, method *byte, methodLen uintptr, params *byte, paramsLen uintptr) int32
	call                func(method *byte, methodLen uintptr, params *byte, paramsLen uintptr, reply, context uintptr) int32
	destroyApp          func(app uint32) int32
	runHost             func() int32
	eventCallback       uintptr
	replyCallback       uintptr
}

var eventHandler EventHandler
var replyText string

func bindLibrary(path string) (API, error) {
	lib, err := purego.Dlopen(path, purego.RTLD_NOW|purego.RTLD_GLOBAL)
	if err != nil {
		return nil, fmt.Errorf("load QuickGUI host %s: %w", path, err)
	}
	api := &loaded{}
	if err := register(lib, "quickgui_protocol_version", &api.protocolVersion); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_set_event_callback", &api.setEventCallback); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_clear_event_callback", &api.clearEventCallback); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_create_app", &api.createApp); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_prepare_app", &api.prepareApp); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_allocate_window", &api.allocateWindow); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_create_window", &api.createWindow); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_create_system_popover", &api.createSystemPopover); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_create_embedded_view", &api.createEmbeddedView); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_apply_batch", &api.applyBatch); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_close_window", &api.closeWindow); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_focus_node", &api.focusNode); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_show_dialog", &api.showDialog); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_command", &api.command); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_invoke", &api.invoke); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_call", &api.call); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_destroy_app", &api.destroyApp); err != nil {
		return nil, err
	}
	if err := register(lib, "quickgui_run_host", &api.runHost); err != nil {
		return nil, err
	}
	api.eventCallback = purego.NewCallback(onHostEvent)
	api.replyCallback = purego.NewCallback(onHostReply)
	return api, nil
}

func register(lib uintptr, name string, fptr any) error {
	sym, err := purego.Dlsym(lib, name)
	if err != nil {
		return fmt.Errorf("QuickGUI host missing %s: %w", name, err)
	}
	purego.RegisterFunc(fptr, sym)
	return nil
}

func onHostEvent(
	kind *byte, kindLen uintptr,
	window, target, flags uint32,
	value *byte, valueLen uintptr,
	extra *byte, extraLen uintptr,
	data *byte, dataLen uintptr,
	_ uintptr,
) {
	handler := eventHandler
	if handler == nil {
		return
	}
	handler(
		cloneString(kind, kindLen),
		window,
		target,
		flags,
		cloneString(value, valueLen),
		cloneString(extra, extraLen),
		cloneBytes(data, dataLen),
	)
}

func onHostReply(json *byte, length uintptr, _ uintptr) {
	replyText = cloneString(json, length)
}

func cloneString(ptr *byte, length uintptr) string {
	if ptr == nil || length == 0 {
		return ""
	}
	return string(unsafe.Slice(ptr, length))
}

func cloneBytes(ptr *byte, length uintptr) []byte {
	if ptr == nil || length == 0 {
		return nil
	}
	src := unsafe.Slice(ptr, length)
	out := make([]byte, length)
	copy(out, src)
	return out
}

func stringSpan(value string) (*byte, uintptr) {
	if value == "" {
		return nil, 0
	}
	return unsafe.StringData(value), uintptr(len(value))
}

func bytesSpan(value []byte) (*byte, uintptr) {
	if len(value) == 0 {
		return nil, 0
	}
	return unsafe.SliceData(value), uintptr(len(value))
}

func (l *loaded) ProtocolVersion() uint32 { return l.protocolVersion() }

func (l *loaded) SetEventCallback(handler EventHandler) {
	eventHandler = handler
	l.setEventCallback(l.eventCallback, 0)
}

func (l *loaded) ClearEventCallback() {
	l.clearEventCallback(l.eventCallback, 0)
	eventHandler = nil
}

func (l *loaded) CreateApp(options string) uint32 {
	ptr, n := stringSpan(options)
	return l.createApp(ptr, n)
}

func (l *loaded) PrepareApp(app, request uint32) {
	l.prepareApp(app, request)
}

func (l *loaded) AllocateWindow() uint32 { return l.allocateWindow() }

func (l *loaded) CreateWindow(app, window uint32, options string, batch []byte) {
	opts, optn := stringSpan(options)
	bytes, n := bytesSpan(batch)
	l.createWindow(app, window, opts, optn, bytes, n)
}

func (l *loaded) CreateSystemPopover(app, window, parent, anchor uint32, options string, batch []byte) {
	opts, optn := stringSpan(options)
	bytes, n := bytesSpan(batch)
	l.createSystemPopover(app, window, parent, anchor, opts, optn, bytes, n)
}

func (l *loaded) CreateEmbeddedView(app, window, parent uint32, matchHorizontal, matchVertical bool, options string, batch []byte) {
	opts, optn := stringSpan(options)
	bytes, n := bytesSpan(batch)
	var mh, mv uint8
	if matchHorizontal {
		mh = 1
	}
	if matchVertical {
		mv = 1
	}
	l.createEmbeddedView(app, window, parent, mh, mv, opts, optn, bytes, n)
}

func (l *loaded) ApplyBatch(app, window uint32, batch []byte) {
	bytes, n := bytesSpan(batch)
	l.applyBatch(app, window, bytes, n)
}

func (l *loaded) CloseWindow(app, window uint32) { l.closeWindow(app, window) }

func (l *loaded) FocusNode(app, window, node uint32) { l.focusNode(app, window, node) }

func (l *loaded) ShowDialog(app, window, request, kind uint32, options string) {
	ptr, n := stringSpan(options)
	l.showDialog(app, window, request, kind, ptr, n)
}

func (l *loaded) Command(app, request uint32, json string) {
	ptr, n := stringSpan(json)
	l.command(app, request, ptr, n)
}

func (l *loaded) Invoke(request uint32, method, params string) {
	m, mn := stringSpan(method)
	p, pn := stringSpan(params)
	l.invoke(request, m, mn, p, pn)
}

func (l *loaded) Call(method, params string) string {
	replyText = ""
	m, mn := stringSpan(method)
	p, pn := stringSpan(params)
	l.call(m, mn, p, pn, l.replyCallback, 0)
	return replyText
}

func (l *loaded) DestroyApp(app uint32) { l.destroyApp(app) }

func (l *loaded) RunHost() int { return int(l.runHost()) }
