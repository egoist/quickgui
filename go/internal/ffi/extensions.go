package ffi

import (
	"fmt"

	"github.com/ebitengine/purego"
)

// LoadExtension keeps the image loaded for process lifetime: its session workers
// and the Rust core retain function pointers. Sessions release their own resources.
func (library *Library) LoadExtension(name, path string, version ...string) (err error) {
	if len(version) > 1 {
		return fmt.Errorf("QuickGUI extension requires one exact version")
	}
	handle, err := openLibrary(path)
	if err != nil {
		return fmt.Errorf("load QuickGUI extension %s: %w", name, err)
	}
	defer func() {
		if failure := recover(); failure != nil {
			err = fmt.Errorf("incompatible QuickGUI extension %s: %v", name, failure)
		}
	}()
	var descriptor func() uintptr
	purego.RegisterLibFunc(&descriptor, handle, "quickgui_extension_v1")
	value := descriptor()
	if value == 0 {
		return fmt.Errorf("QuickGUI extension %s returned no descriptor", name)
	}
	bytes := []byte(name)
	var status int32
	if len(version) > 0 && version[0] != "" {
		var register func(uintptr, []byte, uintptr, []byte, uintptr) int32
		purego.RegisterLibFunc(&register, library.handle, "quickgui_register_extension_versioned")
		expected := []byte(version[0])
		status = register(value, bytes, uintptr(len(bytes)), expected, uintptr(len(expected)))
	} else {
		status = library.RegisterExtension(value, bytes, uintptr(len(bytes)))
	}
	if status != 0 {
		return fmt.Errorf("QuickGUI extension %s at %s (requested version %q) failed name, version, ABI, or duplicate-provider validation", name, path, version)
	}
	return nil
}
