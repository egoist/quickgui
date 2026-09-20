package terminal

import (
	"encoding/json"
	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

// Palette contains the sixteen ANSI colors in normal, then bright order.
type Palette = [16]string

// StatusDetails is a snapshot reported by the native PTY. Agent fields
// describe a detected foreground process, not the program requested at launch.
type StatusDetails struct {
	Status           string `json:"status"`
	Title            string `json:"title"`
	WorkingDirectory string `json:"workingDirectory"`
	ProcessID        int    `json:"processId,omitempty"`
	ExitCode         *int   `json:"exitCode,omitempty"`
	Signal           string `json:"signal,omitempty"`
	Message          string `json:"message,omitempty"`
	Agent            string `json:"agent,omitempty"`
	AgentStatus      string `json:"agentStatus,omitempty"`
	AgentProcessID   int    `json:"agentProcessId,omitempty"`
}

func StatusFromEvent(event *native.Event) *StatusDetails {
	if event == nil {
		return nil
	}
	var status StatusDetails
	if json.Unmarshal([]byte(event.Value), &status) != nil {
		return nil
	}
	return &status
}

// Props configure the retained terminal view and its optional PTY backend.
type Props struct {
	ui.Props
	Program          string
	Args             []string
	WorkingDirectory string
	Scrollback       int
	Environment      map[string]string
	Palette          any // Palette or an accessor returning one.
	CursorColor      any
	PaddingColor     string
	FontThicken      bool
	OnStatus         func(*native.Event)
}

func View(props Props, children ...any) *native.Node {
	previous := props.Props.OnComponentChange
	props.Props.OnComponentChange = func(event *native.Event) {
		var payload struct {
			Kind  string
			Value json.RawMessage
		}
		if json.Unmarshal([]byte(event.Value), &payload) == nil && payload.Kind == "status" && props.OnStatus != nil {
			forwarded := *event
			forwarded.Value = string(payload.Value)
			props.OnStatus(&forwarded)
		}
		if previous != nil {
			previous(event)
		}
	}
	return ui.ExtensionComponent(
		"terminal",
		"terminal",
		map[string]any{
			"program": props.Program, "arguments": props.Args, "workingDirectory": props.WorkingDirectory,
			"scrollback": props.Scrollback, "environment": props.Environment, "palette": props.Palette,
			"cursorColor": props.CursorColor, "paddingColor": props.PaddingColor, "fontThicken": props.FontThicken,
		},
		props.Props,
	).
		Children(children...).Node
}
func init() { host.RequireExtension("terminal", "0.1.6") }
