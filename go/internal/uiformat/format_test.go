package uiformat

import (
	"bytes"
	"go/format"
	"go/scanner"
	"go/token"
	"reflect"
	"strings"
	"testing"
)

func TestWrapsFluentDeclarations(t *testing.T) {
	source := []byte(`package example
import ui "github.com/egoist/quickgui/go/ui"
func root() {
ui.View().Child(ui.Text("hello")).Child(ui.Input().Value("xxx")).Flex().Style(ui.Style().Merge(ui.Style().PaddingLeft(20), ui.Style().TextAlign("center"), ui.Style().Bg("#112233")))
ui.Button().Child("Toggle").When(func() bool { return selected() }, ui.Style().BackgroundColor("blue"), ui.Style().TextColor("white"))
}`)
	result := string(checkFormat(t, source))
	for _, want := range []string{
		"ui.View().\n",
		"Child(ui.Text(\"hello\")).\n",
		"Flex().\n",
		"ui.Style().\n",
		"Merge(\n",
		"ui.Style().PaddingLeft(20),\n",
		"ui.Style().TextAlign(\"center\"),\n",
		"ui.Button().\n",
		"When(\n",
		"ui.Style().BackgroundColor(\"blue\"),\n",
	} {
		if !strings.Contains(result, want) {
			t.Errorf("missing %q in:\n%s", want, result)
		}
	}
}

func TestWrapsFluentMethodChains(t *testing.T) {
	source := []byte(`package example
import ui "github.com/egoist/quickgui/go/ui"
func buttonStyle() ui.StyleBuilder {
return ui.Style().Display("flex").AlignItems("center").JustifyContent("center").Height(34).FlexShrink(0).PaddingLeft(12).PaddingRight(12).BorderRadius(7).BackgroundColor("#253855").TextColor("#e2e8f0").UserSelect("none").AppRegion("no-drag").Cursor("default").FontSize(13)
}
func root() *ui.Element {
return ui.View().Children(ui.Text("Conversations").FontSize(18).FontWeight(700), ui.Button().Child("New chat")).Display("flex").FlexDirection("column").Width(248).Height("100%").FlexShrink(0).Padding(16)
}
func short() *ui.Element {
return ui.Text("Conversations").FontSize(18).FontWeight(700)
}
`)
	result := string(checkFormat(t, source))
	for _, want := range []string{
		"return ui.Style().\n",
		"Display(\"flex\").\n",
		"AlignItems(\"center\").\n",
		"FontSize(13)\n",
		"return ui.View().\n",
		"Children(\n",
		"ui.Text(\"Conversations\").FontSize(18).FontWeight(700),\n",
		"Display(\"flex\").\n",
		"FlexDirection(\"column\").\n",
		"return ui.Text(\"Conversations\").FontSize(18).FontWeight(700)\n",
	} {
		if !strings.Contains(result, want) {
			t.Errorf("missing %q in:\n%s", want, result)
		}
	}
}

func TestWrapsInstanceFluentChains(t *testing.T) {
	source := []byte(`package example
import (
 native "github.com/egoist/quickgui/go/native"
 ui "github.com/egoist/quickgui/go/ui"
)
func root() *ui.Element {
popover := ui.NewPopover()
return popover.Root().Children(popover.Trigger().Child("Open"), popover.Content().Child("Hello")).Display("flex").Width("100%").Height("100%").Padding(20).Gap(12)
}
func nodes() *native.Node {
popover := ui.NewPopover()
return ui.Fragment([]*native.Node{popover.Trigger().Child("Open").NativeNode(), popover.Content().Child("Hello").NativeNode()})
}
`)
	result := string(checkFormat(t, source))
	for _, want := range []string{
		"return popover.Root().\n",
		"Children(\n",
		"Display(\"flex\").\n",
		"Width(\"100%\").\n",
		"popover.Trigger().Child(\"Open\"),\n",
		"[]*native.Node{\n",
		"popover.Trigger().Child(\"Open\").NativeNode(),\n",
	} {
		if !strings.Contains(result, want) {
			t.Errorf("missing %q in:\n%s", want, result)
		}
	}
}

func TestWrapsCallbacksWithoutChangingTokens(t *testing.T) {
	source := []byte(`package example
import gui "github.com/egoist/quickgui/go/ui"
func root() {
gui.Show(func() bool { return selected() }, func() {
gui.Button(gui.OnClick(func() { increment() }), gui.Style().BackgroundColor("#ccc"), "Increment")
}, func() { fallback() })
}`)
	result := checkFormat(t, source)
	for _, want := range []string{
		"gui.Show(\n",
		"func() bool { return selected() },\n",
		"gui.Button(\n",
		"gui.OnClick(func() { increment() }),\n",
		"},\n\t\tfunc() { fallback() },\n\t)",
	} {
		if !strings.Contains(string(result), want) {
			t.Errorf("missing %q in:\n%s", want, result)
		}
	}
}

