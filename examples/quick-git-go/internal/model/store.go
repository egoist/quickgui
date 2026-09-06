package model

import (
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"

	"github.com/egoist/quickgui/examples/quick-git-go/internal/agent"
	"github.com/egoist/quickgui/examples/quick-git-go/internal/git"
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/reactive"
)

type ViewID string
type ListID string

const (
	ViewChanges         ViewID = "changes"
	ViewHistory         ViewID = "history"
	ViewBranches        ViewID = "branches"
	ViewWorktrees       ViewID = "worktrees"
	ViewStashes         ViewID = "stashes"
	ListUnstaged        ListID = "unstaged"
	ListStaged          ListID = "staged"
	HistoryPage                = 300
	MaxDiffCacheEntries        = 64
	MaxDiffCacheBytes          = 64 * 1024 * 1024
)

type Notice struct {
	Type        string
	Title       string
	Description string
	Timeout     int
}

type DiffTarget struct {
	Key          string
	Kind         string
	Path         string
	OriginalPath string
	Untracked    bool
	SHA          string
}

type DiffState struct {
	Target  *DiffTarget
	Diff    *git.Diff
	Loading bool
	Error   string
}

type HistoryState struct {
	Commits     []git.Commit
	Graph       []git.GraphRow
	Loading     bool
	Exhausted   bool
	AllBranches bool
}

type CommitDetailState struct {
	SHA          string
	Files        []git.CommitFile
	Loading      bool
	SelectedPath string
}

type BusyState struct {
	Label  string
	Cancel func()
}

type GenerationState struct {
	Agent  agent.AgentID
	Cancel func()
}

type Identity struct {
	Name  string
	Email string
}

type NumstatMaps struct {
	Unstaged map[string]git.NumstatEntry
	Staged   map[string]git.NumstatEntry
}

type Selection struct {
	Unstaged [][]int
	Staged   [][]int
}

type ActiveCell struct {
	List ListID
	Row  int
}

type HeadMessage struct {
	Subject string
	Body    string
}

type StoreOptions struct {
	Runner      *git.Runner
	Persistence *Persistence
	Trash       func(absolutePath string) error
}

type Store struct {
	runner      *git.Runner
	persistence *Persistence
	trash       func(string) error
	notifier    func(Notice)
	repo        *git.Repository
	main        *git.Repository
	stopWatch   func()
	mu          sync.Mutex

	Repository            reactive.Accessor[*git.Repository]
	setRepository         reactive.Setter[*git.Repository]
	MainRepository        reactive.Accessor[*git.Repository]
	setMain               reactive.Setter[*git.Repository]
	Opening               reactive.Accessor[string]
	setOpening            reactive.Setter[string]
	Worktrees             reactive.Accessor[[]git.Worktree]
	setWorktrees          reactive.Setter[[]git.Worktree]
	Status                reactive.Accessor[*git.RepositoryStatus]
	setStatus             reactive.Setter[*git.RepositoryStatus]
	Numstat               reactive.Accessor[NumstatMaps]
	setNumstat            reactive.Setter[NumstatMaps]
	Generation            reactive.Accessor[int]
	setGeneration         reactive.Setter[int]
	Refs                  reactive.Accessor[git.RefCollections]
	setRefs               reactive.Setter[git.RefCollections]
	Stashes               reactive.Accessor[[]git.StashEntry]
	setStashes            reactive.Setter[[]git.StashEntry]
	Remotes               reactive.Accessor[[]string]
	setRemotes            reactive.Setter[[]string]
	Busy                  reactive.Accessor[*BusyState]
	setBusy               reactive.Setter[*BusyState]
	View                  reactive.Accessor[ViewID]
	setViewSignal         reactive.Setter[ViewID]
	SidebarWidth          reactive.Accessor[float64]
	setSidebarWidth       reactive.Setter[float64]
	ChangesSplit          reactive.Accessor[float64]
	setChangesSplit       reactive.Setter[float64]
	HistorySplit          reactive.Accessor[float64]
	setHistorySplit       reactive.Setter[float64]
	HasHead               reactive.Accessor[bool]
	setHasHead            reactive.Setter[bool]
	Identity              reactive.Accessor[Identity]
	setIdentity           reactive.Setter[Identity]
	RepositoryName        reactive.Accessor[string]
	Unstaged              reactive.Accessor[[]git.ChangeItem]
	Staged                reactive.Accessor[[]git.ChangeItem]
	Conflicts             reactive.Accessor[int]
	ChangeCount           reactive.Accessor[int]
	Selection             reactive.Accessor[Selection]
	setSelectionAll       reactive.Setter[Selection]
	ActiveCell            reactive.Accessor[*ActiveCell]
	SetActiveCell         reactive.Setter[*ActiveCell]
	FocusedList           reactive.Accessor[ListID]
	SetFocusedList        reactive.Setter[ListID]
	ActiveItem            reactive.Accessor[*git.ChangeItem]
	Diff                  reactive.Accessor[DiffState]
	setDiff               reactive.Setter[DiffState]
	DiffSelection         reactive.Accessor[[][]int]
	SetDiffSelection      reactive.Setter[[][]int]
	DiffStats             reactive.Accessor[struct{ Added, Removed int }]
	SelectedDiffLineCount reactive.Accessor[int]
	Subject               reactive.Accessor[string]
	SetSubject            reactive.Setter[string]
	Body                  reactive.Accessor[string]
	SetBody               reactive.Setter[string]
	Amend                 reactive.Accessor[bool]
	setAmendSignal        reactive.Setter[bool]
	Committing            reactive.Accessor[bool]
	setCommitting         reactive.Setter[bool]
	CanCommit             reactive.Accessor[bool]
	HeadMessage           reactive.Accessor[*HeadMessage]
	setHeadMessage        reactive.Setter[*HeadMessage]
	Agents                reactive.Accessor[[]agent.Available]
	setAgents             reactive.Setter[[]agent.Available]
	PreferredAgent        reactive.Accessor[string]
	setPreferred          reactive.Setter[string]
	Generating            reactive.Accessor[*GenerationState]
	setGenerating         reactive.Setter[*GenerationState]
	History               reactive.Accessor[HistoryState]
	setHistory            reactive.Setter[HistoryState]
	HistorySelection      reactive.Accessor[[][]int]
	SetHistorySelection   reactive.Setter[[][]int]
	SelectedCommit        reactive.Accessor[*git.Commit]
	CommitDetail          reactive.Accessor[CommitDetailState]
	setCommitDetail       reactive.Setter[CommitDetailState]

	diffCache      map[string]*git.Diff
	diffCacheBytes int
	diffCancel     context.CancelFunc
	historyCancel  context.CancelFunc
	detailCancel   context.CancelFunc
	refreshing     bool
	refreshPending map[ChangeKind]struct{}
	disposeRoot    func()
}

