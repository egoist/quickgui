package main

import (
	"context"
	"log"
	"os"
	"path/filepath"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	var writer *stateWriter
	opening := false
	err := native.Run(func() {
		openWindow := func() {
			if opening {
				return
			}
			opening = true
			native.App.GetPaths(func(paths *native.AppPaths, err error) {
				home, _ := os.UserHomeDir()
				config, _ := os.UserConfigDir()
				data := config
				if paths != nil {
					if paths.HomeDir != "" {
						home = paths.HomeDir
					}
					if paths.DataDir != "" {
						data = paths.DataDir
					} else if paths.ConfigDir != "" {
						data = paths.ConfigDir
					}
				}
				if home == "" {
					home, _ = os.Getwd()
				}
				file := filepath.Join(data, "herdr-gui-state.json")
				if writer == nil {
					writer = newStateWriter(file)
				}
				ui.Async(
					func(context.Context) (savedState, error) {
						saved, readErr := readSavedState(file)
						if os.IsNotExist(readErr) {
							saved, _ = readSavedState(filepath.Join(config, "quickgui-herdr-go", "state.json"))
						}
						return saved, nil
					},
					func(saved savedState, _ error) {
						opening = false
						preference := saved.Appearance
						if preference != "light" && preference != "dark" {
							preference = "system"
						}
						window := native.NewWindow(native.WindowOptions{
							Title:         "Herdr GUI",
							Width:         1220,
							Height:        780,
							MinimumWidth:  860,
							MinimumHeight: 560,
							Background:    themeFor(preference).App,
							Appearance:    preference,
							TitleBarStyle: "hiddenInset",
							TrafficLightPosition: &native.Point{
								X: 15,
								Y: 14,
							},
							Component: func() *ui.Element {
								m := newModel(home, saved)
								m.Window = native.CurrentWindow()
								m.persist = writer.save
								m.connect()
								return appView(m)
							},
						})
						window.On(native.WindowReadyToShow, func(native.WindowEvent) { window.Focus() })
					},
				)
			})
		}
		native.App.OnReopen(func(e native.ReopenEvent) {
			if !e.HasVisibleWindows {
				openWindow()
			}
		})
		openWindow()
	})
	if writer != nil {
		writer.close()
	}
	if err != nil {
		log.Fatal(err)
	}
}

