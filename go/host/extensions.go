package host

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strings"
	"sync"
)

var extensionNames = regexp.MustCompile(`^[a-z][a-z0-9-]{0,63}$`)
var extensionVersions = regexp.MustCompile(`^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$`)
var extensionState = struct {
	sync.Mutex
	names  map[string]string
	loaded bool
}{names: make(map[string]string)}

// RequireExtension declares an optional backend from a package's init function.
// Pass an exact version for an independently released extension; omitting it keeps
// the built-in core-release requirement. No native code or UI work runs during init.
func RequireExtension(name string, version ...string) {
	extensionState.Lock()
	defer extensionState.Unlock()
	if len(version) > 1 {
		panic("QuickGUI extension requires one exact version")
	}
	expected := ""
	if len(version) == 1 {
		expected = version[0]
		if len(expected) > 64 || !extensionVersions.MatchString(expected) {
			panic("QuickGUI extension version must be an exact release")
		}
	}
	if previous, exists := extensionState.names[name]; exists {
		if previous != expected {
			panic("conflicting QuickGUI extension versions: " + name)
		}
		return
	}
	if name == "host" || !extensionNames.MatchString(name) || len(extensionState.names) >= 32 {
		panic("QuickGUI extension name is invalid or the extension limit was exceeded")
	}
	if extensionState.loaded {
		panic("QuickGUI extensions must be declared before native.Run")
	}
	extensionState.names[name] = expected
}

type extensionRequirement struct{ name, version string }

func requiredExtensions() []extensionRequirement {
	extensionState.Lock()
	defer extensionState.Unlock()
	extensionState.loaded = true
	names := make([]extensionRequirement, 0, len(extensionState.names))
	for name, version := range extensionState.names {
		names = append(names, extensionRequirement{name, version})
	}
	sort.Slice(names, func(i, j int) bool { return names[i].name < names[j].name })
	return names
}

func extensionLibraryName(name string) string {
	name = strings.ReplaceAll(name, "-", "_")
	switch runtime.GOOS {
	case "darwin":
		return "libquickgui_" + name + ".dylib"
	case "windows":
		return "quickgui_" + name + ".dll"
	default:
		return "libquickgui_" + name + ".so"
	}
}

func findExtensionLibrary(name, corePath string) (string, error) {
	file := extensionLibraryName(name)
	if directory := os.Getenv("QUICKGUI_EXTENSION_DIR"); directory != "" {
		path := filepath.Join(directory, file)
		if fileExists(path) {
			return filepath.Abs(path)
		}
		return "", fmt.Errorf("QuickGUI extension %s is missing from QUICKGUI_EXTENSION_DIR", name)
	}
	directory := filepath.Dir(corePath)
	if path := filepath.Join(directory, file); fileExists(path) {
		return path, nil
	}
	// Source checkouts stage independently built extensions in their own packages.
	for i := 0; i < 8; i++ {
		path := filepath.Join(directory, "extensions", name, "lib", StageTarget(), file)
		if fileExists(path) {
			return path, nil
		}
		parent := filepath.Dir(directory)
		if parent == directory {
			break
		}
		directory = parent
	}
	return "", fmt.Errorf("QuickGUI extension %s is not bundled; rebuild with quickgui dev/build or stage the extension in QUICKGUI_EXTENSION_DIR", name)
}
