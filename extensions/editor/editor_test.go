package editor

import (
	"bytes"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
	"testing"
)

func TestEditorPropertiesAndEventsUseGenericBoundary(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		language := reactive.NewSignal("rust")
		var changed string
		element := Editor(EditorProps{Value: "fn main() {}", Language: language.Read, TabSize: 2, OnChange: func(value string) { changed = value }})
		if element.Tag != protocol.TagExtension {
			t.Fatalf("tag = %d", element.Tag)
		}
		before := len(element.Pending.Body())
		language.Write("go")
		update := element.Pending.Body()[before:]
		if !bytes.Contains(update, []byte(`"language":"go"`)) || !bytes.Contains(update, []byte(`"tabSize":2`)) {
			t.Fatal("reactive properties were not sent to the extension")
		}
		host := &native.NodeHost{Nodes: map[uint32]*native.Node{element.ID: element.Node}}
		native.DispatchEvent(
			host,
			protocol.EventComponentChange,
			element.ID,
			`{"kind":"input","value":"package main"}`,
			true,
		)
		if changed != "package main" {
			t.Fatalf("input event = %q", changed)
		}
		return struct{}{}
	})
}
func TestDiffAndCodeBlockKeepPackageOwnedProps(t *testing.T) {
	diff := DiffView(DiffViewProps{OldText: "old", NewText: "new", Options: DiffOptions{Layout: DiffUnified}, Theme: DiffTheme{AddedColor: "#44cc66"}})
	if diff.Tag != protocol.TagExtension || !bytes.Contains(diff.Pending.Body(), []byte(`"layout":"unified"`)) {
		t.Fatal("diff did not declare package-owned options")
	}
	block := CodeBlock(CodeBlockProps{Value: "let answer = 42;", LineNumbers: true})
	if block.Tag != protocol.TagExtension || !bytes.Contains(block.Pending.Body(), []byte(`"lineNumbers":true`)) {
		t.Fatal("code block did not declare package-owned options")
	}
}

func TestAppearanceIsExplicitInComponentProperties(t *testing.T) {
	block := CodeBlock(CodeBlockProps{Value: "code", ContentPadding: ContentPadding{Left: 12, Vertical: 8}, GutterPadding: GutterPadding{Right: 10}})
	for _, property := range []string{`"contentPadding":{"left":12`, `"vertical":8`, `"gutterPadding":{"left":null,"right":10}`} {
		if !bytes.Contains(block.Pending.Body(), []byte(property)) {
			t.Fatalf("missing appearance property %s", property)
		}
	}
	diff := DiffView(DiffViewProps{OldText: "a", NewText: "b", Presentation: DiffPresentation{HeaderPadding: 12}})
	if !bytes.Contains(diff.Pending.Body(), []byte(`"headerPadding":12`)) {
		t.Fatal("missing caller-supplied header padding")
	}
}

func TestLanguagePackRejectsRelativePathsBeforeInvokingTheHost(t *testing.T) {
	calls := 0
	LoadLanguagePack("languages/lua/language.json", func(names []string, err error) {
		calls++
		if len(names) != 0 || err == nil {
			t.Fatal("relative path was accepted")
		}
	})
	if calls != 1 {
		t.Fatal("callback was not delivered exactly once")
	}
}