func (m *model) connect() {
	refresh := func() {
		m.Window.GetState(func(state native.WindowState, err error) {
			if err != nil || m.disposed {
				return
			}
			m.sectionHeight = max(state.ViewportHeight-40, 320)
			if state.Appearance == "light" || state.Appearance == "dark" {
				m.Appearance.Write(state.Appearance)
			}
		})
	}
	stopAppearance := m.Window.On(native.WindowAppearance, func(e native.WindowEvent) {
		if e.Appearance == "light" || e.Appearance == "dark" {
			m.Appearance.Write(e.Appearance)
		}
	})
	stopResize := m.Window.On(native.WindowResize, func(native.WindowEvent) { refresh() })
	stopReady := m.Window.On(native.WindowReadyToShow, func(native.WindowEvent) { refresh(); m.focusPane(m.ActivePaneID.Peek()) })
	stopFocus := m.Window.On(native.WindowFocus, func(native.WindowEvent) { m.installMenu() })
	reactive.OnCleanup(func() {
		m.save()
		m.disposed = true
		stopAppearance()
		stopResize()
		stopReady()
		stopFocus()
	})
	type catalog struct {
		environment map[string]string
		launchers   []launcher
	}
	ui.Async(
		func(ctx context.Context) (catalog, error) {
			env := captureShellEnvironment(ctx)
			return catalog{env, resolveLaunchers(env, m.Home)}, nil
		},
		func(c catalog, _ error) {
			m.Environment = c.environment
			m.Launchers.Write(c.launchers)
			for _, l := range c.launchers {
				if l.installed() {
					m.SelectedLauncherID.Write(l.ID)
					break
				}
			}
			m.CatalogLoading.Write(false)
		},
	)
	m.installMenu()
}
func (m *model) addSpace() {
	if m.addingSpace || m.disposed {
		return
	}
	m.addingSpace = true
	native.ShowOpenDialog(
		native.OpenDialogOptions{
			Window:      m.Window,
			Title:       "Add a space",
			DefaultPath: m.activeSpace().Path,
			Properties:  []string{"openDirectory"},
		},
		func(result native.OpenDialogResult, err error) {
			m.addingSpace = false
			if m.disposed {
				return
			}
			if err != nil {
				m.Error.Write(err.Error())
				return
			}
			if !result.Canceled && len(result.FilePaths) > 0 {
				m.addSpacePath(result.FilePaths[0])
			}
		},
	)
}
func (m *model) setTheme(preference string) {
	m.Preference.Write(preference)
	m.Window.Action("set-appearance", preference)
	if preference != "system" {
		m.Appearance.Write(preference)
	}
	m.installMenu()
	m.save()
}
func (m *model) toggleTheme() { m.setTheme(choose(m.Appearance.Peek() == "dark", "light", "dark")) }
func (m *model) handleSidebarPointer(event *native.Event) {
	p := ui.CapturedPointerFromEvent(event)
	if p == nil || p.Button != "left" {
		return
	}
	switch p.Phase {
	case "down":
		m.sidebarStart = m.SidebarWidth.Peek()
	case "move":
		m.SidebarWidth.Write(clamp(m.sidebarStart+p.Position.X-p.Origin.X, 196, 380))
	default:
		m.save()
	}
}
func (m *model) handleSectionPointer(event *native.Event) {
	p := ui.CapturedPointerFromEvent(event)
	if p == nil || p.Button != "left" {
		return
	}
	switch p.Phase {
	case "down":
		m.sectionStart = m.SectionRatio.Peek()
	case "move":
		m.SectionRatio.Write(clamp(m.sectionStart+(p.Position.Y-p.Origin.Y)/m.sectionHeight, .25, .75))
	default:
		m.save()
	}
}
func (m *model) installMenu() {
	action := func(fn func()) func() {
		return func() {
			if !m.disposed {
				fn()
			}
		}
	}
	native.SetApplicationMenu([]native.MenuDefinition{
		{Label: "Herdr GUI", Items: []native.MenuItem{
			{Label: "About Herdr GUI", Enabled: ptr(false)},
			{Type: "separator"},
			{Label: "Hide Herdr GUI", Role: "hide-application", Accelerator: "CmdOrCtrl+H"},
			{Label: "Hide Others", Role: "hide-other-applications", Accelerator: "CmdOrCtrl+Alt+H"},
			{Label: "Show All", Role: "show-all-applications"},
			{Type: "separator"},
			{Label: "Quit Herdr GUI", Role: "quit", Accelerator: "CmdOrCtrl+Q"},
		}},
		{Label: "File", Items: []native.MenuItem{
			{Label: "New Agent…", Accelerator: "CmdOrCtrl+N", Click: action(m.openAgentSheet)},
			{Label: "New Tab", Accelerator: "CmdOrCtrl+T", Click: action(func() { m.newTerminal(m.activeSpace()) })},
			{Label: "Add Space…", Accelerator: "CmdOrCtrl+O", Click: action(m.addSpace)},
			{Type: "separator"},
			{Label: "Split Right", Accelerator: "CmdOrCtrl+D", Click: action(func() { m.splitTerminal("horizontal") })},
			{Label: "Split Down", Accelerator: "CmdOrCtrl+Shift+D", Click: action(func() { m.splitTerminal("vertical") })},
			{Type: "separator"},
			{Label: "Close", Role: "close-window", Accelerator: "CmdOrCtrl+W", Click: action(m.closeFocusedItem)},
		}},
		{Label: "Edit", Items: []native.MenuItem{
			{Label: "Undo", Role: "undo", Accelerator: "CmdOrCtrl+Z"},
			{Label: "Redo", Role: "redo", Accelerator: "CmdOrCtrl+Shift+Z"},
			{Type: "separator"},
			{Label: "Cut", Role: "cut", Accelerator: "CmdOrCtrl+X"},
			{Label: "Copy", Role: "copy", Accelerator: "CmdOrCtrl+C"},
			{Label: "Paste", Role: "paste", Accelerator: "CmdOrCtrl+V"},
			{Label: "Select All", Role: "select-all", Accelerator: "CmdOrCtrl+A"},
		}},
		{Label: "View", Items: []native.MenuItem{
			{Label: "System Appearance", Checked: m.Preference.Peek() == "system", Click: action(func() { m.setTheme("system") })},
			{Label: "Light Appearance", Checked: m.Preference.Peek() == "light", Click: action(func() { m.setTheme("light") })},
			{Label: "Dark Appearance", Checked: m.Preference.Peek() == "dark", Click: action(func() { m.setTheme("dark") })},
		}},
		{Label: "Window", Items: []native.MenuItem{
			{Label: "Minimize", Role: "minimize-window", Accelerator: "CmdOrCtrl+M"},
			{Label: "Zoom", Role: "zoom-window"},
			{Label: "Enter Full Screen", Role: "toggle-fullscreen", Accelerator: "CmdOrCtrl+Ctrl+F"},
		}},
	})
}
func ptr[T any](value T) *T { return &value }