func CreateStore(options StoreOptions) *Store {
	return reactive.CreateRoot(func(dispose func()) *Store {
		store := buildStore(options)
		store.disposeRoot = dispose
		return store
	})
}

func buildStore(options StoreOptions) *Store {
	persisted := options.Persistence.Current()
	s := &Store{
		runner:         options.Runner,
		persistence:    options.Persistence,
		trash:          options.Trash,
		notifier:       func(notice Notice) { fmt.Printf("[%s] %s\n", notice.Type, notice.Title) },
		diffCache:      map[string]*git.Diff{},
		refreshPending: map[ChangeKind]struct{}{},
	}
	s.Repository, s.setRepository = reactive.CreateSignal[*git.Repository](nil)
	s.MainRepository, s.setMain = reactive.CreateSignal[*git.Repository](nil)
	s.Opening, s.setOpening = reactive.CreateSignal("")
	s.Worktrees, s.setWorktrees = reactive.CreateSignal([]git.Worktree{})
	s.Status, s.setStatus = reactive.CreateSignal[*git.RepositoryStatus](nil)
	s.Numstat, s.setNumstat = reactive.CreateSignal(NumstatMaps{Unstaged: map[string]git.NumstatEntry{}, Staged: map[string]git.NumstatEntry{}})
	s.Generation, s.setGeneration = reactive.CreateSignal(0)
	s.Refs, s.setRefs = reactive.CreateSignal(git.RefCollections{})
	s.Stashes, s.setStashes = reactive.CreateSignal([]git.StashEntry{})
	s.Remotes, s.setRemotes = reactive.CreateSignal([]string{})
	s.Busy, s.setBusy = reactive.CreateSignal[*BusyState](nil)
	s.View, s.setViewSignal = reactive.CreateSignal(ViewChanges)
	s.SidebarWidth, s.setSidebarWidth = reactive.CreateSignal(persisted.SidebarWidth)
	s.ChangesSplit, s.setChangesSplit = reactive.CreateSignal(persisted.ChangesSplit)
	s.HistorySplit, s.setHistorySplit = reactive.CreateSignal(persisted.HistorySplit)
	s.HasHead, s.setHasHead = reactive.CreateSignal(true)
	s.Identity, s.setIdentity = reactive.CreateSignal(Identity{})
	s.Selection, s.setSelectionAll = reactive.CreateSignal(Selection{})
	s.ActiveCell, s.SetActiveCell = reactive.CreateSignal[*ActiveCell](nil)
	s.FocusedList, s.SetFocusedList = reactive.CreateSignal(ListUnstaged)
	s.Diff, s.setDiff = reactive.CreateSignal(DiffState{})
	s.DiffSelection, s.SetDiffSelection = reactive.CreateSignal([][]int{})
	s.Subject, s.SetSubject = reactive.CreateSignal("")
	s.Body, s.SetBody = reactive.CreateSignal("")
	s.Amend, s.setAmendSignal = reactive.CreateSignal(false)
	s.Committing, s.setCommitting = reactive.CreateSignal(false)
	s.HeadMessage, s.setHeadMessage = reactive.CreateSignal[*HeadMessage](nil)
	s.Agents, s.setAgents = reactive.CreateSignal([]agent.Available{})
	s.PreferredAgent, s.setPreferred = reactive.CreateSignal(persisted.PreferredAgent)
	s.Generating, s.setGenerating = reactive.CreateSignal[*GenerationState](nil)
	s.History, s.setHistory = reactive.CreateSignal(HistoryState{AllBranches: persisted.HistoryAllBranches})
	s.HistorySelection, s.SetHistorySelection = reactive.CreateSignal([][]int{})
	s.CommitDetail, s.setCommitDetail = reactive.CreateSignal(CommitDetailState{})

	s.RepositoryName = reactive.CreateMemo(func() string {
		if repo := s.Repository(); repo != nil {
			return filepath.Base(repo.Root())
		}
		return ""
	})
	s.Unstaged = reactive.CreateMemo(func() []git.ChangeItem {
		if status := s.Status(); status != nil {
			return git.UnstagedChanges(*status)
		}
		return nil
	})
	s.Staged = reactive.CreateMemo(func() []git.ChangeItem {
		if status := s.Status(); status != nil {
			return git.StagedChanges(*status)
		}
		return nil
	})
	s.Conflicts = reactive.CreateMemo(func() int {
		count := 0
		for _, item := range s.Unstaged() {
			if item.Conflicted {
				count++
			}
		}
		return count
	})
	s.ChangeCount = reactive.CreateMemo(func() int { return len(s.Unstaged()) + len(s.Staged()) })
	s.ActiveItem = reactive.CreateMemo(func() *git.ChangeItem {
		lists := []ListID{ListUnstaged, ListStaged}
		if s.FocusedList() == ListStaged {
			lists = []ListID{ListStaged, ListUnstaged}
		}
		for _, list := range lists {
			selected := s.SelectedItems(list)
			if len(selected) == 0 {
				continue
			}
			if cell := s.ActiveCell(); cell != nil && cell.List == list && cell.Row >= 0 {
				items := s.listItems(list)
				if cell.Row < len(items) {
					item := items[cell.Row]
					for _, candidate := range selected {
						if candidate.ID == item.ID {
							copy := item
							return &copy
						}
					}
				}
			}
			copy := selected[0]
			return &copy
		}
		if items := s.Unstaged(); len(items) > 0 {
			copy := items[0]
			return &copy
		}
		if items := s.Staged(); len(items) > 0 {
			copy := items[0]
			return &copy
		}
		return nil
	})
	s.DiffStats = reactive.CreateMemo(func() struct{ Added, Removed int } {
		if parsed := s.Diff().Diff; parsed != nil {
			return struct{ Added, Removed int }{parsed.Added, parsed.Removed}
		}
		return struct{ Added, Removed int }{}
	})
	s.SelectedDiffLineCount = reactive.CreateMemo(func() int {
		count := 0
		if parsed := s.Diff().Diff; parsed != nil {
			for _, group := range parsed.SelectedLines(s.DiffSelection()) {
				count += len(group.Lines)
			}
		}
		return count
	})
	s.CanCommit = reactive.CreateMemo(func() bool {
		if s.Committing() || s.Repository() == nil || strings.TrimSpace(s.Subject()) == "" {
			return false
		}
		if s.Conflicts() > 0 && len(s.Staged()) == 0 {
			return false
		}
		return len(s.Staged()) > 0 || s.Amend()
	})
	s.SelectedCommit = reactive.CreateMemo(func() *git.Commit {
		ranges := s.HistorySelection()
		if len(ranges) == 0 || len(ranges[0]) == 0 {
			return nil
		}
		row := ranges[0][0]
		commits := s.History().Commits
		if row < 0 || row >= len(commits) {
			return nil
		}
		copy := commits[row]
		return &copy
	})

	reactive.CreateEffect(func() {
		sha := ""
		if commit := s.SelectedCommit(); commit != nil {
			sha = commit.Sha
		}
		go s.loadCommitDetail(sha)
	})
	reactive.CreateEffect(func() {
		target := s.diffTarget()
		go s.loadDiff(target)
	})

	go func() {
		found := agent.DetectAgents()
		native.Dispatch(func() {
			s.setAgents(found)
			preferred := s.PreferredAgent()
			has := false
			for _, item := range found {
				if string(item.ID) == preferred {
					has = true
					break
				}
			}
			if !has && len(found) > 0 {
				s.setPreferred(string(found[0].ID))
			}
		})
	}()
	return s
}

