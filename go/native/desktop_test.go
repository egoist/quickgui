package native

import (
	"bytes"
	"encoding/json"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/reactive"
)

func installCommandHost(t *testing.T) *commandHost {
	t.Helper()
	fake := &commandHost{}
	restore := host.Install(fake)
	id, ready := appID, appReady
	setAppContext(1, true)
	t.Cleanup(func() { setAppContext(id, ready); restore() })
	return fake
}

func replyCommand(fake *commandHost, kind, value, failure string) {
	App.dispatchHostEvent(hostEvent{kind: kind, target: fake.request, flags: 3, value: value, extra: mustString(map[string]string{"error": failure})})
}

func TestClipboardRepresentationsRoundTripWithoutLosingEmptyOrBinaryValues(t *testing.T) {
	fake := installCommandHost(t)
	item := ClipboardItem{Entries: []ClipboardEntry{
		{Type: "text", Text: "", Metadata: "source"},
		{Type: "data", MIMEType: "application/octet-stream", Data: []byte{0, 255, 42}},
		{Type: "bookmark", Title: "QuickGUI", URL: "https://quickgui.dev"},
		{Type: "files", Paths: []string{"/tmp/file with spaces"}},
	}}
	completed := false
	Clipboard.Write(item, func(err error) {
		completed = true
		if err != nil {
			t.Fatal(err)
		}
	})
	var payload struct {
		Method string
		Item   json.RawMessage
	}
	if err := json.Unmarshal([]byte(fake.payload), &payload); err != nil {
		t.Fatal(err)
	}
	if payload.Method != "write-clipboard" || completed || !bytes.Contains(payload.Item, []byte(`"text":""`)) || !bytes.Contains(payload.Item, []byte(`"data":"AP8q"`)) {
		t.Fatalf("invalid clipboard write: %s", fake.payload)
	}
	replyCommand(fake, "command", "", "")
	Clipboard.Read(func(result *ClipboardItem, err error) {
		if err != nil || result == nil || !reflect.DeepEqual(*result, item) {
			t.Fatalf("read = %+v, %v", result, err)
		}
	})
	replyCommand(fake, "command", string(payload.Item), "")
	for _, response := range []string{`null`, `{"entries":[{"kind":"text","text":""}]}`} {
		Clipboard.ReadText(func(result *string, err error) {
			if err != nil || response == "null" && result != nil || response != "null" && (result == nil || *result != "") {
				t.Fatalf("text = %v, %v", result, err)
			}
		})
		replyCommand(fake, "command", response, "")
	}
}

func TestFileIconDecodesPixelsAndRejectsInvalidDimensions(t *testing.T) {
	fake := installCommandHost(t)
	for _, valid := range []bool{true, false} {
		completed := false
		Desktop.GetFileIcon("/tmp/icon", "", func(result NativeImage, err error) {
			completed = true
			if valid && (err != nil || result.Width != 1 || result.Height != 1 || !bytes.Equal(result.Data, []byte{1, 2, 3, 255})) {
				t.Fatalf("icon = %+v, %v", result, err)
			}
			if !valid && err == nil {
				t.Fatal("accepted truncated pixel data")
			}
		})
		pixels := []byte{1, 2, 3, 255}
		if !valid {
			pixels = pixels[:3]
		}
		App.dispatchHostEvent(hostEvent{kind: "file-icon", target: fake.request, flags: 6, extra: `{"width":1,"height":1}`, data: pixels})
		if !completed {
			t.Fatal("icon request not completed")
		}
	}
}

