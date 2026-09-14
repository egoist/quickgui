package main

import "github.com/egoist/quickgui/go/ui"

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/egoist/quickgui/extensions/terminal"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

func testModel(t *testing.T, fn func(*model)) {
	t.Helper()
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		fn(newModel(t.TempDir(), savedState{}))
		return struct{}{}
	})
}
func TestFocusedCloseOrder(t *testing.T) {
	for _, tc := range []struct {
		name       string
		tab        *workspaceTab
		pane, tabs int
		kind       string
		id         int
	}{
		{"split pane", &workspaceTab{ID: 3, PaneIDs: []int{7, 8}}, 8, 1, "pane", 8},
		{"single pane with other tabs", &workspaceTab{ID: 3, PaneIDs: []int{7}}, 7, 2, "tab", 3},
		{"last terminal", &workspaceTab{ID: 3, PaneIDs: []int{7}}, 7, 1, "window", 0},
		{"no tab", nil, 0, 0, "window", 0},
	} {
		t.Run(tc.name, func(t *testing.T) {
			kind, id := focusedCloseTarget(tc.tab, tc.pane, tc.tabs)
			if kind != tc.kind || id != tc.id {
				t.Fatalf("got %s %d", kind, id)
			}
		})
	}
}
func TestSpacesRememberTabsAndRemoveOnlyTheirPanes(t *testing.T) {
	testModel(t, func(m *model) {
		home := m.activeSpace()
		m.newTerminal(home)
		homeTab := m.ActiveTabID.Peek()
		otherPath := filepath.Join(m.Home, "project")
		m.addSpacePath(otherPath)
		other := m.activeSpace()
		otherTab := m.ActiveTabID.Peek()
		m.selectSpace(home)
		if m.ActiveTabID.Peek() != homeTab {
			t.Fatal("space forgot selected tab")
		}
		m.selectSpace(other)
		if m.ActiveTabID.Peek() != otherTab {
			t.Fatal("other space forgot selected tab")
		}
		m.addSpacePath(otherPath)
		if len(m.Spaces.Peek()) != 2 || len(m.Tabs.Peek()) != 3 {
			t.Fatal("adding an existing space duplicated a tab")
		}
		m.removeSpace(other)
		if len(m.Panes.Peek()) != 2 || m.activeSpace().ID != home.ID {
			t.Fatal("removed an unrelated terminal")
		}
		m.removeSpace(home)
		if len(m.Spaces.Peek()) != 1 {
			t.Fatal("removed the final space")
		}
	})
}
func TestSplitInsertsAfterFocusedPaneAndClosesNeighbor(t *testing.T) {
	testModel(t, func(m *model) {
		first := m.activePane()
		m.splitTerminal("horizontal")
		second := m.activePane()
		m.selectPane(first)
		m.splitTerminal("vertical")
		third := m.activePane()
		tab := m.activeTab()
		if !reflect.DeepEqual(tab.PaneIDs, []int{first.ID, third.ID, second.ID}) || tab.Direction != "vertical" {
			t.Fatalf("unexpected split: %+v", tab)
		}
		m.closePane(third.ID)
		if m.ActivePaneID.Peek() != second.ID || len(m.Panes.Peek()) != 2 {
			t.Fatal("close did not focus the adjacent pane")
		}
		m.closePane(first.ID)
		if m.ActivePaneID.Peek() != second.ID {
			t.Fatal("closing an inactive pane changed selection")
		}
		m.closeTab(tab.ID)
		if len(m.Tabs.Peek()) != 1 || len(m.Panes.Peek()) != 1 || m.ActivePaneID.Peek() == second.ID {
			t.Fatal("last tab button did not create a fresh shell")
		}
	})
}
func TestRetainedTerminalIdentityAcrossNavigationThemeAndRestart(t *testing.T) {
	testModel(t, func(m *model) {
		roots := native.CollectChildren(func() *ui.Element { return appView(m) })
		if len(roots) != 1 {
			t.Fatalf("roots %d", len(roots))
		}
		first := m.activePane()
		firstNode := m.terminals[first.ID]
		if firstNode == nil {
			t.Fatal("terminal did not mount")
		}
		m.splitTerminal("horizontal")
		second := m.activePane()
		secondNode := m.terminals[second.ID]
		m.newTerminal(m.activeSpace())
		third := m.activePane()
		thirdNode := m.terminals[third.ID]
		m.selectPane(first)
		m.Appearance.Write("light")
		m.SidebarWidth.Write(330)
		m.SectionRatio.Write(.65)
		if m.terminals[first.ID] != firstNode || m.terminals[second.ID] != secondNode || m.terminals[third.ID] != thirdNode {
			t.Fatal("navigation, resize or theme restarted a PTY")
		}
		m.restartPane(first)
		if m.terminals[first.ID] == firstNode || m.terminals[first.ID] == nil {
			t.Fatal("restart did not replace the PTY")
		}
		if m.terminals[second.ID] != secondNode || m.terminals[third.ID] != thirdNode {
			t.Fatal("restart replaced unrelated PTYs")
		}
		m.closePane(second.ID)
		if m.terminals[second.ID] != nil {
			t.Fatal("closed pane retained its terminal")
		}
		if m.terminals[third.ID] != thirdNode {
			t.Fatal("closing a pane changed another tab")
		}
		m.SheetOpen.Write(true)
		m.Prompt.Write("test instruction")
		m.SheetOpen.Write(false)
		if m.terminals[third.ID] != thirdNode {
			t.Fatal("agent sheet restarted a terminal")
		}
	})
}
func TestAgentsAreDetectedNotAssumedFromLaunch(t *testing.T) {
	testModel(t, func(m *model) {
		p := m.activePane()
		p.RequestedAgent = "codex"
		if len(m.visibleAgents()) != 0 {
			t.Fatal("requested program was counted as a detected agent")
		}
		p.Status.Write(terminal.StatusDetails{
			Status:      "running",
			Agent:       "codex",
			AgentStatus: "working",
			Title:       "zsh",
		})
		if len(m.visibleAgents()) != 1 || paneTitle(p) != "Codex" || aggregateStatus(m.visibleAgents()) != "working" {
			t.Fatal("native agent state not reflected")
		}
		p.Status.Write(terminal.StatusDetails{
			Status:      "running",
			Agent:       "codex",
			AgentStatus: "blocked",
			Title:       "Review changes",
		})
		if paneTitle(p) != "Review changes" || aggregateStatus(m.visibleAgents()) != "blocked" {
			t.Fatal("agent title or blocked status lost")
		}
		p.Status.Write(terminal.StatusDetails{Status: "running", Title: "zsh"})
		if len(m.visibleAgents()) != 0 {
			t.Fatal("agent lingered after returning to shell")
		}
	})
}
func TestLaunchAgentUsesResolvedExecutableEnvironmentAndArguments(t *testing.T) {
	testModel(t, func(m *model) {
		launchers := launcherDefinitions()
		launchers[2].Executable = "/test/bin/opencode"
		m.Launchers.Write(launchers)
		m.SelectedLauncherID.Write("opencode")
		m.CatalogLoading.Write(false)
		m.Environment = map[string]string{"PATH": "/test/bin"}
		m.Prompt.Write("  explain this repo  ")
		m.SheetOpen.Write(true)
		m.launchAgent()
		p := m.activePane()
		if p.Program != "/test/bin/opencode" || !reflect.DeepEqual(p.Arguments, []string{"--prompt", "explain this repo"}) || p.Environment["PATH"] != "/test/bin" || m.SheetOpen.Peek() {
			t.Fatalf("incorrect launch: %+v", p)
		}
		if len(m.visibleAgents()) != 0 {
			t.Fatal("launcher bypassed native agent detection")
		}
	})
}
func TestPersistenceRestoresOriginalSchemaAndClampsGeometry(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "herdr-gui-state.json")
	body := `{"spaces":[{"path":"/work"},{"path":"/work"},{"path":""}],"activeSpaceId":"/work","sidebarWidth":900,"sidebarSectionRatio":0.01,"appearance":"light"}`
	if err := os.WriteFile(file, []byte(body), 0600); err != nil {
		t.Fatal(err)
	}
	saved, err := readSavedState(file)
	if err != nil {
		t.Fatal(err)
	}
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		m := newModel(dir, saved)
		if len(m.Spaces.Peek()) != 2 || m.activeSpace().ID != "/work" || m.SidebarWidth.Peek() != 380 || m.SectionRatio.Peek() != .25 || m.Preference.Peek() != "light" {
			t.Fatal("original saved state was not restored")
		}
		w := newStateWriter(file)
		w.save(savedState{Appearance: "dark"})
		w.save(m.snapshot())
		w.close()
		reloaded, err := readSavedState(file)
		if err != nil || !reflect.DeepEqual(reloaded, m.snapshot()) {
			t.Fatalf("latest state not flushed: %+v %v", reloaded, err)
		}
		return struct{}{}
	})
}
func TestLauncherDiscoveryUsesLoginPathAndSkipsNonExecutables(t *testing.T) {
	home := t.TempDir()
	bin := filepath.Join(home, "bin")
	if err := os.Mkdir(bin, 0700); err != nil {
		t.Fatal(err)
	}
	for name, mode := range map[string]os.FileMode{"codex": 0700, "claude": 0600} {
		if err := os.WriteFile(filepath.Join(bin, name), []byte("#!/bin/sh\n"), mode); err != nil {
			t.Fatal(err)
		}
	}
	t.Setenv("PATH", bin)
	found := resolveLaunchers(map[string]string{"PATH": bin}, home)
	if found[0].Executable != filepath.Join(bin, "codex") {
		t.Fatal("login PATH executable not selected")
	}
	if found[1].Executable == filepath.Join(bin, "claude") {
		t.Fatal("non-executable file selected")
	}
	if args := found[0].arguments("literal $(command) 'prompt'"); !reflect.DeepEqual(args, []string{"literal $(command) 'prompt'"}) {
		t.Fatal("prompt was not passed literally")
	}
	env := environmentMap([]string{"PATH=/usr/bin", "TOKEN=a=b", "noise", "bad\nNAME=value"})
	if env["TOKEN"] != "a=b" || len(env) != 2 {
		t.Fatal("shell environment parsing corrupted values")
	}
}