func (s *Store) RecentRepositories() []string {
	return s.persistence.State().RecentRepositories
}

func (s *Store) ListItems(list ListID) []git.ChangeItem {
	if list == ListStaged {
		return s.Staged()
	}
	return s.Unstaged()
}

func (s *Store) listItems(list ListID) []git.ChangeItem {
	return s.ListItems(list)
}

func (s *Store) SelectedItems(list ListID) []git.ChangeItem {
	items := s.listItems(list)
	rows := map[int]struct{}{}
	selection := s.Selection()
	ranges := selection.Unstaged
	if list == ListStaged {
		ranges = selection.Staged
	}
	for _, r := range ranges {
		if len(r) < 2 {
			continue
		}
		for row := r[0]; row <= r[1]; row++ {
			rows[row] = struct{}{}
		}
	}
	var selected []git.ChangeItem
	for index, item := range items {
		if _, ok := rows[index]; ok {
			selected = append(selected, item)
		}
	}
	return selected
}

func (s *Store) SetSelection(list ListID, ranges [][]int) {
	current := s.Selection()
	if list == ListUnstaged {
		current.Unstaged = ranges
	} else {
		current.Staged = ranges
	}
	s.setSelectionAll(current)
}

func (s *Store) SelectChange(list ListID, index int) {
	s.SetSelection(list, [][]int{{index, index}})
	s.SetActiveCell(&ActiveCell{List: list, Row: index})
	s.SetFocusedList(list)
}

func (s *Store) DiffRowCount() int {
	if parsed := s.Diff().Diff; parsed != nil {
		return parsed.RowCount
	}
	return 0
}

func (s *Store) DiffRows() []git.DiffRow {
	if parsed := s.Diff().Diff; parsed != nil {
		return parsed.Rows
	}
	return nil
}

func (s *Store) diffTarget() *DiffTarget {
	if s.View() == ViewHistory {
		return s.commitDiffTarget()
	}
	item := s.ActiveItem()
	if item == nil {
		return nil
	}
	kind := "unstaged"
	if item.Staged {
		kind = "staged"
	}
	return &DiffTarget{
		Key:  fmt.Sprintf("%s:%s:%d", kind, item.Path, s.Generation()),
		Kind: kind, Path: item.Path, OriginalPath: item.OriginalPath,
		Untracked: item.Entry.Kind == git.KindUntracked,
	}
}

func (s *Store) commitDiffTarget() *DiffTarget {
	detail := s.CommitDetail()
	commit := s.SelectedCommit()
	if commit == nil || detail.SHA != commit.Sha {
		return nil
	}
	path := detail.SelectedPath
	if path == "" && len(detail.Files) > 0 {
		path = detail.Files[0].Path
	}
	if path == "" {
		return nil
	}
	original := ""
	for _, file := range detail.Files {
		if file.Path == path {
			original = file.OriginalPath
			break
		}
	}
	return &DiffTarget{Key: "commit:" + commit.Sha + ":" + path, Kind: "commit", Path: path, OriginalPath: original, SHA: commit.Sha}
}

func (s *Store) cacheDiff(key string, parsed *git.Diff) {
	if previous, ok := s.diffCache[key]; ok {
		delete(s.diffCache, key)
		s.diffCacheBytes -= previous.Bytes
	}
	s.diffCache[key] = parsed
	s.diffCacheBytes += parsed.Bytes
	for oldKey, old := range s.diffCache {
		if len(s.diffCache) <= MaxDiffCacheEntries && s.diffCacheBytes <= MaxDiffCacheBytes {
			break
		}
		if oldKey == key {
			continue
		}
		delete(s.diffCache, oldKey)
		s.diffCacheBytes -= old.Bytes
	}
}

func (s *Store) clearDiffCache() {
	s.diffCache = map[string]*git.Diff{}
	s.diffCacheBytes = 0
}

