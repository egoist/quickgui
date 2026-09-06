package host

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sync"
)

var loadOnce sync.Once
var loadErr error

// Load opens the prebuilt host shared library and binds the C ABI. It is idempotent.
func Load() error {
	loadOnce.Do(func() {
		path, err := findLibrary()
		if err != nil {
			loadErr = err
			return
		}
		api, err := bindLibrary(path)
		if err != nil {
			loadErr = err
			return
		}
		Current = api
	})
	return loadErr
}

// LibraryName is the staged shared-library file for this GOOS.
func LibraryName() string {
	switch runtime.GOOS {
	case "darwin":
		return "libquickgui_host.dylib"
	case "windows":
		return "quickgui_host.dll"
	default:
		return "libquickgui_host.so"
	}
}

// StageTarget is the packages/native/lib directory name for this platform.
func StageTarget() string {
	arch := runtime.GOARCH
	if arch == "amd64" {
		arch = "x64"
	}
	osName := runtime.GOOS
	if osName == "darwin" {
		osName = "darwin"
	} else if osName == "windows" {
		osName = "windows"
	} else {
		osName = "linux"
	}
	return osName + "-" + arch
}

func findLibrary() (string, error) {
	if path := os.Getenv("QUICKGUI_HOST_LIB"); path != "" {
		if fileExists(path) {
			return path, nil
		}
		return "", fmt.Errorf("QUICKGUI_HOST_LIB=%s does not exist", path)
	}
	name := LibraryName()
	var roots []string
	if exe, err := os.Executable(); err == nil {
		roots = append(roots, filepath.Dir(exe))
	}
	if cwd, err := os.Getwd(); err == nil {
		roots = append(roots, cwd)
	}
	seen := map[string]struct{}{}
	for _, root := range roots {
		dir := root
		for i := 0; i < 8; i++ {
			if _, ok := seen[dir]; ok {
				break
			}
			seen[dir] = struct{}{}
			candidates := []string{
				filepath.Join(dir, name),
				filepath.Join(dir, "lib", name),
				filepath.Join(dir, "packages", "native", "lib", StageTarget(), name),
			}
			for _, candidate := range candidates {
				if fileExists(candidate) {
					return candidate, nil
				}
			}
			parent := filepath.Dir(dir)
			if parent == dir {
				break
			}
			dir = parent
		}
	}
	return "", fmt.Errorf(
		"QuickGUI host library %s not found; set QUICKGUI_HOST_LIB or run bun run build:native",
		name,
	)
}

func fileExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}
