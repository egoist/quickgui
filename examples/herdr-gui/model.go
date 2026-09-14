package main

import (
	"fmt"
	"net/url"
	"path/filepath"
	"slices"
	"strings"

	"github.com/egoist/quickgui/extensions/terminal"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

type space struct{ ID, Name, Path string }
type workspaceTab struct {
	ID           int
	SpaceID      string
	Number       int
	CustomName   string
	PaneIDs      []int
	ActivePaneID int
	Direction    string
}
type pane struct {
	ID, TabID                     int
	SpaceID, Label, Program       string
	Arguments                     []string
	Environment                   map[string]string
	RequestedAgent, InitialPrompt string
	Status                        *reactive.Signal[terminal.StatusDetails]
}
type launcher struct{ ID, Label, Mark, Description, Executable string }

func (l launcher) installed() bool { return l.Executable != "" }
func (l launcher) arguments(prompt string) []string {
	if prompt == "" {
		return nil
	}
	if l.ID == "opencode" {
		return []string{"--prompt", prompt}
	}
	return []string{prompt}
}

type model struct {
	Window                                    *native.Window
	Home                                      string
	Spaces                                    *reactive.Signal[[]space]
	Tabs                                      *reactive.Signal[[]workspaceTab]
	Panes                                     *reactive.Signal[[]*pane]
	ActiveSpaceID                             *reactive.Signal[string]
	ActiveTabID, ActivePaneID                 *reactive.Signal[int]
	Appearance, Preference                    *reactive.Signal[string]
	SidebarWidth, SectionRatio                *reactive.Signal[float64]
	SheetOpen, CatalogLoading                 *reactive.Signal[bool]
	SelectedLauncherID, Prompt, Error         *reactive.Signal[string]
	Launchers                                 *reactive.Signal[[]launcher]
	Environment                               map[string]string
	lastTab                                   map[string]int
	terminals                                 map[int]*native.Node
	nextTab, nextPane                         int
	disposed, addingSpace                     bool
	sidebarStart, sectionStart, sectionHeight float64
	persist                                   func(savedState)
}

func newModel(home string, saved savedState) *model {
	spaces := normalizeSpaces(saved.Spaces, home)
	active := spaces[0]
	for _, s := range spaces {
		if s.ID == saved.ActiveSpaceID {
			active = s
		}
	}
	preference := saved.Appearance
	if preference != "light" && preference != "dark" {
		preference = "system"
	}
	m := &model{
		Home: home, Spaces: reactive.NewSignal(spaces), Tabs: reactive.NewSignal([]workspaceTab{}), Panes: reactive.NewSignal([]*pane{}),
		ActiveSpaceID: reactive.NewSignal(active.ID), ActiveTabID: reactive.NewSignal(0), ActivePaneID: reactive.NewSignal(0),
		Appearance: reactive.NewSignal(choose(preference == "light", "light", "dark")), Preference: reactive.NewSignal(preference),
		SidebarWidth: reactive.NewSignal(savedDimension(saved.SidebarWidth, 248, 196, 380)),
		SectionRatio: reactive.NewSignal(savedDimension(saved.SidebarSectionRatio, .5, .25, .75)),
		SheetOpen:    reactive.NewSignal(false), CatalogLoading: reactive.NewSignal(true),
		SelectedLauncherID: reactive.NewSignal("codex"), Prompt: reactive.NewSignal(""), Error: reactive.NewSignal(""),
		Launchers: reactive.NewSignal(launcherDefinitions()), lastTab: map[string]int{}, terminals: map[int]*native.Node{},
		nextTab: 1, nextPane: 1, sectionHeight: 740,
	}
	m.newTerminal(active)
	return m
}

func choose[T any](condition bool, yes, no T) T {
	if condition {
		return yes
	}
	return no
}
func clamp(value, minimum, maximum float64) float64 { return min(max(value, minimum), maximum) }
func savedDimension(value *float64, fallback, minimum, maximum float64) float64 {
	if value == nil {
		return fallback
	}
	return clamp(*value, minimum, maximum)
}
func (m *model) activeSpace() space {
	for _, s := range m.Spaces.Read() {
		if s.ID == m.ActiveSpaceID.Read() {
			return s
		}
	}
	return m.Spaces.Read()[0]
}
func (m *model) tab(id int) *workspaceTab {
	for _, t := range m.Tabs.Read() {
		if t.ID == id {
			return &t
		}
	}
	return nil
}
func (m *model) activeTab() *workspaceTab { return m.tab(m.ActiveTabID.Read()) }
func (m *model) activePane() *pane {
	for _, p := range m.Panes.Read() {
		if p.ID == m.ActivePaneID.Read() {
			return p
		}
	}
	return nil
}
func (m *model) spaceTabs() []workspaceTab {
	var out []workspaceTab
	for _, t := range m.Tabs.Read() {
		if t.SpaceID == m.ActiveSpaceID.Read() {
			out = append(out, t)
		}
	}
	return out
}
func (m *model) tabPanes(id int) []*pane {
	var out []*pane
	if t := m.tab(id); t != nil {
		for _, id := range t.PaneIDs {
			for _, p := range m.Panes.Read() {
				if p.ID == id {
					out = append(out, p)
					break
				}
			}
		}
	}
	return out
}
func (m *model) visibleAgents() []*pane {
	var out []*pane
	// Preserve space and tab order; detection is exclusively native PTY status.
	for _, s := range m.Spaces.Read() {
		for _, t := range m.Tabs.Read() {
			if t.SpaceID == s.ID {
				for _, p := range m.tabPanes(t.ID) {
					if p.Status.Read().Agent != "" {
						out = append(out, p)
					}
				}
			}
		}
	}
	return out
}
func (m *model) selectedLauncher() launcher {
	for _, l := range m.Launchers.Read() {
		if l.ID == m.SelectedLauncherID.Read() {
			return l
		}
	}
	return m.Launchers.Read()[0]
}
func (m *model) snapshot() savedState {
	width, ratio := m.SidebarWidth.Peek(), m.SectionRatio.Peek()
	saved := savedState{ActiveSpaceID: m.ActiveSpaceID.Peek(), SidebarWidth: &width, SidebarSectionRatio: &ratio, Appearance: m.Preference.Peek()}
	for _, s := range m.Spaces.Peek() {
		saved.Spaces = append(saved.Spaces, savedSpace{Path: s.Path})
	}
	return saved
}
func (m *model) save() {
	if m.persist != nil {
		m.persist(m.snapshot())
	}
}
func (m *model) shellPane(s space, tabID int) *pane {
	count := 1
	for _, p := range m.Panes.Peek() {
		if p.SpaceID == s.ID {
			count++
		}
	}
	p := &pane{ID: m.nextPane, TabID: tabID, SpaceID: s.ID, Label: fmt.Sprintf("Terminal %d", count), Program: loginShell(), Arguments: []string{"-l"}, Environment: m.Environment, Status: reactive.NewSignal(terminal.StatusDetails{
		Status:           "starting",
		WorkingDirectory: s.Path,
	})}
	m.nextPane++
	return p
}
func (m *model) addTab(s space, p *pane) {
	number := 1
	for _, t := range m.Tabs.Peek() {
		if t.SpaceID == s.ID {
			number = max(number, t.Number+1)
		}
	}
	t := workspaceTab{ID: p.TabID, SpaceID: s.ID, Number: number, PaneIDs: []int{p.ID}, ActivePaneID: p.ID, Direction: "horizontal"}
	reactive.Batch(func() {
		m.Panes.Write(append(slices.Clone(m.Panes.Peek()), p))
		m.Tabs.Write(append(slices.Clone(m.Tabs.Peek()), t))
		m.activateTab(t)
	})
	m.save()
}
func (m *model) newTerminal(s space) { p := m.shellPane(s, m.nextTab); m.nextTab++; m.addTab(s, p) }
func (m *model) activateTab(t workspaceTab) {
	m.ActiveSpaceID.Write(t.SpaceID)
	m.ActiveTabID.Write(t.ID)
	m.ActivePaneID.Write(t.ActivePaneID)
	m.lastTab[t.SpaceID] = t.ID
	m.focusPane(t.ActivePaneID)
}
func (m *model) selectTab(t workspaceTab) { reactive.Batch(func() { m.activateTab(t) }); m.save() }
func (m *model) selectSpace(s space) {
	var candidate *workspaceTab
	for _, t := range m.Tabs.Peek() {
		if t.SpaceID == s.ID {
			candidate = &t
			if t.ID == m.lastTab[s.ID] {
				break
			}
		}
	}
	if candidate != nil {
		m.selectTab(*candidate)
	} else {
		m.newTerminal(s)
	}
}
func (m *model) splitTerminal(direction string) {
	t := m.activeTab()
	if t == nil {
		m.newTerminal(m.activeSpace())
		return
	}
	p := m.shellPane(m.activeSpace(), t.ID)
	ids := slices.Clone(t.PaneIDs)
	offset := slices.Index(ids, t.ActivePaneID) + 1
	ids = slices.Insert(ids, offset, p.ID)
	reactive.Batch(func() {
		m.Panes.Write(append(slices.Clone(m.Panes.Peek()), p))
		m.updateTab(t.ID, func(tab *workspaceTab) { tab.PaneIDs = ids; tab.ActivePaneID = p.ID; tab.Direction = direction })
		m.ActivePaneID.Write(p.ID)
	})
	m.focusPane(p.ID)
}
func (m *model) updateTab(id int, update func(*workspaceTab)) {
	tabs := slices.Clone(m.Tabs.Peek())
	for i := range tabs {
		if tabs[i].ID == id {
			update(&tabs[i])
			break
		}
	}
	m.Tabs.Write(tabs)
}
func (m *model) selectPane(p *pane) {
	t := m.tab(p.TabID)
	if t == nil {
		return
	}
	reactive.Batch(func() {
		m.updateTab(t.ID, func(t *workspaceTab) { t.ActivePaneID = p.ID })
		t.ActivePaneID = p.ID
		m.activateTab(*t)
	})
	m.save()
}
func (m *model) focusPane(id int) {
	if m.Window == nil {
		return
	}
	native.Dispatch(func() {
		if !m.disposed && !m.Window.Closed && !m.SheetOpen.Peek() && m.ActivePaneID.Peek() == id {
			if node := m.terminals[id]; node != nil {
				node.Focus()
			}
		}
	})
}
func (m *model) registerTerminal(p *pane, node *native.Node) {
	m.terminals[p.ID] = node
	reactive.OnCleanup(func() {
		if m.terminals[p.ID] == node {
			delete(m.terminals, p.ID)
		}
	})
	if m.ActivePaneID.Peek() == p.ID {
		m.focusPane(p.ID)
	}
}
func (m *model) closePane(id int) {
	var closing *pane
	for _, p := range m.Panes.Peek() {
		if p.ID == id {
			closing = p
			break
		}
	}
	if closing == nil {
		return
	}
	t := m.tab(closing.TabID)
	if t == nil {
		return
	}
	if len(t.PaneIDs) == 1 {
		m.closeTab(t.ID)
		return
	}
	ids := slices.DeleteFunc(slices.Clone(t.PaneIDs), func(value int) bool { return value == id })
	next := t.ActivePaneID
	if next == id {
		next = ids[min(slices.Index(t.PaneIDs, id), len(ids)-1)]
	}
	reactive.Batch(func() {
		m.Panes.Write(slices.DeleteFunc(slices.Clone(m.Panes.Peek()), func(p *pane) bool { return p.ID == id }))
		m.updateTab(t.ID, func(t *workspaceTab) { t.PaneIDs = ids; t.ActivePaneID = next })
		if m.ActivePaneID.Peek() == id {
			m.ActivePaneID.Write(next)
			m.focusPane(next)
		}
	})
}
func (m *model) closeTab(id int) {
	t := m.tab(id)
	if t == nil {
		return
	}
	reactive.Batch(func() {
		tabs := slices.DeleteFunc(slices.Clone(m.Tabs.Peek()), func(t workspaceTab) bool { return t.ID == id })
		m.Tabs.Write(tabs)
		m.Panes.Write(slices.DeleteFunc(slices.Clone(m.Panes.Peek()), func(p *pane) bool { return p.TabID == id }))
		if m.ActiveTabID.Peek() != id {
			return
		}
		var next *workspaceTab
		for _, candidate := range tabs {
			if next == nil || candidate.SpaceID == t.SpaceID {
				next = &candidate
			}
		}
		if next != nil {
			m.activateTab(*next)
		} else {
			m.ActiveTabID.Write(0)
			m.ActivePaneID.Write(0)
			m.newTerminal(m.activeSpace())
		}
	})
	m.save()
}
func focusedCloseTarget(tab *workspaceTab, paneID, tabCount int) (string, int) {
	if tab != nil && len(tab.PaneIDs) > 1 && slices.Contains(tab.PaneIDs, paneID) {
		return "pane", paneID
	}
	if tab != nil && tabCount > 1 {
		return "tab", tab.ID
	}
	return "window", 0
}
func (m *model) closeFocusedItem() {
	kind, id := focusedCloseTarget(m.activeTab(), m.ActivePaneID.Peek(), len(m.Tabs.Peek()))
	switch kind {
	case "pane":
		m.closePane(id)
	case "tab":
		m.closeTab(id)
	default:
		if m.Window != nil {
			m.Window.Close()
		}
	}
}
func (m *model) restartPane(p *pane) {
	replacement := *p
	replacement.Status = reactive.NewSignal(terminal.StatusDetails{
		Status:           "starting",
		WorkingDirectory: p.Status.Peek().WorkingDirectory,
	})
	panes := slices.Clone(m.Panes.Peek())
	for i, candidate := range panes {
		if candidate == p {
			panes[i] = &replacement
		}
	}
	m.Panes.Write(panes)
	m.focusPane(p.ID)
}
func (m *model) addSpacePath(path string) {
	if path == "" {
		return
	}
	for _, s := range m.Spaces.Peek() {
		if s.Path == path {
			m.selectSpace(s)
			return
		}
	}
	s := space{ID: path, Name: filepath.Base(path), Path: path}
	reactive.Batch(func() { m.Spaces.Write(append(slices.Clone(m.Spaces.Peek()), s)); m.newTerminal(s) })
	m.save()
}
func (m *model) removeSpace(s space) {
	if len(m.Spaces.Peek()) == 1 {
		return
	}
	reactive.Batch(func() {
		spaces := slices.DeleteFunc(slices.Clone(m.Spaces.Peek()), func(candidate space) bool { return candidate.ID == s.ID })
		m.Spaces.Write(spaces)
		m.Tabs.Write(slices.DeleteFunc(slices.Clone(m.Tabs.Peek()), func(t workspaceTab) bool { return t.SpaceID == s.ID }))
		m.Panes.Write(slices.DeleteFunc(slices.Clone(m.Panes.Peek()), func(p *pane) bool { return p.SpaceID == s.ID }))
		delete(m.lastTab, s.ID)
		if m.ActiveSpaceID.Peek() == s.ID {
			m.selectSpace(spaces[0])
		}
	})
	m.save()
}
func (m *model) openAgentSheet() {
	for _, l := range m.Launchers.Peek() {
		if l.installed() {
			m.SelectedLauncherID.Write(l.ID)
			break
		}
	}
	m.Prompt.Write("")
	m.SheetOpen.Write(true)
}
func (m *model) closeAgentSheet() { m.SheetOpen.Write(false); m.focusPane(m.ActivePaneID.Peek()) }
func (m *model) launchAgent() {
	l := m.selectedLauncher()
	if m.CatalogLoading.Peek() || !l.installed() {
		return
	}
	p := m.shellPane(m.activeSpace(), m.nextTab)
	m.nextTab++
	p.Label = l.Label
	p.Program = l.Executable
	p.InitialPrompt = strings.TrimSpace(m.Prompt.Peek())
	p.Arguments = l.arguments(p.InitialPrompt)
	p.RequestedAgent = l.ID
	reactive.Batch(func() { m.SheetOpen.Write(false); m.addTab(m.activeSpace(), p) })
}

func agentLabel(agent string) string {
	switch agent {
	case "claude":
		return "Claude Code"
	case "codex":
		return "Codex"
	case "opencode":
		return "OpenCode"
	case "copilot":
		return "GitHub Copilot"
	case "gemini":
		return "Gemini"
	case "cursor":
		return "Cursor"
	case "agy":
		return "Antigravity"
	case "mastracode":
		return "Mastra Code"
	case "qodercli":
		return "Qoder CLI"
	case "":
		return "Agent"
	}
	return strings.ToUpper(agent[:1]) + agent[1:]
}
func normalizeTerminalPath(path string) string {
	if strings.HasPrefix(path, "file://") {
		if u, err := url.Parse(path); err == nil {
			return u.Path
		}
	}
	return path
}
func shortPath(path, home string) string {
	path = normalizeTerminalPath(path)
	if path == home {
		return "~"
	}
	if strings.HasPrefix(path, home+string(filepath.Separator)) {
		return "~" + strings.TrimPrefix(path, home)
	}
	return path
}
func paneTitle(p *pane) string {
	status := p.Status.Read()
	title := strings.TrimSpace(status.Title)
	lower := strings.ToLower(title)
	cwd := normalizeTerminalPath(status.WorkingDirectory)
	generic := slices.Contains([]string{"zsh", "bash", "sh", "fish", "xterm", "terminal", strings.ToLower(status.Agent)}, lower) || title == status.WorkingDirectory || title == cwd || title == filepath.Base(cwd)
	if title != "" && !generic {
		return title
	}
	if status.Agent != "" {
		return agentLabel(status.Agent)
	}
	if p.RequestedAgent != "" {
		return agentLabel(p.RequestedAgent)
	}
	return p.Label
}
func tabTitle(t workspaceTab, panes []*pane) string {
	if t.CustomName != "" {
		return t.CustomName
	}
	for _, p := range panes {
		if p.ID == t.ActivePaneID {
			return paneTitle(p)
		}
	}
	for _, p := range panes {
		if slices.Contains(t.PaneIDs, p.ID) {
			return paneTitle(p)
		}
	}
	return fmt.Sprint(t.Number)
}
func statusLabel(status string) string {
	if status == "blocked" {
		return "needs input"
	}
	return status
}
func aggregateStatus(panes []*pane) string {
	for _, status := range []string{"blocked", "working", "idle"} {
		for _, p := range panes {
			if p.Status.Read().Agent != "" && p.Status.Read().AgentStatus == status {
				return status
			}
		}
	}
	return ""
}