func (s *Store) loadDiff(target *DiffTarget) {
	if s.diffCancel != nil {
		s.diffCancel()
		s.diffCancel = nil
	}
	if target == nil {
		native.Dispatch(func() { s.setDiff(DiffState{}) })
		return
	}
	if cached, ok := s.diffCache[target.Key]; ok {
		copy := *target
		native.Dispatch(func() {
			s.cacheDiff(target.Key, cached)
			s.setDiff(DiffState{Target: &copy, Diff: cached})
		})
		return
	}
	s.mu.Lock()
	repo := s.repo
	s.mu.Unlock()
	if repo == nil {
		return
	}
	ctx, cancel := context.WithCancel(context.Background())
	s.diffCancel = cancel
	copy := *target
	native.Dispatch(func() {
		current := s.Diff()
		next := DiffState{Target: &copy, Loading: true}
		if current.Target != nil && current.Target.Path == target.Path {
			next.Diff = current.Diff
		}
		s.setDiff(next)
	})
	var parsed *git.Diff
	var err error
	switch {
	case target.Kind == "commit":
		parsed, err = repo.DiffCommit(ctx, target.SHA, target.Path)
	case target.Untracked:
		parsed, err = repo.DiffUntracked(ctx, target.Path)
	case target.Kind == "staged":
		parsed, err = repo.DiffIndex(ctx, target.Path, target.OriginalPath)
	default:
		parsed, err = repo.DiffWorkingTree(ctx, target.Path)
	}
	if ctx.Err() != nil {
		return
	}
	native.Dispatch(func() {
		if err != nil {
			s.setDiff(DiffState{Target: &copy, Error: DescribeError(err)})
			return
		}
		s.cacheDiff(target.Key, parsed)
		s.setDiff(DiffState{Target: &copy, Diff: parsed})
		s.SetDiffSelection(nil)
	})
}

func (s *Store) loadCommitDetail(sha string) {
	if s.detailCancel != nil {
		s.detailCancel()
		s.detailCancel = nil
	}
	s.mu.Lock()
	repo := s.repo
	s.mu.Unlock()
	if sha == "" || repo == nil {
		native.Dispatch(func() { s.setCommitDetail(CommitDetailState{}) })
		return
	}
	ctx, cancel := context.WithCancel(context.Background())
	s.detailCancel = cancel
	native.Dispatch(func() { s.setCommitDetail(CommitDetailState{SHA: sha, Loading: true}) })
	files, err := repo.CommitFiles(ctx, sha)
	if ctx.Err() != nil {
		return
	}
	native.Dispatch(func() {
		if err != nil {
			s.setCommitDetail(CommitDetailState{SHA: sha})
			s.Notify(Notice{Type: "error", Title: "Unable to read commit", Description: DescribeError(err)})
			return
		}
		s.setCommitDetail(CommitDetailState{SHA: sha, Files: files})
	})
}

func (s *Store) LoadHistory(reset bool) {
	s.mu.Lock()
	repo := s.repo
	s.mu.Unlock()
	if repo == nil {
		return
	}
	current := reactive.Untrack(s.History)
	if !reset && (current.Loading || current.Exhausted) {
		return
	}
	if s.historyCancel != nil {
		s.historyCancel()
	}
	ctx, cancel := context.WithCancel(context.Background())
	s.historyCancel = cancel
	native.Dispatch(func() {
		state := s.History()
		state.Loading = true
		if reset {
			state.Exhausted = false
		}
		s.setHistory(state)
	})
	skip := 0
	if !reset {
		skip = len(current.Commits)
	}
	page, err := repo.Log(ctx, git.LogOptions{Limit: HistoryPage, Skip: skip, All: current.AllBranches})
	if ctx.Err() != nil {
		return
	}
	native.Dispatch(func() {
		state := s.History()
		if err != nil {
			state.Loading = false
			s.setHistory(state)
			s.Notify(Notice{Type: "error", Title: "Unable to load history", Description: DescribeError(err)})
			return
		}
		commits := page
		if !reset {
			commits = append(append([]git.Commit{}, current.Commits...), page...)
		}
		state.Commits = commits
		state.Graph = git.LayoutGraph(commits)
		state.Loading = false
		state.Exhausted = len(page) < HistoryPage
		s.setHistory(state)
		if reset {
			previous := ""
			if commit := s.SelectedCommit(); commit != nil {
				previous = commit.Sha
			}
			index := -1
			if previous != "" {
				for i, commit := range commits {
					if commit.Sha == previous {
						index = i
						break
					}
				}
			}
			if index >= 0 {
				s.SetHistorySelection([][]int{{index, index}})
			} else if len(commits) > 0 {
				s.SetHistorySelection([][]int{{0, 0}})
			} else {
				s.SetHistorySelection(nil)
			}
		}
	})
}

func (s *Store) Refresh() {
	s.refresh(map[ChangeKind]struct{}{ChangeWorktree: {}, ChangeIndex: {}, ChangeRefs: {}})
}

func (s *Store) refresh(kinds map[ChangeKind]struct{}) {
	s.mu.Lock()
	if s.refreshing {
		for kind := range kinds {
			s.refreshPending[kind] = struct{}{}
		}
		s.mu.Unlock()
		return
	}
	s.refreshing = true
	repo := s.repo
	s.mu.Unlock()
	if repo == nil {
		s.mu.Lock()
		s.refreshing = false
		s.mu.Unlock()
		return
	}
	go func() {
		ctx := context.Background()
		status, err := repo.Status(ctx)
		unstagedStats, _ := repo.Numstat(ctx, false)
		stagedStats, _ := repo.Numstat(ctx, true)
		head, _ := repo.HasHead(ctx)
		var refs *git.RefCollections
		var stashes []git.StashEntry
		var worktrees []git.Worktree
		var remotes []string
		wantsRefs := false
		if _, ok := kinds[ChangeRefs]; ok {
			wantsRefs = true
			if value, e := repo.Refs(ctx); e == nil {
				refs = &value
			}
			stashes, _ = repo.Stashes(ctx)
			worktrees, _ = repo.Worktrees(ctx)
			remotes, _ = repo.Remotes(ctx)
		}
		native.Dispatch(func() {
			s.mu.Lock()
			same := s.repo == repo
			s.mu.Unlock()
			if err != nil {
				if !errors.Is(err, context.Canceled) {
					s.Notify(Notice{Type: "error", Title: "Unable to refresh", Description: DescribeError(err)})
				}
			} else if same {
				s.setStatus(&status)
				unstaged := map[string]git.NumstatEntry{}
				staged := map[string]git.NumstatEntry{}
				for _, entry := range unstagedStats {
					unstaged[entry.Path] = entry
				}
				for _, entry := range stagedStats {
					staged[entry.Path] = entry
				}
				s.setNumstat(NumstatMaps{Unstaged: unstaged, Staged: staged})
				s.setHasHead(head)
				if refs != nil {
					s.setRefs(*refs)
				}
				if wantsRefs {
					s.setStashes(stashes)
					s.setWorktrees(worktrees)
					s.setRemotes(remotes)
				}
				s.clearDiffCache()
				s.setGeneration(s.Generation() + 1)
				s.reconcileSelection(status)
				if wantsRefs {
					go s.LoadHistory(true)
					go s.loadHeadMessage(repo)
				}
			}
			s.mu.Lock()
			s.refreshing = false
			pending := s.refreshPending
			s.refreshPending = map[ChangeKind]struct{}{}
			s.mu.Unlock()
			if len(pending) > 0 {
				s.refresh(pending)
			}
		})
	}()
}

