package terminal

import (
	"bytes"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestTerminalThemeUpdatesWithoutRestartingSession(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		var initial Palette
		for i := range initial {
			initial[i] = "#112233"
		}
		palette := reactive.NewSignal(initial)
		cursor := reactive.NewSignal("#123456")
		node := View(Props{Program: "/bin/sh", Palette: palette.Read, CursorColor: cursor.Read, FontThicken: true, PaddingColor: "extend"})
		offset := len(node.Pending.Body())
		next := initial
		next[3] = "#abcdef"
		palette.Write(next)
		cursor.Write("#654321")
		update := node.Pending.Body()[offset:]
		if node.Tag != protocol.TagExtension || !bytes.Contains(update, []byte(`"#abcdef"`)) || !bytes.Contains(update, []byte(`"cursorColor":"#654321"`)) {
			t.Fatal("theme update changed PTY config or encoded an incompatible palette")
		}
		return struct{}{}
	})
}
func TestTerminalStatusDecodesNativeAgentLifecycle(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		var status *StatusDetails
		node := View(Props{OnStatus: func(event *native.Event) { status = StatusFromEvent(event) }})
		host := &native.NodeHost{Nodes: map[uint32]*native.Node{node.ID: node}}
		native.DispatchEvent(
			host,
			protocol.EventComponentChange,
			node.ID,
			`{"kind":"status","value":{"status":"running","title":"Review","workingDirectory":"/work","processId":10,"agent":"codex","agentStatus":"blocked","agentProcessId":12}}`,
			true,
		)
		if status == nil || status.Agent != "codex" || status.AgentStatus != "blocked" || status.AgentProcessID != 12 {
			t.Fatalf("native status lost: %+v", status)
		}
		native.DispatchEvent(
			host,
			protocol.EventComponentChange,
			node.ID,
			`{"kind":"status","value":{"status":"exited","exitCode":0}}`,
			true,
		)
		if status == nil || status.Agent != "" || status.ExitCode == nil || *status.ExitCode != 0 {
			t.Fatal("exit did not clear agent metadata")
		}
		return struct{}{}
	})
}
