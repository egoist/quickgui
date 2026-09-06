package model

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/reactive"
)

const (
	StateVersion          = 1
	MaxRecentRepositories = 20
	DefaultSidebarWidth   = 236
	DefaultChangesSplit   = 320
	DefaultHistorySplit   = 420
)

type PersistedState struct {
	RecentRepositories []string `json:"recentRepositories"`
	LastRepository     string   `json:"lastRepository,omitempty"`
	SidebarWidth       float64  `json:"sidebarWidth"`
	ChangesSplit       float64  `json:"changesSplit"`
	HistorySplit       float64  `json:"historySplit"`
	PreferredAgent     string   `json:"preferredAgent,omitempty"`
	HistoryAllBranches bool     `json:"historyAllBranches"`
}

type PersistedPatch struct {
	RecentRepositories *[]string
	LastRepository     *string
	ForgetLast         bool
	SidebarWidth       *float64
	ChangesSplit       *float64
	HistorySplit       *float64
	PreferredAgent     *string
	HistoryAllBranches *bool
}

func DefaultState() PersistedState {
	return PersistedState{
		RecentRepositories: []string{},
		SidebarWidth:       DefaultSidebarWidth,
		ChangesSplit:       DefaultChangesSplit,
		HistorySplit:       DefaultHistorySplit,
	}
}

func ApplyPatch(current PersistedState, patch PersistedPatch) PersistedState {
	next := current
	if patch.RecentRepositories != nil {
		next.RecentRepositories = append([]string{}, (*patch.RecentRepositories)...)
	}
	if patch.ForgetLast {
		next.LastRepository = ""
	} else if patch.LastRepository != nil {
		next.LastRepository = *patch.LastRepository
	}
	if patch.SidebarWidth != nil {
		next.SidebarWidth = *patch.SidebarWidth
	}
	if patch.ChangesSplit != nil {
		next.ChangesSplit = *patch.ChangesSplit
	}
	if patch.HistorySplit != nil {
		next.HistorySplit = *patch.HistorySplit
	}
	if patch.PreferredAgent != nil {
		next.PreferredAgent = *patch.PreferredAgent
	}
	if patch.HistoryAllBranches != nil {
		next.HistoryAllBranches = *patch.HistoryAllBranches
	}
	return next
}

func ParsePersistedState(raw []byte) PersistedState {
	var record map[string]any
	if err := json.Unmarshal(raw, &record); err != nil {
		return DefaultState()
	}
	version, _ := record["version"].(float64)
	if int(version) != StateVersion {
		return DefaultState()
	}
	state := DefaultState()
	if list, ok := record["recentRepositories"].([]any); ok {
		seen := map[string]struct{}{}
		for _, entry := range list {
			path, ok := entry.(string)
			if !ok || path == "" {
				continue
			}
			path = CanonicalPath(path)
			if _, exists := seen[path]; exists {
				continue
			}
			seen[path] = struct{}{}
			state.RecentRepositories = append(state.RecentRepositories, path)
			if len(state.RecentRepositories) >= MaxRecentRepositories {
				break
			}
		}
	}
	if last, ok := record["lastRepository"].(string); ok {
		state.LastRepository = last
	}
	state.SidebarWidth = clampNumber(record["sidebarWidth"], 180, 420, DefaultSidebarWidth)
	state.ChangesSplit = clampNumber(record["changesSplit"], 220, 800, DefaultChangesSplit)
	state.HistorySplit = clampNumber(record["historySplit"], 260, 1000, DefaultHistorySplit)
	if agent, ok := record["preferredAgent"].(string); ok && (agent == "codex" || agent == "claude") {
		state.PreferredAgent = agent
	}
	if all, ok := record["historyAllBranches"].(bool); ok {
		state.HistoryAllBranches = all
	}
	return state
}

func clampNumber(value any, min, max, fallback float64) float64 {
	n, ok := value.(float64)
	if !ok {
		return fallback
	}
	if n < min {
		return min
	}
	if n > max {
		return max
	}
	return n
}

func LoadPersistedState(path string) PersistedState {
	if path == "" {
		return DefaultState()
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return DefaultState()
	}
	return ParsePersistedState(data)
}

func SavePersistedState(path string, state PersistedState) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	payload := map[string]any{
		"version":            StateVersion,
		"recentRepositories": state.RecentRepositories,
		"sidebarWidth":       state.SidebarWidth,
		"changesSplit":       state.ChangesSplit,
		"historySplit":       state.HistorySplit,
		"historyAllBranches": state.HistoryAllBranches,
	}
	if state.LastRepository != "" {
		payload["lastRepository"] = state.LastRepository
	}
	if state.PreferredAgent != "" {
		payload["preferredAgent"] = state.PreferredAgent
	}
	data, err := json.MarshalIndent(payload, "", "  ")
	if err != nil {
		return err
	}
	temporary := path + "." + strconvPID() + ".tmp"
	if err := os.WriteFile(temporary, append(data, '\n'), 0o644); err != nil {
		return err
	}
	if err := os.Rename(temporary, path); err != nil {
		_ = os.Remove(temporary)
		return err
	}
	return nil
}

func strconvPID() string {
	return itoa(os.Getpid())
}

func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	neg := n < 0
	if neg {
		n = -n
	}
	var buf [20]byte
	i := len(buf)
	for n > 0 {
		i--
		buf[i] = byte('0' + n%10)
		n /= 10
	}
	if neg {
		i--
		buf[i] = '-'
	}
	return string(buf[i:])
}

func RememberRepository(recent []string, path string) []string {
	target := CanonicalPath(path)
	next := []string{target}
	for _, entry := range recent {
		if CanonicalPath(entry) != target {
			next = append(next, entry)
		}
		if len(next) >= MaxRecentRepositories {
			break
		}
	}
	return next
}

type Persistence struct {
	path    string
	mu      sync.Mutex
	current PersistedState
	State   reactive.Accessor[PersistedState]
	set     reactive.Setter[PersistedState]
	timer   *time.Timer
	delay   time.Duration
}

type persistenceSignals struct {
	state reactive.Accessor[PersistedState]
	set   reactive.Setter[PersistedState]
}

func CreatePersistence(path string, initial PersistedState, saveDelay time.Duration) *Persistence {
	if saveDelay <= 0 {
		saveDelay = 300 * time.Millisecond
	}
	current := initial
	signals := reactive.CreateRoot(func(func()) persistenceSignals {
		state, setState := reactive.CreateSignal(current)
		return persistenceSignals{state: state, set: setState}
	})
	return &Persistence{path: path, current: current, State: signals.state, set: signals.set, delay: saveDelay}
}

func (p *Persistence) Current() PersistedState {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.current
}

func (p *Persistence) Update(patch PersistedPatch) {
	p.mu.Lock()
	p.current = ApplyPatch(p.current, patch)
	next := p.current
	if p.timer != nil {
		p.timer.Stop()
	}
	path := p.path
	delay := p.delay
	p.timer = time.AfterFunc(delay, func() {
		p.mu.Lock()
		snapshot := p.current
		p.timer = nil
		p.mu.Unlock()
		if path != "" {
			if err := SavePersistedState(path, snapshot); err != nil {
				// Persistence is best-effort; the in-memory snapshot stays current.
			}
		}
	})
	p.mu.Unlock()
	native.Dispatch(func() {
		p.set(next)
	})
}

func (p *Persistence) Flush() {
	p.mu.Lock()
	if p.timer != nil {
		p.timer.Stop()
		p.timer = nil
	}
	snapshot := p.current
	path := p.path
	p.mu.Unlock()
	if path != "" {
		_ = SavePersistedState(path, snapshot)
	}
}
