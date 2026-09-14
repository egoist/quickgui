package ffi

import (
	"os"
	"strings"
	"testing"
	"time"

	"github.com/egoist/quickgui/go/protocol"
)

// This headless smoke is also run against real staged images by check-extensions.ts.
func TestExtensionLibrarySmoke(t *testing.T) {
	core := os.Getenv("QUICKGUI_TEST_CORE")
	terminal := os.Getenv("QUICKGUI_TEST_TERMINAL")
	if core == "" || terminal == "" {
		t.Skip("native extension images not supplied")
	}
	library, err := Load(core, protocol.Version)
	if err != nil {
		t.Fatal(err)
	}
	if err := library.LoadExtension("wrong-package", terminal); err == nil {
		t.Fatal("registration must reject a descriptor for a different package")
	}
	if err := library.LoadExtension("terminal", terminal); err != nil {
		t.Fatal(err)
	}
	if err := library.LoadExtension("terminal", terminal); err != nil {
		t.Fatal(err)
	}
}

func TestIndependentComponentLibrarySmoke(t *testing.T) {
	core, provider := os.Getenv("QUICKGUI_TEST_CORE"), os.Getenv("QUICKGUI_TEST_COMPONENT")
	if core == "" || provider == "" {
		t.Skip("component images not supplied")
	}
	library, err := Load(core, protocol.Version)
	if err != nil {
		t.Fatal(err)
	}
	if err := library.LoadExtension("acme-counter", provider, "7.2.0"); err == nil {
		t.Fatal("wrong component version was accepted")
	}
	if err := library.LoadExtension("unrelated", provider, "7.2.1"); err == nil {
		t.Fatal("wrong component identity was accepted")
	}
	for i := 0; i < 2; i++ {
		if err := library.LoadExtension("acme-counter", provider, "7.2.1"); err != nil {
			t.Fatal(err)
		}
	}
}

func TestIndependentServiceLibrarySmoke(t *testing.T) {
	core, provider := os.Getenv("QUICKGUI_TEST_CORE"), os.Getenv("QUICKGUI_TEST_SERVICE")
	if core == "" || provider == "" {
		t.Skip("independent provider images not supplied")
	}
	name, version := os.Getenv("QUICKGUI_TEST_SERVICE_NAME"), os.Getenv("QUICKGUI_TEST_SERVICE_VERSION")
	if name == "" {
		name = "acme-echo"
	}
	if version == "" {
		version = "1.0.0"
	}
	library, err := Load(core, protocol.Version)
	if err != nil {
		t.Fatal(err)
	}
	if err := library.LoadExtension(name, provider, "9.9.9"); err == nil {
		t.Fatal("wrong provider version was accepted")
	}
	if err := library.LoadExtension("wrong-name", provider, version); err == nil {
		t.Fatal("wrong provider identity was accepted")
	}
	for i := 0; i < 2; i++ {
		if err := library.LoadExtension(name, provider, version); err != nil {
			t.Fatal(err)
		}
	}
	events := make(chan Event, 8)
	defer library.Listen(func(event Event) { events <- event })()
	invoke := func(method, params string) Event {
		t.Helper()
		m, p := []byte(method), []byte(params)
		if library.Invoke(712, m, uintptr(len(m)), p, uintptr(len(p))) != 0 {
			t.Fatal("native invoke rejected the request")
		}
		select {
		case event := <-events:
			if event.Kind != "invoke" || event.Target != 712 {
				t.Fatalf("wrong reply route: %+v", event)
			}
			return event
		case <-time.After(5 * time.Second):
			t.Fatal("extension never replied")
			return Event{}
		}
	}
	const payload = `{ "keep": null, "nested": {"zero":0,"null":null}, "integer": 900719925474099312345, "fraction": 0.12345678901234567890123 }`
	method := "extension/" + name + "/"
	result := invoke(method+"echo", payload)
	if result.Value != payload {
		t.Fatalf("core changed provider JSON: %s", result.Value)
	}
	if result := invoke(method+"echo", `not json`); !strings.Contains(result.Extra, "invalid extension response") {
		t.Fatalf("malformed provider JSON was accepted: %+v", result)
	}
	if result := invoke(method+"missing", `null`); !strings.Contains(result.Extra, "Unknown method") {
		t.Fatalf("provider error was lost: %+v", result)
	}
	if result := invoke(method+"echo", strings.Repeat("x", 64*1024+1)); !strings.Contains(result.Extra, "oversized") {
		t.Fatal("oversized input was accepted")
	}
	select {
	case extra := <-events:
		t.Fatalf("duplicate reply: %+v", extra)
	default:
	}
}
