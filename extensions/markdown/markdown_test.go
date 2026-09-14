package markdown

import (
	"bytes"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
	"testing"
)

func TestMarkdownPropertiesStayReactive(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		source := reactive.NewSignal("# Hello")
		element := View(Props{Value: source.Read, Streaming: true, CodeBlockComponent: ui.ComponentReference{
			Package: "acme",
			Name:    "code",
			Props:   map[string]any{},
		}, CodeBlockMaxHeight: 280})
		if element.Tag != protocol.TagExtension {
			t.Fatalf("tag = %d", element.Tag)
		}
		if !bytes.Contains(element.Pending.Body(), []byte(`"package":"acme"`)) {
			t.Fatal("custom component reference was lost")
		}
		before := len(element.Pending.Body())
		source.Write("# Updated")
		if !bytes.Contains(element.Pending.Body()[before:], []byte(`"value":"# Updated"`)) {
			t.Fatal("source did not update")
		}
		return struct{}{}
	})
}
