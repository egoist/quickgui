package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// CreateSignal is re-exported so application code can import only ui, as with @quickgui/ui.
func CreateSignal[T any](value T, options ...reactive.SignalOptions) (reactive.Accessor[T], reactive.Setter[T]) {
	return reactive.CreateSignal(value, options...)
}

func CreateMemo[T any](compute func() T, options ...reactive.SignalOptions) reactive.Accessor[T] {
	return reactive.CreateMemo(compute, options...)
}

func CreateEffect(fn func()) { reactive.CreateEffect(fn) }

func Batch(fn func()) { reactive.Batch(fn) }

func OnCleanup(fn func()) { reactive.OnCleanup(fn) }

func View(props Props) *native.Node {
	node := native.CreateElement(protocol.TagView)
	applyProps(node, props)
	return node
}

func Text(props Props) *native.Node {
	node := native.CreateElement(protocol.TagView)
	applyProps(node, props)
	return node
}

func Button(props Props) *native.Node {
	node := native.CreateElement(protocol.TagButton)
	applyProps(node, props)
	return node
}

func Input(props Props) *native.Node {
	node := native.CreateElement(protocol.TagInput)
	applyProps(node, props)
	return node
}

func TextArea(props Props) *native.Node {
	props.Multiline = true
	return Input(props)
}

func Markdown(props Props) *native.Node {
	node := native.CreateElement(protocol.TagMarkdown)
	applyProps(node, props)
	return node
}

func Image(props Props) *native.Node {
	node := native.CreateElement(protocol.TagImage)
	applyProps(node, props)
	return node
}
