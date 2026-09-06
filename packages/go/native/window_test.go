package native

import (
	"encoding/json"
	"testing"
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
