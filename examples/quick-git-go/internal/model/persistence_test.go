package model

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestParsePersistedState(t *testing.T) {
	raw, _ := json.Marshal(map[string]any{
		"version": 1, "recentRepositories": []string{"/tmp/app", "/tmp/app", ""},
		"sidebarWidth": 200, "changesSplit": 400, "historySplit": 500,
		"preferredAgent": "codex", "historyAllBranches": true,
	})
	state := ParsePersistedState(raw)
	if len(state.RecentRepositories) != 1 || state.SidebarWidth != 200 || !state.HistoryAllBranches || state.PreferredAgent != "codex" {
		t.Fatalf("%+v", state)
	}
}

func TestRememberAndPatch(t *testing.T) {
	recent := RememberRepository([]string{"/tmp/old"}, "/tmp/new")
	if recent[0] != CanonicalPath("/tmp/new") || len(recent) != 2 {
		t.Fatal(recent)
	}
	next := ApplyPatch(DefaultState(), PersistedPatch{ForgetLast: true, HistoryAllBranches: boolRef(true)})
	if next.LastRepository != "" || !next.HistoryAllBranches {
		t.Fatalf("%+v", next)
	}
}

func TestSaveLoadRoundTrip(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "state.json")
	state := DefaultState()
	state.LastRepository = "/tmp/repo"
	state.PreferredAgent = "claude"
	if err := SavePersistedState(path, state); err != nil {
		t.Fatal(err)
	}
	loaded := LoadPersistedState(path)
	if loaded.LastRepository != "/tmp/repo" || loaded.PreferredAgent != "claude" {
		t.Fatalf("%+v", loaded)
	}
	if _, err := os.Stat(path); err != nil {
		t.Fatal(err)
	}
}

func TestClassifyGitPath(t *testing.T) {
	if ClassifyGitPath("objects/ab/cd") != "" || ClassifyGitPath("index") != ChangeIndex || ClassifyGitPath("refs/heads/main") != ChangeRefs {
		t.Fatal("classification")
	}
	if ClassifyWorktreePath("src/app.go", ".git") != ChangeWorktree || ClassifyWorktreePath(".git/HEAD", ".git") != ChangeRefs {
		t.Fatal("worktree classification")
	}
}

func TestRepositoryLabels(t *testing.T) {
	// covered in ui package; keep paths helper honest
	if Basename("/tmp/app") != "app" {
		t.Fatal(Basename("/tmp/app"))
	}
}

func boolRef(value bool) *bool { return &value }