func (s *Store) reconcileSelection(next git.RepositoryStatus) {
	previousUnstaged := pathsOf(s.SelectedItems(ListUnstaged))
	previousStaged := pathsOf(s.SelectedItems(ListStaged))
	s.setSelectionAll(Selection{
		Unstaged: rangesFor(git.UnstagedChanges(next), previousUnstaged),
		Staged:   rangesFor(git.StagedChanges(next), previousStaged),
	})
}

func (s *Store) loadHeadMessage(repo *git.Repository) {
	commits, err := repo.Log(context.Background(), git.LogOptions{Limit: 1})
	native.Dispatch(func() {
		if err != nil || len(commits) == 0 {
			s.setHeadMessage(nil)
			return
		}
		s.setHeadMessage(&HeadMessage{Subject: commits[0].Subject, Body: commits[0].Body})
	})
}

func (s *Store) OpenRepository(path string) {
	target := CanonicalPath(path)
	native.Dispatch(func() { s.setOpening(target) })
	go func() {
		repo, err := git.Open(context.Background(), s.runner, target, s.trash)
		native.Dispatch(func() {
			s.setOpening("")
			if err != nil {
				s.Notify(Notice{Type: "error", Title: "Unable to open " + filepath.Base(target), Description: DescribeError(err)})
				recent := filterOut(s.persistence.Current().RecentRepositories, target)
				s.persist(PersistedPatch{RecentRepositories: &recent})
				return
			}
			s.activate(repo, repo)
			last := repo.Root()
			recent := RememberRepository(s.persistence.Current().RecentRepositories, repo.Root())
			s.persist(PersistedPatch{LastRepository: &last, RecentRepositories: &recent})
		})
	}()
}

func (s *Store) activate(repo, main *git.Repository) {
	if s.stopWatch != nil {
		s.stopWatch()
		s.stopWatch = nil
	}
	if s.diffCancel != nil {
		s.diffCancel()
	}
	if s.historyCancel != nil {
		s.historyCancel()
	}
	s.mu.Lock()
	s.repo = repo
	s.main = main
	s.mu.Unlock()
	s.setRepository(repo)
	s.setMain(main)
	s.setStatus(nil)
	s.setSelectionAll(Selection{})
	s.SetActiveCell(nil)
	s.setDiff(DiffState{})
	s.SetDiffSelection(nil)
	history := s.History()
	history.Commits = nil
	history.Graph = nil
	history.Loading = false
	history.Exhausted = false
	s.setHistory(history)
	s.SetHistorySelection(nil)
	s.setCommitDetail(CommitDetailState{})
	s.SetSubject("")
	s.SetBody("")
	s.setAmendSignal(false)
	s.clearDiffCache()
	s.stopWatch = WatchRepository(WatcherOptions{
		Root: repo.Root(), GitDir: repo.Info.GitDir, CommonDir: repo.Info.CommonDir,
		OnChange: func(kinds map[ChangeKind]struct{}) { s.refresh(kinds) },
	})
	go func() {
		name, _ := repo.Config(context.Background(), "user.name")
		email, _ := repo.Config(context.Background(), "user.email")
		native.Dispatch(func() {
			s.mu.Lock()
			same := s.repo == repo
			s.mu.Unlock()
			if same {
				s.setIdentity(Identity{Name: name, Email: email})
			}
		})
	}()
	s.refresh(map[ChangeKind]struct{}{ChangeWorktree: {}, ChangeIndex: {}, ChangeRefs: {}})
}

func (s *Store) CloseRepository() {
	if s.stopWatch != nil {
		s.stopWatch()
		s.stopWatch = nil
	}
	if s.diffCancel != nil {
		s.diffCancel()
	}
	if s.historyCancel != nil {
		s.historyCancel()
	}
	s.mu.Lock()
	s.repo = nil
	s.main = nil
	s.mu.Unlock()
	s.setRepository(nil)
	s.setMain(nil)
	s.setStatus(nil)
	s.setWorktrees(nil)
	s.setRefs(git.RefCollections{})
	s.setStashes(nil)
	s.setDiff(DiffState{})
	history := s.History()
	history.Commits = nil
	history.Graph = nil
	s.setHistory(history)
	s.SetHistorySelection(nil)
	s.SetView(ViewChanges)
	s.persist(PersistedPatch{ForgetLast: true})
}

func (s *Store) SelectWorktree(path string) {
	s.mu.Lock()
	main := s.main
	current := s.repo
	s.mu.Unlock()
	if main == nil || (current != nil && current.Root() == path) {
		return
	}
	go func() {
		var repo *git.Repository
		var err error
		if path == main.Root() {
			repo = main
		} else {
			repo, err = main.Worktree(context.Background(), path)
		}
		native.Dispatch(func() {
			if err != nil {
				s.Notify(Notice{Type: "error", Title: "Unable to open worktree", Description: DescribeError(err)})
				return
			}
			s.activate(repo, main)
			s.SetView(ViewChanges)
		})
	}()
}

func (s *Store) operation(label string, run func(*git.Repository, context.Context) error, refreshRefs bool, success *Notice) {
	s.mu.Lock()
	repo := s.repo
	s.mu.Unlock()
	if repo == nil {
		return
	}
	ctx, cancel := context.WithCancel(context.Background())
	native.Dispatch(func() { s.setBusy(&BusyState{Label: label, Cancel: cancel}) })
	go func() {
		err := run(repo, ctx)
		native.Dispatch(func() {
			s.setBusy(nil)
			if err != nil {
				if errors.Is(err, context.Canceled) {
					s.Notify(Notice{Type: "info", Title: label + " cancelled"})
				} else {
					s.Notify(Notice{Type: "error", Title: label + " failed", Description: DescribeError(err), Timeout: 9000})
				}
			} else if success != nil {
				s.Notify(*success)
			}
			kinds := map[ChangeKind]struct{}{ChangeWorktree: {}, ChangeIndex: {}}
			if refreshRefs {
				kinds[ChangeRefs] = struct{}{}
			}
			s.refresh(kinds)
		})
	}()
}

