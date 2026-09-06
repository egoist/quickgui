package native

import (
	"encoding/json"
	"testing"
)

func TestEncodeMenuKeepsEmptySubmenuItems(t *testing.T) {
	ids := []uint32{}
	encoded := encodeMenuItems([]MenuItem{
		{Label: "Open Repository…", Click: func() {}},
		{Type: "submenu", Label: "Open Recent", Items: nil},
		{Type: "separator"},
		{Type: "role", Label: "Close Window", Role: "close-window"},
	}, &ids)
	payload, err := json.Marshal(encoded)
	if err != nil {
		t.Fatal(err)
	}
	var decoded []map[string]any
	if err := json.Unmarshal(payload, &decoded); err != nil {
		t.Fatal(err)
	}
	if len(decoded) != 4 {
		t.Fatalf("items %d", len(decoded))
	}
	submenu := decoded[1]
	if submenu["type"] != "submenu" {
		t.Fatalf("type %v", submenu["type"])
	}
	items, ok := submenu["items"].([]any)
	if !ok {
		t.Fatalf("submenu omitted items: %s", payload)
	}
	if len(items) != 0 {
		t.Fatalf("expected an empty items array, got %#v", items)
	}
	if _, hasItems := decoded[0]["items"]; hasItems {
		t.Fatal("action items should not declare items")
	}
}
