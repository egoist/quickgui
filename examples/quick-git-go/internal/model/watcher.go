package model

import (
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

type ChangeKind string

const (
	ChangeWorktree ChangeKind = "worktree"
	ChangeRefs     ChangeKind = "refs"
	ChangeIndex    ChangeKind = "index"
)

type WatcherOptions struct {
	Root      string
	GitDir    string
	CommonDir string
	OnChange  func(kinds map[ChangeKind]struct{})
	Debounce  time.Duration
	PollEvery time.Duration
}

func ClassifyGitPath(path string) ChangeKind {
	normalized := filepath.ToSlash(path)
	if strings.HasPrefix(normalized, "objects/") || strings.HasPrefix(normalized, "lfs/") || normalized == "objects" {
		return ""
	}
	if normalized == "index" || normalized == "index.lock" {
		return ChangeIndex
	}
	if normalized == "HEAD" || normalized == "ORIG_HEAD" || normalized == "FETCH_HEAD" ||
		normalized == "MERGE_HEAD" || normalized == "packed-refs" ||
		strings.HasPrefix(normalized, "refs/") || strings.HasPrefix(normalized, "logs/") ||
		strings.HasPrefix(normalized, "worktrees/") {
		return ChangeRefs
	}
	if strings.HasSuffix(normalized, ".lock") || normalized == "COMMIT_EDITMSG" {
		return ""
	}
	return ChangeRefs
}

func ClassifyWorktreePath(path, gitDirRelative string) ChangeKind {
	normalized := filepath.ToSlash(path)
	gitRelative := filepath.ToSlash(gitDirRelative)
	if normalized == ".git" || normalized == gitRelative {
		return ChangeRefs
	}
	if strings.HasPrefix(normalized, ".git/") {
		return ClassifyGitPath(normalized[5:])
	}
	if gitRelative != "" && strings.HasPrefix(normalized, gitRelative+"/") {
		return ClassifyGitPath(normalized[len(gitRelative)+1:])
	}
	return ChangeWorktree
}

type pollSnapshot struct {
	index time.Time
	head  time.Time
	refs  time.Time
}

func WatchRepository(options WatcherOptions) func() {
	debounce := options.Debounce
	if debounce <= 0 {
		debounce = 150 * time.Millisecond
	}
	interval := options.PollEvery
	if interval <= 0 {
		interval = 750 * time.Millisecond
	}
	pending := map[ChangeKind]struct{}{}
	var mu sync.Mutex
	var timer *time.Timer
	closed := false
	flush := func() {
		mu.Lock()
		timer = nil
		if closed || len(pending) == 0 {
			mu.Unlock()
			return
		}
		kinds := map[ChangeKind]struct{}{}
		for kind := range pending {
			kinds[kind] = struct{}{}
		}
		pending = map[ChangeKind]struct{}{}
		callback := options.OnChange
		mu.Unlock()
		if callback != nil {
			callback(kinds)
		}
	}
	note := func(kind ChangeKind) {
		if kind == "" {
			return
		}
		mu.Lock()
		defer mu.Unlock()
		if closed {
			return
		}
		pending[kind] = struct{}{}
		if timer != nil {
			timer.Stop()
		}
		timer = time.AfterFunc(debounce, flush)
	}
	last := readSnapshot(options.GitDir, options.CommonDir)
	stop := make(chan struct{})
	go func() {
		ticker := time.NewTicker(interval)
		defer ticker.Stop()
		for {
			select {
			case <-stop:
				return
			case <-ticker.C:
				next := readSnapshot(options.GitDir, options.CommonDir)
				if !next.index.Equal(last.index) {
					note(ChangeIndex)
				}
				if !next.head.Equal(last.head) || !next.refs.Equal(last.refs) {
					note(ChangeRefs)
				}
				last = next
			}
		}
	}()
	return func() {
		mu.Lock()
		closed = true
		if timer != nil {
			timer.Stop()
		}
		mu.Unlock()
		close(stop)
	}
}

func readSnapshot(gitDir, commonDir string) pollSnapshot {
	if commonDir == "" {
		commonDir = gitDir
	}
	return pollSnapshot{
		index: mtime(filepath.Join(commonDir, "index")),
		head:  mtime(filepath.Join(gitDir, "HEAD")),
		refs:  latestMtime(filepath.Join(commonDir, "refs"), filepath.Join(commonDir, "packed-refs"), filepath.Join(gitDir, "HEAD")),
	}
}

func mtime(path string) time.Time {
	info, err := os.Stat(path)
	if err != nil {
		return time.Time{}
	}
	return info.ModTime()
}

func latestMtime(paths ...string) time.Time {
	var latest time.Time
	for _, path := range paths {
		info, err := os.Stat(path)
		if err != nil {
			continue
		}
		if info.IsDir() {
			_ = filepath.WalkDir(path, func(_ string, entry os.DirEntry, err error) error {
				if err != nil || entry.IsDir() {
					return nil
				}
				info, err := entry.Info()
				if err != nil {
					return nil
				}
				if info.ModTime().After(latest) {
					latest = info.ModTime()
				}
				return nil
			})
			continue
		}
		if info.ModTime().After(latest) {
			latest = info.ModTime()
		}
	}
	return latest
}
