package main

import (
	"os"
	"path/filepath"

	"github.com/egoist/quickgui/examples/quick-git-go/internal/git"
	"github.com/egoist/quickgui/examples/quick-git-go/internal/model"
	appui "github.com/egoist/quickgui/examples/quick-git-go/internal/ui"
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/reactive"
	gui "github.com/egoist/quickgui/packages/go/ui"
)

type session struct {
	window *native.Window
	store  *model.Store
}

func main() {
	native.Run(func() {
		native.App.GetPaths(func(paths *native.AppPaths, _ error) {
			dataDir := ""
			if paths != nil {
				dataDir = paths.DataDir
			}
			if dataDir == "" {
				dataDir = native.FallbackDataDir("Quick Git")
			}
			startApp(filepath.Join(dataDir, "quick-git-state.json"))
		})
	})
}

func startApp(statePath string) {
	persistence := model.CreatePersistence(statePath, model.LoadPersistedState(statePath), 0)
	runner := git.NewRunner(4)
	appearance, setAppearance := gui.CreateSignal("light")
	sessions := map[uint32]*session{}
	var active *native.Window

	activeStore := func() *model.Store {
		if active == nil || active.Closed {
			return nil
		}
		if session, ok := sessions[active.NativeID]; ok {
			return session.store
		}
		return nil
	}

	var openWindow func(initial string) *native.Window
	var openRepositoryPath func(path string, from *native.Window)
	var openRepositoryDialog func(from *native.Window)
	var installMenu func()

	windowShowing := func(root string) *native.Window {
		for _, session := range sessions {
			if session.window.Closed {
				continue
			}
			current := session.store.MainRepository()
			if current == nil {
				current = session.store.Repository()
			}
			if current != nil && current.Root() == root {
				return session.window
			}
		}
		return nil
	}

	openRepositoryPath = func(path string, from *native.Window) {
		target := model.CanonicalPath(path)
		if existing := windowShowing(target); existing != nil {
			existing.Focus()
			return
		}
		var fromStore *model.Store
		if from != nil && !from.Closed {
			if session, ok := sessions[from.NativeID]; ok {
				fromStore = session.store
			}
		}
		if fromStore != nil && fromStore.Repository() == nil && fromStore.Opening() == "" {
			fromStore.OpenRepository(target)
			fromStore.SetView(model.ViewChanges)
			return
		}
		openWindow(target)
	}

	openRepositoryDialog = func(from *native.Window) {
		native.ShowOpenDialog(native.OpenDialogOptions{
			Title: "Open Repository", ButtonLabel: "Open", Properties: []string{"openDirectory"}, Window: from,
		}, func(result native.OpenDialogResult, err error) {
			if err != nil || result.Canceled || len(result.FilePaths) == 0 {
				return
			}
			openRepositoryPath(result.FilePaths[0], from)
		})
	}

	installMenu = func() {
		recent := persistence.Current().RecentRepositories
		hasRepo := activeStore() != nil && activeStore().Repository() != nil
		recentItems := make([]native.MenuItem, 0, len(recent))
		for _, path := range recent {
			p := path
			recentItems = append(recentItems, native.MenuItem{Label: p, Click: func() { openRepositoryPath(p, active) }})
		}
		enabled := true
		native.SetApplicationMenu([]native.MenuDefinition{
			{Label: "File", Items: []native.MenuItem{
				{Label: "Open Repository…", Accelerator: "CmdOrCtrl+O", Click: func() {
					if w := active; w != nil {
						openRepositoryDialog(w)
					}
				}},
				{Type: "submenu", Label: "Open Recent", Enabled: boolPtr(len(recent) > 0), Items: recentItems},
				{Type: "separator"},
				{Label: "New Branch…", Accelerator: "CmdOrCtrl+Shift+N", Click: func() {}},
				{Label: "Close Repository", Accelerator: "CmdOrCtrl+Shift+W", Click: func() {
					if store := activeStore(); store != nil {
						store.CloseRepository()
					}
				}},
				{Type: "role", Label: "Close Window", Role: "close-window", Accelerator: "CmdOrCtrl+W"},
			}},
			{Label: "Edit", Items: []native.MenuItem{
				{Type: "role", Label: "Undo", Role: "undo", Accelerator: "CmdOrCtrl+Z"},
				{Type: "role", Label: "Redo", Role: "redo", Accelerator: "CmdOrCtrl+Shift+Z"},
				{Type: "separator"},
				{Type: "role", Label: "Cut", Role: "cut", Accelerator: "CmdOrCtrl+X"},
				{Type: "role", Label: "Copy", Role: "copy", Accelerator: "CmdOrCtrl+C"},
				{Type: "role", Label: "Paste", Role: "paste", Accelerator: "CmdOrCtrl+V"},
				{Type: "role", Label: "Select All", Role: "select-all", Accelerator: "CmdOrCtrl+A"},
			}},
			{Label: "View", Items: []native.MenuItem{
				{Label: "Changes", Accelerator: "CmdOrCtrl+1", Click: func() {
					if s := activeStore(); s != nil {
						s.SetView(model.ViewChanges)
					}
				}},
				{Label: "History", Accelerator: "CmdOrCtrl+2", Click: func() {
					if s := activeStore(); s != nil {
						s.SetView(model.ViewHistory)
					}
				}},
				{Label: "Branches", Accelerator: "CmdOrCtrl+3", Click: func() {
					if s := activeStore(); s != nil {
						s.SetView(model.ViewBranches)
					}
				}},
				{Label: "Worktrees", Accelerator: "CmdOrCtrl+4", Click: func() {
					if s := activeStore(); s != nil {
						s.SetView(model.ViewWorktrees)
					}
				}},
				{Label: "Stashes", Accelerator: "CmdOrCtrl+5", Click: func() {
					if s := activeStore(); s != nil {
						s.SetView(model.ViewStashes)
					}
				}},
				{Type: "separator"},
				{Label: "Refresh", Accelerator: "CmdOrCtrl+R", Click: func() {
					if s := activeStore(); s != nil {
						s.Refresh()
					}
				}},
			}},
			{Label: "Repository", Items: []native.MenuItem{
				{Label: "Commit", Accelerator: "CmdOrCtrl+Enter", Click: func() {
					if s := activeStore(); s != nil {
						s.Commit()
					}
				}},
				{Label: "Generate Commit Message", Accelerator: "CmdOrCtrl+Shift+G", Click: func() {
					if s := activeStore(); s != nil {
						s.GenerateMessage("")
					}
				}},
				{Type: "separator"},
				{Label: "Stage All", Accelerator: "CmdOrCtrl+Shift+A", Click: func() {
					if s := activeStore(); s != nil {
						s.StageAll()
					}
				}},
				{Label: "Unstage All", Accelerator: "CmdOrCtrl+Shift+U", Click: func() {
					if s := activeStore(); s != nil {
						s.UnstageAll()
					}
				}},
				{Type: "separator"},
				{Label: "Fetch", Accelerator: "CmdOrCtrl+Shift+F", Click: func() {
					if s := activeStore(); s != nil {
						s.Fetch()
					}
				}},
				{Label: "Pull", Accelerator: "CmdOrCtrl+Shift+L", Click: func() {
					if s := activeStore(); s != nil {
						s.Pull()
					}
				}},
				{Label: "Push", Accelerator: "CmdOrCtrl+Shift+P", Click: func() {
					if s := activeStore(); s != nil {
						s.Push()
					}
				}},
				{Type: "separator"},
				{Label: "Reveal in Finder", Enabled: boolPtr(hasRepo), Click: func() {
					if s := activeStore(); s != nil && s.Repository() != nil {
						native.ShowItemInFolder(s.Repository().Root(), func(error) {})
					}
				}},
			}},
			{Label: "Window", Items: []native.MenuItem{
				{Type: "role", Label: "Minimize", Role: "minimize-window", Accelerator: "CmdOrCtrl+M"},
				{Type: "role", Label: "Zoom", Role: "zoom-window"},
			}},
			{Label: "Help", Items: []native.MenuItem{
				{Label: "QuickGUI on GitHub", Click: func() { native.OpenExternal("https://github.com/egoist/quickgui", func(error) {}) }},
			}},
		})
		_ = enabled
	}

	openWindow = func(initial string) *native.Window {
		store := model.CreateStore(model.StoreOptions{
			Runner: runner, Persistence: persistence,
			Trash: func(path string) error {
				done := make(chan error, 1)
				native.Dispatch(func() {
					native.TrashItem(path, func(err error) { done <- err })
				})
				return <-done
			},
		})
		window := native.NewWindow(native.WindowOptions{
			Title: "Quick Git", Width: 1240, Height: 800, MinimumWidth: 900, MinimumHeight: 560,
			Background: "transparent", Vibrancy: "sidebar", VisualEffectState: "followWindow",
			Appearance: "system", TitleBarStyle: "hiddenInset",
			TrafficLightPosition: &native.Point{X: 14, Y: 19},
			Renderer: gui.CreateRenderer(appui.App(store, appearance, func(_ string) {
				openRepositoryDialog(native.CurrentWindow())
			}, func(path string) {
				openRepositoryPath(path, native.CurrentWindow())
			})),
		})
		sessions[window.NativeID] = &session{window: window, store: store}
		active = window
		window.On(native.WindowReadyToShow, func(native.WindowEvent) {
			window.GetState(func(state native.WindowState, err error) {
				if err == nil && !window.Closed {
					if state.Appearance == "dark" {
						setAppearance("dark")
					} else {
						setAppearance("light")
					}
				}
			})
		})
		window.On(native.WindowAppearance, func(event native.WindowEvent) {
			if event.Appearance == "dark" || event.Appearance == "light" {
				setAppearance(event.Appearance)
			}
		})
		window.On(native.WindowFocus, func(native.WindowEvent) {
			active = window
			installMenu()
			if store.Repository() != nil {
				store.Refresh()
			}
		})
		reactive.CreateRoot(func(dispose func()) struct{} {
			gui.CreateEffect(func() {
				name := store.RepositoryName()
				branch := ""
				if status := store.Status(); status != nil {
					branch = status.Branch
				}
				root := ""
				if repo := store.Repository(); repo != nil {
					root = repo.Root()
				}
				if window.Closed {
					return
				}
				title := "Quick Git"
				if name != "" {
					title = name
					if branch != "" {
						title += " — " + branch
					}
				}
				window.SetTitle(title)
				window.SetRepresentedFile(root)
			})
			window.On(native.WindowClosed, func(native.WindowEvent) {
				dispose()
			})
			return struct{}{}
		})
		window.On(native.WindowClosed, func(native.WindowEvent) {
			delete(sessions, window.NativeID)
			store.Dispose()
			if active == window {
				active = nil
				for _, session := range sessions {
					active = session.window
				}
			}
			installMenu()
		})
		if initial != "" {
			store.OpenRepository(initial)
		}
		installMenu()
		return window
	}

	reactive.CreateRoot(func(func()) struct{} {
		gui.CreateEffect(func() {
			_ = persistence.State()
			_ = activeStore()
			installMenu()
		})
		return struct{}{}
	})

	native.App.OnReopen(func(event native.ReopenEvent) {
		if !event.HasVisibleWindows {
			openWindow("")
		}
	})

	initial := os.Getenv("QUICK_GIT_OPEN")
	if initial == "" {
		initial = persistence.Current().LastRepository
	}
	openWindow(initial)
}

func boolPtr(value bool) *bool { return &value }
