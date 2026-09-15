package editor

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"strings"

	"github.com/egoist/quickgui/go/native"
)

// LoadLanguagePack loads a portable Tree-sitter Wasm pack asynchronously. The callback receives
// its canonical language names on the UI goroutine. Registration is shared by Editor, CodeBlock,
// DiffView, filename inference, and injected languages. Call after application readiness.
// path must be an absolute packaged resource path. The single file contains all selected grammars
// and queries; no native libraries are loaded. Identical packs are idempotent; conflicts return errors.
func LoadLanguagePack(path string, done func(languages []string, err error)) {
	if !filepath.IsAbs(path) || strings.ContainsRune(path, '\x00') {
		if done != nil {
			done(nil, fmt.Errorf("language pack path must be absolute"))
		}
		return
	}
	native.InvokeExtension(
		"editor",
		"load-language-pack",
		map[string]any{"path": path},
		func(value string, err error) {
			var names []string
			if err == nil {
				err = json.Unmarshal([]byte(value), &names)
				if err == nil && len(names) == 0 {
					err = fmt.Errorf("invalid language registration reply")
				}
			}
			if done != nil {
				done(names, err)
			}
		},
	)
}