func TestSubscriptionsPreserveOrderContextAndDisposal(t *testing.T) {
	var listeners subscriptions[int]
	var calls []int
	var stopSecond func()
	stopFirst := listeners.add(func(int) {
		calls = append(calls, 1)
		stopSecond()
		listeners.add(func(int) { calls = append(calls, 3) })
	})
	stopSecond = listeners.add(func(int) { calls = append(calls, 2) })
	listeners.emit(0)
	if !reflect.DeepEqual(calls, []int{1}) {
		t.Fatalf("listener order = %v", calls)
	}
	stopFirst()
	listeners.emit(0)
	if !reflect.DeepEqual(calls, []int{1, 3}) {
		t.Fatalf("listener order = %v", calls)
	}
	listeners.clear()
	context := reactive.CreateContext("outside")
	window := &Window{NodeHost: NewNodeHost(1, 2)}
	reactive.CreateRoot(func(dispose func()) struct{} {
		context.Provide("inside", func() {
			withCurrentWindow(window, func() {
				listeners.add(func(int) {
					if CurrentWindow() != window || context.Use() != "inside" {
						t.Fatal("listener lost its component context")
					}
					calls = append(calls, 4)
				})
			})
		})
		listeners.emit(0)
		dispose()
		listeners.emit(0)
		if len(listeners.order) != 0 || !reflect.DeepEqual(calls, []int{1, 3, 4}) {
			t.Fatal("disposed component retained its subscription")
		}
		return struct{}{}
	})
}

func TestNotificationResponseKeepsActionAndEmptyReply(t *testing.T) {
	defer notificationListeners.clear()
	called := false
	Notifications.OnResponse(func(event NotificationResponseEvent) {
		called = true
		if event.Tag != "reply" || event.ActionID == nil || *event.ActionID != "send" || event.Reply == nil || *event.Reply != "" {
			t.Fatalf("response = %+v", event)
		}
	})
	App.dispatchHostEvent(hostEvent{kind: "notification-response", flags: 1, value: `{"tag":"reply","actionId":"send","reply":""}`})
	if !called {
		t.Fatal("notification response was not dispatched")
	}
}

func TestShortcutWaitsForAcceptanceAndRetainsRegistrationAfterFailedRemoval(t *testing.T) {
	fake := installCommandHost(t)
	defer func() { shortcutRegistrations = map[uint32]*ShortcutRegistration{} }()
	var registration *ShortcutRegistration
	clicks := 0
	GlobalShortcut.Register("CmdOrCtrl+Shift+K", func() { clicks++ }, func(value *ShortcutRegistration, err error) {
		if err != nil {
			t.Fatal(err)
		}
		registration = value
	})
	if GlobalShortcut.IsRegistered("CmdOrCtrl+Shift+K") {
		t.Fatal("registration completed before the native result")
	}
	replyCommand(fake, "command", "", "")
	if registration != nil {
		t.Fatal("command acceptance completed registration")
	}
	replyCommand(fake, "global-shortcut-operation", "", "")
	if registration == nil || !GlobalShortcut.IsRegistered("CmdOrCtrl+Shift+K") {
		t.Fatal("missing accepted shortcut")
	}
	App.dispatchHostEvent(hostEvent{kind: "global-shortcut", target: registration.ID})
	registration.Unregister(func(err error) {
		if err == nil {
			t.Fatal("unregister failure was lost")
		}
	})
	replyCommand(fake, "global-shortcut-operation", "", "temporary failure")
	App.dispatchHostEvent(hostEvent{kind: "global-shortcut", target: registration.ID})
	registration.Unregister(nil)
	replyCommand(fake, "global-shortcut-operation", "", "")
	App.dispatchHostEvent(hostEvent{kind: "global-shortcut", target: registration.ID})
	if clicks != 2 || GlobalShortcut.IsRegistered("CmdOrCtrl+Shift+K") {
		t.Fatal("shortcut lifetime is incorrect")
	}
	GlobalShortcut.Register("Invalid", nil, func(value *ShortcutRegistration, err error) {
		if value != nil || err == nil {
			t.Fatal("failed registration succeeded")
		}
	})
	replyCommand(fake, "global-shortcut-operation", "", "invalid shortcut")
	if len(shortcutRegistrations) != 0 {
		t.Fatal("failed shortcut registration leaked")
	}
}