func (s *Store) StageItems(items []git.ChangeItem) {
	if len(items) == 0 {
		return
	}
	paths := make([]string, len(items))
	for i, item := range items {
		paths[i] = item.Path
	}
	s.operation("Stage", func(repo *git.Repository, ctx context.Context) error { return repo.Stage(ctx, paths) }, false, nil)
}

func (s *Store) UnstageItems(items []git.ChangeItem) {
	if len(items) == 0 {
		return
	}
	var paths []string
	for _, item := range items {
		if item.OriginalPath != "" {
			paths = append(paths, item.OriginalPath)
		}
		paths = append(paths, item.Path)
	}
	s.operation("Unstage", func(repo *git.Repository, ctx context.Context) error { return repo.Unstage(ctx, paths) }, false, nil)
}

func (s *Store) DiscardItems(items []git.ChangeItem) {
	if len(items) == 0 {
		return
	}
	var tracked, untracked []string
	for _, item := range items {
		if item.Entry.Kind == git.KindUntracked {
			untracked = append(untracked, item.Path)
		} else {
			tracked = append(tracked, item.Path)
		}
	}
	title := fmt.Sprintf("Discarded %d files", len(items))
	if len(items) == 1 {
		title = "Discarded " + filepath.Base(items[0].Path)
	}
	s.operation("Discard", func(repo *git.Repository, ctx context.Context) error {
		return repo.Discard(ctx, tracked, untracked)
	}, false, &Notice{Type: "info", Title: title})
}

func (s *Store) ToggleStaging(list ListID) {
	items := s.SelectedItems(list)
	if len(items) == 0 {
		if cell := s.ActiveCell(); cell != nil && cell.List == list && cell.Row >= 0 {
			all := s.listItems(list)
			if cell.Row < len(all) {
				items = []git.ChangeItem{all[cell.Row]}
			}
		}
	}
	if list == ListUnstaged {
		s.StageItems(items)
		return
	}
	s.UnstageItems(items)
}

func (s *Store) StageAll() {
	s.operation("Stage all", func(repo *git.Repository, ctx context.Context) error { return repo.StageAll(ctx) }, false, nil)
}

func (s *Store) UnstageAll() {
	s.operation("Unstage all", func(repo *git.Repository, ctx context.Context) error { return repo.UnstageAll(ctx) }, false, nil)
}

func (s *Store) applyHunk(action string, selections []git.HunkSelection, fileIndex int) {
	file := s.Diff().Diff.FileAt(fileIndex)
	if file == nil || len(selections) == 0 {
		return
	}
	label := "Stage lines"
	if action == "unstage" {
		label = "Unstage lines"
	} else if action == "discard" {
		label = "Discard lines"
	}
	copyFile := *file
	s.operation(label, func(repo *git.Repository, ctx context.Context) error {
		switch action {
		case "unstage":
			return repo.UnstagePatch(ctx, copyFile, selections)
		case "discard":
			return repo.DiscardPatch(ctx, copyFile, selections)
		default:
			return repo.StagePatch(ctx, copyFile, selections)
		}
	}, false, nil)
}

func (s *Store) StageHunk(fileIndex, hunkIndex int) {
	s.applyHunk("stage", []git.HunkSelection{{HunkIndex: hunkIndex}}, fileIndex)
}

func (s *Store) UnstageHunk(fileIndex, hunkIndex int) {
	s.applyHunk("unstage", []git.HunkSelection{{HunkIndex: hunkIndex}}, fileIndex)
}

func (s *Store) DiscardHunk(fileIndex, hunkIndex int) {
	s.applyHunk("discard", []git.HunkSelection{{HunkIndex: hunkIndex}}, fileIndex)
}

func (s *Store) ApplySelectedLines(action string) {
	parsed := s.Diff().Diff
	if parsed == nil {
		return
	}
	groups := parsed.SelectedLines(s.DiffSelection())
	byFile := map[int][]git.HunkSelection{}
	for _, group := range groups {
		byFile[group.FileIndex] = append(byFile[group.FileIndex], git.HunkSelection{HunkIndex: group.HunkIndex, Lines: group.Lines})
	}
	for fileIndex, selections := range byFile {
		s.applyHunk(action, selections, fileIndex)
	}
	s.SetDiffSelection(nil)
}

func (s *Store) Commit() {
	if !s.CanCommit() {
		return
	}
	message := strings.TrimSpace(s.Subject()) + "\n\n" + strings.TrimSpace(s.Body())
	amending := s.Amend()
	label := "Commit"
	title := "Committed “" + truncate(strings.TrimSpace(s.Subject()), 60) + "”"
	if amending {
		label = "Amend"
		title = "Commit amended"
	}
	s.setCommitting(true)
	s.operation(label, func(repo *git.Repository, ctx context.Context) error {
		return repo.Commit(ctx, strings.TrimSpace(message), git.CommitOptions{Amend: amending})
	}, true, &Notice{Type: "success", Title: title})
	s.SetSubject("")
	s.SetBody("")
	s.setAmendSignal(false)
	s.setCommitting(false)
}

func (s *Store) SetAmend(value bool) {
	s.setAmendSignal(value)
	if value {
		if head := s.HeadMessage(); head != nil && strings.TrimSpace(s.Subject()) == "" && strings.TrimSpace(s.Body()) == "" {
			s.SetSubject(head.Subject)
			s.SetBody(head.Body)
		}
	}
}

