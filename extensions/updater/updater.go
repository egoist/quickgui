// Package updater adds optional Sparkle-compatible application updates. Importing
// it makes the CLI bundle @quickgui/extension-updater and Sparkle on macOS.
package updater

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/native"
)

func init() {
	host.RequireExtension("updater")
	native.App.OnReady(func() {
		// The helper owns the private staging directory. Acknowledge the new Go/native app
		// reaching readiness without depending on whether its UI starts an updater session.
		path := os.Getenv("QUICKGUI_UPDATE_READY_FILE")
		if path == "" {
			return
		}
		_ = os.Unsetenv("QUICKGUI_UPDATE_READY_FILE")
		if filepath.IsAbs(path) && filepath.Base(path) == "application-ready" && len(path) < 4096 {
			go func() {
				file, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
				if err == nil {
					_, _ = file.WriteString("ready\n")
					_ = file.Close()
				}
			}()
		}
	})
}

// Options override the updater defaults embedded by quickgui.toml. Development
// builds stay disabled unless AllowDevelopment is explicitly set for update testing.
// CurrentVersion and Identifier apply to Windows/Linux; Sparkle uses the app bundle.
type Options struct {
	FeedURL          string `json:"feedUrl,omitempty"`
	PublicKey        string `json:"publicKey,omitempty"`
	CurrentVersion   string `json:"currentVersion,omitempty"`
	Identifier       string `json:"identifier,omitempty"`
	AllowDevelopment bool   `json:"allowDevelopment,omitempty"`
}

type Status string

const (
	Idle        Status = "idle"
	Checking    Status = "checking"
	Available   Status = "available"
	Downloading Status = "downloading"
	Installing  Status = "installing"
	Disabled    Status = "disabled"
)

// Event carries state changes and explicit check results. Automatic checks never
// open a window on Windows/Linux; use Available to offer an Install action.
// QuitRequired means the validated installer is waiting for the normal app quit.
// Call native.App.Quit(false, nil) when unsaved work is safe; a cancelled quit cancels the
// handoff after its timeout without replacing the running app.
type Event struct {
	Kind            string `json:"kind"`
	Status          Status `json:"status"`
	Version         string `json:"version,omitempty"`
	Notes           string `json:"notes,omitempty"`
	DownloadedBytes uint64 `json:"downloadedBytes,omitempty"`
	TotalBytes      uint64 `json:"totalBytes,omitempty"`
	AutomaticChecks bool   `json:"automaticChecks"`
	Error           string `json:"error,omitempty"`
	QuitRequired    bool   `json:"quitRequired,omitempty"`
}

// Updater is app-wide, not window-owned. Start at most one, after app readiness.
type Updater struct {
	session *native.ExtensionSession
	state   Event
}

var buildMetadata string // Injected only when this package is imported.

// Start initializes native updating. Both callbacks run on the UI goroutine.
// Defaults come from [updates] in quickgui.toml, with per-app preferences retained.
func Start(options Options, changed func(Event), ready func(error)) *Updater {
	u := &Updater{state: Event{Status: Idle}}
	config := map[string]any{}
	if buildMetadata != "" {
		data, err := base64.RawURLEncoding.DecodeString(buildMetadata)
		if err == nil {
			err = json.Unmarshal(data, &config)
		}
		if err != nil {
			if ready != nil {
				ready(fmt.Errorf("invalid updater build metadata: %w", err))
			}
			return u
		}
	}
	data, _ := json.Marshal(options)
	var overrides map[string]any
	_ = json.Unmarshal(data, &overrides)
	for key, value := range overrides {
		config[key] = value
	}
	u.session = native.OpenExtension(
		"updater",
		config,
		func(raw string) {
			var event Event
			if err := json.Unmarshal([]byte(raw), &event); err != nil {
				return
			}
			u.state = event
			if changed != nil {
				changed(event)
			}
		},
		ready,
	)
	return u
}

// State returns the latest cached event without native calls or polling.
func (u *Updater) State() Event { return u.state }

// Check runs a user-initiated check. macOS presents Sparkle's standard UI.
func (u *Updater) Check(done func(error)) { u.command("check", true, done) }

// Install accepts the offered update. macOS uses Sparkle; other platforms verify
// and stage it, then emit QuitRequired after their handoff helper is ready.
func (u *Updater) Install(done func(error)) { u.command("install", nil, done) }

// SetAutomaticChecks persists this preference. Enabling it starts a quiet check.
func (u *Updater) SetAutomaticChecks(enabled bool, done func(error)) {
	u.command("automatic", enabled, done)
}
func (u *Updater) command(method string, value any, done func(error)) {
	if u == nil || u.session == nil {
		if done != nil {
			done(fmt.Errorf("updater is not initialized"))
		}
		return
	}
	u.session.Command(method, value, done)
}
func (u *Updater) Close() {
	if u != nil && u.session != nil {
		u.session.Close()
	}
}
