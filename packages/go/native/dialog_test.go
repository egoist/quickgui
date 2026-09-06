package native

import (
	"encoding/json"
	"testing"
)

func TestEncodeOpenDialogAlwaysIncludesFilters(t *testing.T) {
	payload, err := json.Marshal(encodeOpenDialogOptions(OpenDialogOptions{
		Title: "Open Repository", ButtonLabel: "Open", Properties: []string{"openDirectory"},
	}))
	if err != nil {
		t.Fatal(err)
	}
	var decoded map[string]any
	if err := json.Unmarshal(payload, &decoded); err != nil {
		t.Fatal(err)
	}
	filters, ok := decoded["filters"].([]any)
	if !ok {
		t.Fatalf("host requires filters: %s", payload)
	}
	if len(filters) != 0 {
		t.Fatalf("expected an empty filters array, got %#v", filters)
	}
	if decoded["directories"] != true {
		t.Fatalf("directories %v", decoded["directories"])
	}
	if decoded["files"] != false {
		t.Fatalf("files %v", decoded["files"])
	}
	if decoded["showsHiddenFiles"] != false {
		t.Fatalf("showsHiddenFiles %v", decoded["showsHiddenFiles"])
	}
	if decoded["title"] != "Open Repository" || decoded["prompt"] != "Open" {
		t.Fatalf("labels %s", payload)
	}
}