func (s *Store) GenerateMessage(agentID string) {
	s.mu.Lock()
	repo := s.repo
	s.mu.Unlock()
	if repo == nil {
		return
	}
	agents := s.Agents()
	var chosen *agent.Available
	want := agentID
	if want == "" {
		want = s.PreferredAgent()
	}
	for i := range agents {
		if string(agents[i].ID) == want {
			chosen = &agents[i]
			break
		}
	}
	if chosen == nil && len(agents) > 0 {
		chosen = &agents[0]
	}
	if chosen == nil {
		s.Notify(Notice{Type: "warning", Title: "No agent CLI found", Description: "Install Codex (codex) or Claude Code (claude) to generate commit messages."})
		return
	}
	if current := s.Generating(); current != nil {
		current.Cancel()
	}
	ctx, cancel := context.WithCancel(context.Background())
	s.setGenerating(&GenerationState{Agent: chosen.ID, Cancel: cancel})
	s.SetPreferredAgent(string(chosen.ID))
	useStaged := len(s.Staged()) > 0
	items := s.Unstaged()
	if useStaged {
		items = s.Staged()
	}
	if len(items) > 200 {
		items = items[:200]
	}
	name := s.RepositoryName()
	branch := ""
	if status := s.Status(); status != nil {
		branch = status.Branch
	}
	amending := ""
	if s.Amend() {
		if head := s.HeadMessage(); head != nil {
			amending = strings.TrimSpace(head.Subject + "\n\n" + head.Body)
		}
	}
	files := make([]string, len(items))
	for i, item := range items {
		files[i] = string(item.Code) + " " + item.Path
	}
	go func() {
		var diffs []string
		for _, item := range items {
			var parsed *git.Diff
			var err error
			if item.Entry.Kind == git.KindUntracked {
				parsed, err = repo.DiffUntracked(ctx, item.Path)
			} else if useStaged {
				parsed, err = repo.DiffIndex(ctx, item.Path, item.OriginalPath)
			} else {
				parsed, err = repo.DiffWorkingTree(ctx, item.Path)
			}
			if err != nil || ctx.Err() != nil {
				continue
			}
			diffs = append(diffs, parsed.Render())
		}
		subjects, _ := repo.RecentSubjects(ctx, 15)
		message, err := agent.Generate(ctx, *chosen, agent.Request{
			RepositoryName: name, Branch: branch, Diff: strings.Join(diffs, "\n"),
			Files: files, RecentSubjects: subjects, Amending: amending,
		}, repo.Root())
		native.Dispatch(func() {
			s.setGenerating(nil)
			if err != nil {
				if errors.Is(err, context.Canceled) {
					s.Notify(Notice{Type: "info", Title: "Generation cancelled", Timeout: 2000})
					return
				}
				s.Notify(Notice{Type: "error", Title: "Unable to generate a message", Description: DescribeError(err), Timeout: 9000})
				return
			}
			s.SetSubject(message.Subject)
			s.SetBody(message.Body)
			extra := fmt.Sprintf("%.1fs", float64(message.DurationMs)/1000)
			if !useStaged {
				extra += " · from unstaged changes"
			}
			s.Notify(Notice{Type: "success", Title: chosen.Label + " wrote a commit message", Description: extra, Timeout: 3000})
		})
	}()
}

func (s *Store) CancelGeneration() {
	if current := s.Generating(); current != nil {
		current.Cancel()
	}
	s.setGenerating(nil)
}

func (s *Store) SetPreferredAgent(id string) {
	s.setPreferred(id)
	s.persist(PersistedPatch{PreferredAgent: &id})
}

func (s *Store) Fetch() {
	s.operation("Fetch", func(repo *git.Repository, ctx context.Context) error { return repo.Fetch(ctx) }, true, &Notice{Type: "success", Title: "Fetched", Timeout: 2500})
}

func (s *Store) Pull() {
	s.operation("Pull", func(repo *git.Repository, ctx context.Context) error { return repo.Pull(ctx) }, true, &Notice{Type: "success", Title: "Pulled", Timeout: 2500})
}

func (s *Store) Push() {
	status := s.Status()
	remote := "origin"
	if remotes := s.Remotes(); len(remotes) > 0 {
		remote = remotes[0]
	}
	branch := ""
	setUpstream := false
	if status != nil {
		branch = status.Branch
		setUpstream = branch != "" && status.Upstream == ""
	}
	title := "Pushed"
	if setUpstream {
		title = "Pushed and set upstream to " + remote + "/" + branch
	}
	s.operation("Push", func(repo *git.Repository, ctx context.Context) error {
		return repo.Push(ctx, remote, branch, setUpstream, false)
	}, true, &Notice{Type: "success", Title: title, Timeout: 2500})
}

func (s *Store) SwitchBranch(name string) {
	s.operation("Switch branch", func(repo *git.Repository, ctx context.Context) error { return repo.SwitchBranch(ctx, name) }, true, &Notice{Type: "success", Title: "Switched to " + name, Timeout: 2500})
}

func (s *Store) CreateBranch(name, from string, checkout bool) {
	s.operation("Create branch", func(repo *git.Repository, ctx context.Context) error {
		return repo.CreateBranch(ctx, name, from, checkout)
	}, true, &Notice{Type: "success", Title: "Created " + name, Timeout: 2500})
}

func (s *Store) DeleteBranch(name string, force bool) {
	s.operation("Delete branch", func(repo *git.Repository, ctx context.Context) error {
		return repo.DeleteBranch(ctx, name, force)
	}, true, &Notice{Type: "info", Title: "Deleted " + name, Timeout: 2500})
}

func (s *Store) CheckoutCommit(sha string) {
	short := sha
	if len(short) > 7 {
		short = short[:7]
	}
	s.operation("Checkout", func(repo *git.Repository, ctx context.Context) error {
		return repo.CheckoutCommit(ctx, sha)
	}, true, &Notice{Type: "success", Title: "Checked out " + short + " (detached)", Timeout: 2500})
}

func (s *Store) StashPush(message string, includeUntracked bool) {
	s.operation("Stash", func(repo *git.Repository, ctx context.Context) error {
		return repo.StashPush(ctx, message, includeUntracked, false)
	}, true, &Notice{Type: "success", Title: "Changes stashed", Timeout: 2500})
}

func (s *Store) StashApply(ref string) {
	s.operation("Apply stash", func(repo *git.Repository, ctx context.Context) error { return repo.StashApply(ctx, ref) }, true, nil)
}

func (s *Store) StashPop(ref string) {
	s.operation("Pop stash", func(repo *git.Repository, ctx context.Context) error { return repo.StashPop(ctx, ref) }, true, nil)
}

