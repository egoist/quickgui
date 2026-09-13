// Package {{GO_PACKAGE}} wraps an independently built {{TYPE}} service extension.
package {{GO_PACKAGE}}

import (
	_ "embed"
	"encoding/json"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/native"
)

// The manifest supplies the same identity and version to Go, the native build,
// and the CLI's artifact resolver. Importing this package does not load native code.
//
//go:embed quickgui.extension.json
var manifestJSON []byte

var manifest struct {
	Name    string `json:"name"`
	Version string `json:"version"`
}

func init() {
	if err := json.Unmarshal(manifestJSON, &manifest); err != nil {
		panic(err)
	}
	host.RequireExtension(manifest.Name, manifest.Version)
}

// Echo replies on the QuickGUI UI goroutine. Call it from a window event or
// effect after native.Run has started; done must not be nil.
func Echo(message string, done func(string, error)) {
	native.InvokeExtension(
		manifest.Name,
		"echo",
		message,
		func(raw string, err error) {
			var reply string
			if err == nil {
				err = json.Unmarshal([]byte(raw), &reply)
			}
			done(reply, err)
		},
	)
}