func TestPreservesCommentsRawStringsAndVariadics(t *testing.T) {
	source := []byte("package example\n" +
		"import ui \"github.com/egoist/quickgui/go/ui\"\n" +
		"func root() {\n" +
		"ui.Text(/* leading, comma */ ui.Style().Padding(20), // spacing\n" +
		"/* next argument */ ui.Style().BackgroundColor(\"a long value which should wrap the containing UI declaration\"), `raw\n  text, stays exact`, args... /* final, comment */)\n" +
		"}")
	result := checkFormat(t, source)
	if !bytes.Contains(result, []byte("args..., /* final, comment */\n")) {
		t.Fatalf("variadic comma or comment was lost:\n%s", result)
	}
}

func TestWrapsPropertiesNestedCallsAndGenericCalls(t *testing.T) {
	source := []byte(`package example
import (
 gui "github.com/egoist/quickgui/go/ui"
 native "github.com/egoist/quickgui/go/native"
)
func root() {
gui.Text(gui.Props{Style: gui.Style().Flex().Width("100%").Height("100%").Bg("#ccc"), OnClick: func(event *native.Event) { handle(event) }}, func() { child() })
gui.For[int](values, func(item int, index func() int) *native.Node { return row(item, index) }, nil, nil)
}`)
	result := checkFormat(t, source)
	if !bytes.Contains(result, []byte("gui.Props{\n")) || !bytes.Contains(result, []byte("gui.For[int](\n")) {
		t.Fatalf("declarations were not wrapped:\n%s", result)
	}
}

func TestLeavesShortCallsAndOtherPackagesToGofmt(t *testing.T) {
	source := []byte(`package example
import (
 "github.com/egoist/quickgui/go/ui"
 other "example.com/ui"
)
func root() {
ui.Text(ui.Style().FontSize(14), "Hello")
other.View(other.Padding(20), other.BackgroundColor("a long value that is not part of a QuickGUI UI declaration"), "Hello")
other.View().Child("Hello").Style(other.Padding(20)).Width("100%").Height("100%").Padding(16)
}`)
	want, err := format.Source(source)
	if err != nil {
		t.Fatal(err)
	}
	if got := checkFormat(t, source); !bytes.Equal(got, want) {
		t.Fatalf("unexpected wrapping:\n%s", got)
	}
}

func TestGeneratedFilesAndSyntaxErrors(t *testing.T) {
	generated := []byte("// Code generated by fixture. DO NOT EDIT.\npackage example\nfunc f( ){}\n")
	got, err := Source(generated, DefaultWidth)
	if err != nil || !bytes.Equal(got, generated) {
		t.Fatalf("generated source changed: %s, %v", got, err)
	}
	if _, err := Source([]byte("package incomplete\nfunc"), DefaultWidth); err == nil {
		t.Fatal("invalid Go must be rejected")
	}
	if _, err := Source([]byte("package example"), 0); err == nil {
		t.Fatal("invalid width must be rejected")
	}
}

func checkFormat(t *testing.T, source []byte) []byte {
	t.Helper()
	result, err := Source(source, DefaultWidth)
	if err != nil {
		t.Fatal(err)
	}
	again, err := Source(result, DefaultWidth)
	if err != nil || !bytes.Equal(result, again) {
		t.Fatalf("formatting is not idempotent: %v\n%s\n%s", err, result, again)
	}
	standard, err := format.Source(result)
	if err != nil || !bytes.Equal(result, standard) {
		t.Fatalf("gofmt changed the custom layout: %v\n%s", err, result)
	}
	baseline, err := format.Source(source)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(tokens(baseline), tokens(result)) {
		t.Fatalf("formatting changed Go tokens or comments:\n%s", result)
	}
	return result
}

func tokens(source []byte) []string {
	fs := token.NewFileSet()
	var scan scanner.Scanner
	scan.Init(fs.AddFile("", -1, len(source)), source, nil, scanner.ScanComments)
	var result []string
	for {
		_, kind, literal := scan.Scan()
		if kind == token.EOF {
			return result
		}
		if kind != token.COMMA && kind != token.SEMICOLON {
			result = append(result, kind.String()+":"+literal)
		}
	}
}
