package ui

import (
	"encoding/json"
	"reflect"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
)

// ComponentReference identifies a registered component and its initial properties.
type ComponentReference struct {
	Package string `json:"package"`
	Name    string `json:"name"`
	Props   any    `json:"props"`
}

// ExtensionComponent mounts any registered native component. Properties stay owned by the
// extension package; accessors in a property object are tracked by the normal retained binding.
func ExtensionComponent(packageName, component string, properties any, props Props) *Element {
	element := NativeElement(protocol.TagExtension, props)
	native.SetString(element.Node, protocol.ExtensionPackage, packageName)
	native.SetString(element.Node, protocol.ExtensionComponent, component)
	element.Bind(func() {
		value := resolveExtensionValue(reflect.ValueOf(properties), 0)
		encoded, err := json.Marshal(value)
		if err != nil {
			panic(err)
		}
		if len(encoded) > 12*1024*1024 {
			panic("QuickGUI component properties exceed their byte limit")
		}
		native.SetString(element.Node, protocol.ExtensionProps, string(encoded))
	})
	return element
}

func resolveExtensionValue(value reflect.Value, depth int) any {
	if depth > 64 {
		panic("QuickGUI component properties are nested too deeply")
	}
	if !value.IsValid() {
		return nil
	}
	switch value.Kind() {
	case reflect.Interface, reflect.Pointer:
		if value.IsNil() {
			return nil
		}
		return resolveExtensionValue(value.Elem(), depth+1)
	case reflect.Func:
		if value.IsNil() {
			return nil
		}
		if value.Type().NumIn() != 0 || value.Type().NumOut() != 1 {
			panic("QuickGUI property accessors take no arguments and return one value")
		}
		return resolveExtensionValue(value.Call(nil)[0], depth+1)
	case reflect.Map:
		result := make(map[string]any, value.Len())
		for _, key := range value.MapKeys() {
			if key.Kind() != reflect.String {
				panic("QuickGUI component property keys must be strings")
			}
			result[key.String()] = resolveExtensionValue(value.MapIndex(key), depth+1)
		}
		return result
	case reflect.Array, reflect.Slice:
		result := make([]any, value.Len())
		for i := range result {
			result[i] = resolveExtensionValue(value.Index(i), depth+1)
		}
		return result
	default:
		return value.Interface()
	}
}