func TestTrayUpdatesSerializeAndKeepTheLastSuccessfulMenu(t *testing.T) {
	fake := installCommandHost(t)
	defer func() { trayIcons = map[uint32]*TrayIcon{} }()
	clicked := ""
	options := func(name string) TrayIconOptions {
		return TrayIconOptions{Icon: ImageSource{Data: []byte{0, 255}}, Menu: []TrayMenuItem{{Label: name, Click: func() { clicked = name }}}}
	}
	var icon *TrayIcon
	Tray.Create(options("initial"), func(value *TrayIcon, err error) {
		if err != nil {
			t.Fatal(err)
		}
		icon = value
	})
	replyCommand(fake, "tray-operation", "", "")
	icon.Update(options("failed"), func(err error) {
		if err == nil {
			t.Fatal("update failure was lost")
		}
	})
	firstRequest := fake.request
	icon.Update(options("latest"), nil)
	if firstRequest != fake.request {
		t.Fatal("tray updates ran concurrently")
	}
	replyCommand(fake, "tray-operation", "", "invalid icon")
	if firstRequest == fake.request {
		t.Fatal("tray did not advance to the next update")
	}
	dispatchTrayEvent(icon.ID, `{"kind":"menu-item","menuItemId":1}`)
	if clicked != "initial" {
		t.Fatal("failed update discarded the previous menu callback")
	}
	replyCommand(fake, "tray-operation", "", "")
	dispatchTrayEvent(icon.ID, `{"kind":"menu-item","menuItemId":3}`)
	if clicked != "latest" {
		t.Fatal("successful update did not install its callback")
	}
	icon.Destroy(func(err error) {
		if err == nil {
			t.Fatal("remove failure was lost")
		}
	})
	replyCommand(fake, "tray-operation", "", "temporary failure")
	if icon.removed || trayIcons[icon.ID] == nil {
		t.Fatal("failed removal discarded a live tray icon")
	}
	icon.Destroy(nil)
	replyCommand(fake, "tray-operation", "", "")
	clicked = ""
	dispatchTrayEvent(icon.ID, `{"kind":"menu-item","menuItemId":3}`)
	if clicked != "" || trayIcons[icon.ID] != nil || len(icon.callbacks) != 0 {
		t.Fatal("removed tray retained callbacks")
	}
}

func TestImageSourceEncodesTemplateFlag(t *testing.T) {
	payload, err := json.Marshal(TemplateImage("/icons/statusTemplate.png"))
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Contains(payload, []byte(`"path":"/icons/statusTemplate.png"`)) || !bytes.Contains(payload, []byte(`"template":true`)) {
		t.Fatalf("template image: %s", payload)
	}
	disabled := false
	payload, err = json.Marshal(ImageSource{Path: "/icons/statusTemplate.png", Template: &disabled})
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Contains(payload, []byte(`"template":false`)) {
		t.Fatalf("explicit false template: %s", payload)
	}
}

func TestTrayOmitsInferredTemplateFlag(t *testing.T) {
	fake := installCommandHost(t)
	defer func() { trayIcons = map[uint32]*TrayIcon{} }()
	Tray.Create(TrayIconOptions{Icon: ImageSource{Path: "/icons/statusTemplate.png"}}, nil)
	if bytes.Contains([]byte(fake.payload), []byte("iconIsTemplate")) {
		t.Fatalf("inferred template paths should omit iconIsTemplate: %s", fake.payload)
	}
	replyCommand(fake, "tray-operation", "", "")
	Tray.Create(TrayIconOptions{Icon: TemplateImage("/icons/status.png")}, nil)
	if !bytes.Contains([]byte(fake.payload), []byte(`"iconIsTemplate":true`)) {
		t.Fatalf("explicit template: %s", fake.payload)
	}
	replyCommand(fake, "tray-operation", "", "")
}
