# Quick Git (Go)

A native git client built with the QuickGUI Go frontend. It recreates the TypeScript `examples/quick-git` app: changes, diffs, commit, history, branches, worktrees, stashes, and local coding agents.

```console
CGO_ENABLED=0 go build -o quick-git-go .
```

The host shared library must already be available for `native.Run` (same as the other Go examples). Pass `QUICK_GIT_OPEN=/path/to/repo` to open a repository at launch; otherwise the last one opens.

## What it does

- **Changes**: unstaged and staged lists with per-file line counts, stage or unstage by checkbox or double-click, stage all / unstage all, discard with a native confirmation sheet, native context menus.
- **Diffs**: unified diff with line numbers and hunk headers; stage or unstage a hunk or selected lines via `git apply` patches.
- **Commit**: summary and description, amend, and *Generate* with Codex or Claude when those CLIs are on `PATH` (`codex exec --sandbox read-only`, `claude -p --tools ""`).
- **History**: paged commit list with a lane marker, decorations, commit files, and per-file diffs; new branch from a commit, detached checkout, copy SHA.
- **Branches, stashes, worktrees**: switch, create, delete, stash, apply, pop, drop, add and remove worktrees.
- Fetch, pull, and push (with automatic upstream), ahead/behind counts, and conflict awareness.
- **Windows**: each repository gets its own window. The sidebar header lists recent repositories; *Open Repository…* fills an empty window or opens another.

## How it is built

- `internal/git` is a UI-free git layer: porcelain parsers, a bounded process runner, and repository operations. `CGO_ENABLED=0 go test ./internal/git ./internal/model ./internal/agent` covers the parsers and persistence.
- `internal/model` holds signals, persistence (`quick-git-state.json`), and a poll-based repository watcher. Git work runs on background goroutines; `native.Dispatch` writes signals on the application goroutine.
- `internal/ui` is ordinary QuickGUI views (`View` / `Text` / `Button` / `For` / `Show`). Compound Table, Toast, and Dialog widgets stay TypeScript-only; this example approximates them with primitives and native sheets/menus.
- `main.go` shares one `GitRunner` and persistence across windows, matching the TypeScript app.

The Go frontend remains cgo-free: this example never compiles Rust or invokes a C compiler.
