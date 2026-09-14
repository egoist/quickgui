package ui

import (
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

// CreateSignal returns a reactive getter and setter for component state.
func CreateSignal[T any](value T, options ...reactive.SignalOptions) (reactive.Accessor[T], reactive.Setter[T]) {
	return reactive.CreateSignal(value, options...)
}

func CreateMemo[T any](compute func() T, options ...reactive.SignalOptions) reactive.Accessor[T] {
	return reactive.CreateMemo(compute, options...)
}

func CreateEffect(fn func()) { reactive.CreateEffect(fn) }

func CreateRenderEffect(fn func()) { reactive.CreateRenderEffect(fn) }

func Batch(fn func()) { reactive.Batch(fn) }

func Untrack[T any](fn func() T) T { return reactive.Untrack(fn) }

func Flush() { reactive.Flush() }

func OnCleanup(fn func()) { reactive.OnCleanup(fn) }

// NativeElement applies ordinary children, styles, options, and reactive ownership
// to a native node kind supplied by an extension package.
func NativeElement(tag uint8, arguments ...any) *Element {
	return newElement(tag, arguments)
}

// View constructs an empty retained container. Append content with Child or
// Children and configure it with fluent properties, events, and styles.
func View() *Element {
	return newElement(protocol.TagView, nil)
}

func Text(children ...any) *Element {
	return newElement(protocol.TagView, children)
}

// Button constructs an empty button. Add content with Child or Children.
func Button() *Element {
	return newElement(protocol.TagButton, nil)
}

// Input constructs an empty editor. Configure its text with Value.
func Input() *Element {
	return newElement(protocol.TagInput, nil)
}

func TextArea() *Element {
	return Input().Multiline(true)
}

func Image() *Element {
	return newElement(protocol.TagImage, nil)
}

// SVG renders inline SVG markup supplied through Value using the Rust renderer.
func SVG() *Element {
	return newElement(protocol.TagSvg, nil)
}

// Shader paints WGSL supplied through Value, with up to sixteen parameter floats.
func Shader() *Element {
	return newElement(protocol.TagShader, nil)
}

// VirtualList lays out and paints the visible children using the core's virtual list.
func VirtualList() *Element {
	return newElement(protocol.TagVirtualList, nil)
}