func (s *Store) StashDrop(ref string) {
	s.operation("Drop stash", func(repo *git.Repository, ctx context.Context) error { return repo.StashDrop(ctx, ref) }, true, nil)
}

func (s *Store) AddWorktree(path, newBranch, branch, base string) {
	s.mu.Lock()
	main := s.main
	s.mu.Unlock()
	if main == nil {
		return
	}
	ctx, cancel := context.WithCancel(context.Background())
	native.Dispatch(func() { s.setBusy(&BusyState{Label: "Add worktree", Cancel: cancel}) })
	go func() {
		err := main.AddWorktree(ctx, path, newBranch, branch, base)
		native.Dispatch(func() {
			s.setBusy(nil)
			if err != nil {
				s.Notify(Notice{Type: "error", Title: "Unable to add worktree", Description: DescribeError(err), Timeout: 9000})
			} else {
				s.Notify(Notice{Type: "success", Title: "Added worktree at " + filepath.Base(path), Timeout: 3000})
			}
			s.refresh(map[ChangeKind]struct{}{ChangeWorktree: {}, ChangeIndex: {}, ChangeRefs: {}})
		})
	}()
}

func (s *Store) RemoveWorktree(path string, force bool) {
	s.mu.Lock()
	main := s.main
	current := s.repo
	s.mu.Unlock()
	if main == nil {
		return
	}
	if current != nil && current.Root() == path {
		s.SelectWorktree(main.Root())
	}
	ctx, cancel := context.WithCancel(context.Background())
	native.Dispatch(func() { s.setBusy(&BusyState{Label: "Remove worktree", Cancel: cancel}) })
	go func() {
		err := main.RemoveWorktree(ctx, path, force)
		native.Dispatch(func() {
			s.setBusy(nil)
			if err != nil {
				s.Notify(Notice{Type: "error", Title: "Unable to remove worktree", Description: DescribeError(err), Timeout: 9000})
			} else {
				s.Notify(Notice{Type: "info", Title: "Removed worktree " + filepath.Base(path), Timeout: 3000})
			}
			s.refresh(map[ChangeKind]struct{}{ChangeWorktree: {}, ChangeIndex: {}, ChangeRefs: {}})
		})
	}()
}

func (s *Store) SuggestWorktreePath(branch string) string {
	s.mu.Lock()
	root := ""
	if s.main != nil {
		root = s.main.Root()
	} else if s.repo != nil {
		root = s.repo.Root()
	}
	s.mu.Unlock()
	if root == "" {
		root, _ = os.Getwd()
	}
	return git.SuggestWorktreePath(root, branch)
}

func (s *Store) SetView(next ViewID) {
	s.setViewSignal(next)
	if next == ViewHistory && len(s.History().Commits) == 0 {
		go s.LoadHistory(true)
	}
}

func (s *Store) SetHistoryAllBranches(all bool) {
	state := s.History()
	state.AllBranches = all
	s.setHistory(state)
	s.persist(PersistedPatch{HistoryAllBranches: &all})
	go s.LoadHistory(true)
}

func (s *Store) SelectCommit(index int) {
	s.SetHistorySelection([][]int{{index, index}})
}

func (s *Store) SelectCommitFile(path string) {
	detail := s.CommitDetail()
	detail.SelectedPath = path
	s.setCommitDetail(detail)
}

func (s *Store) SetSidebarWidth(width float64) {
	if width < 180 {
		width = 180
	}
	if width > 420 {
		width = 420
	}
	s.setSidebarWidth(width)
	s.persist(PersistedPatch{SidebarWidth: &width})
}

func (s *Store) SetChangesSplit(width float64) {
	if width < 220 {
		width = 220
	}
	if width > 800 {
		width = 800
	}
	s.setChangesSplit(width)
	s.persist(PersistedPatch{ChangesSplit: &width})
}

func (s *Store) SetHistorySplit(width float64) {
	if width < 260 {
		width = 260
	}
	if width > 1000 {
		width = 1000
	}
	s.setHistorySplit(width)
	s.persist(PersistedPatch{HistorySplit: &width})
}

func (s *Store) persist(patch PersistedPatch) {
	s.persistence.Update(patch)
}

func (s *Store) Notify(notice Notice) {
	s.notifier(notice)
}

func (s *Store) SetNotifier(next func(Notice)) {
	s.notifier = next
}

func (s *Store) CancelBusy() {
	if busy := s.Busy(); busy != nil && busy.Cancel != nil {
		busy.Cancel()
	}
}

func (s *Store) Dispose() {
	if s.diffCancel != nil {
		s.diffCancel()
	}
	if s.historyCancel != nil {
		s.historyCancel()
	}
	if s.detailCancel != nil {
		s.detailCancel()
	}
	s.clearDiffCache()
	if s.stopWatch != nil {
		s.stopWatch()
	}
	if current := s.Generating(); current != nil {
		current.Cancel()
	}
	if busy := s.Busy(); busy != nil && busy.Cancel != nil {
		busy.Cancel()
	}
	if s.disposeRoot != nil {
		s.disposeRoot()
	}
}

func DescribeError(err error) string {
	if err == nil {
		return ""
	}
	if gitErr, ok := err.(*git.Error); ok {
		return gitErr.Summary()
	}
	return err.Error()
}

func pathsOf(items []git.ChangeItem) []string {
	paths := make([]string, len(items))
	for i, item := range items {
		paths[i] = item.Path
	}
	return paths
}

func rangesFor(items []git.ChangeItem, paths []string) [][]int {
	if len(paths) == 0 {
		return nil
	}
	wanted := map[string]struct{}{}
	for _, path := range paths {
		wanted[path] = struct{}{}
	}
	var ranges [][]int
	for index, item := range items {
		if _, ok := wanted[item.Path]; !ok {
			continue
		}
		if len(ranges) > 0 && ranges[len(ranges)-1][1] == index-1 {
			ranges[len(ranges)-1][1] = index
		} else {
			ranges = append(ranges, []int{index, index})
		}
	}
	return ranges
}

func filterOut(recent []string, target string) []string {
	var next []string
	for _, entry := range recent {
		if CanonicalPath(entry) != target {
			next = append(next, entry)
		}
	}
	return next
}

func truncate(text string, length int) string {
	if len(text) <= length {
		return text
	}
	return text[:length-1] + "…"
}
