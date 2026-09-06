package git

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

const (
	MaxUntrackedPreviewBytes = 4 * 1024 * 1024
	MaxDiffOutputBytes       = 32 * 1024 * 1024
)

type RepositoryInfo struct {
	Root      string
	GitDir    string
	CommonDir string
	Bare      bool
}

type LogOptions struct {
	Limit int
	Skip  int
	Ref   string
	All   bool
}

type CommitOptions struct {
	Amend      bool
	Signoff    bool
	AllowEmpty bool
	NoVerify   bool
}

type CommitFile struct {
	Status       string
	Path         string
	OriginalPath string
}

type NumstatEntry struct {
	Path    string
	Added   *int
	Removed *int
}

type Repository struct {
	Info   RepositoryInfo
	Runner *Runner
	Trash  func(absolutePath string) error
}

func (r *Repository) Root() string { return r.Info.Root }

func Open(ctx context.Context, runner *Runner, path string, trash func(string) error) (*Repository, error) {
	cwd, err := filepath.Abs(path)
	if err != nil {
		return nil, err
	}
	info, err := os.Stat(cwd)
	if err != nil {
		return nil, &Error{Cwd: cwd, ExitCode: -1, Stderr: path + " does not exist", message: path + " does not exist"}
	}
	if !info.IsDir() {
		cwd = filepath.Dir(cwd)
	}
	output, err := runner.Text(ctx, []string{"rev-parse", "--show-toplevel", "--absolute-git-dir", "--git-common-dir", "--is-bare-repository"}, CommandOptions{Cwd: cwd})
	if err != nil {
		return nil, err
	}
	lines := strings.Split(strings.TrimRight(output, "\n"), "\n")
	if len(lines) < 2 || lines[0] == "" || lines[1] == "" {
		return nil, &Error{Cwd: cwd, ExitCode: -1, Stderr: output, message: "not a git repository"}
	}
	root, gitDir := lines[0], lines[1]
	commonDir := gitDir
	if len(lines) > 2 && lines[2] != "" {
		if filepath.IsAbs(lines[2]) {
			commonDir = lines[2]
		} else {
			commonDir = filepath.Join(root, lines[2])
		}
	}
	bare := len(lines) > 3 && lines[3] == "true"
	if trash == nil {
		trash = func(p string) error { return os.RemoveAll(p) }
	}
	return &Repository{
		Info:   RepositoryInfo{Root: root, GitDir: gitDir, CommonDir: commonDir, Bare: bare},
		Runner: runner,
		Trash:  trash,
	}, nil
}

func (r *Repository) Worktree(ctx context.Context, path string) (*Repository, error) {
	return Open(ctx, r.Runner, path, r.Trash)
}

func (r *Repository) opts(priority Priority) CommandOptions {
	return CommandOptions{
		Cwd:            r.Info.Root,
		Priority:       priority,
		MaxOutputBytes: DefaultMaxOutputBytes,
		Env:            map[string]string{"GIT_EDITOR": "true"},
	}
}

func (r *Repository) Status(ctx context.Context) (RepositoryStatus, error) {
	output, err := r.Runner.Text(ctx, []string{
		"--no-optional-locks", "status", "--porcelain=v2", "-z", "--branch", "--show-stash",
		"--untracked-files=all", "--renames",
	}, r.opts(Interactive))
	if err != nil {
		return RepositoryStatus{}, err
	}
	return ParseStatus(output), nil
}

func (r *Repository) Numstat(ctx context.Context, staged bool) ([]NumstatEntry, error) {
	args := []string{"--no-optional-locks", "diff", "--numstat", "-z", "--find-renames"}
	if staged {
		args = append(args, "--cached")
	}
	output, err := r.Runner.Text(ctx, args, r.opts(Interactive))
	if err != nil {
		return nil, err
	}
	return ParseNumstat(output), nil
}

func (r *Repository) DiffWorkingTree(ctx context.Context, path string) (*Diff, error) {
	return r.diff(ctx, []string{"diff"}, []string{path})
}

func (r *Repository) DiffIndex(ctx context.Context, path, originalPath string) (*Diff, error) {
	paths := []string{path}
	if originalPath != "" {
		paths = []string{originalPath, path}
	}
	return r.diff(ctx, []string{"diff", "--cached"}, paths)
}

func (r *Repository) DiffUntracked(ctx context.Context, path string) (*Diff, error) {
	absolute := filepath.Join(r.Info.Root, path)
	info, err := os.Stat(absolute)
	if err != nil {
		return nil, err
	}
	if info.Size() > MaxUntrackedPreviewBytes {
		return OpenDiff(SyntheticDiffText(path, "", false), true), nil
	}
	if ctx.Err() != nil {
		return nil, ctx.Err()
	}
	data, err := os.ReadFile(absolute)
	if err != nil {
		return nil, err
	}
	if LooksBinary(data) {
		return OpenDiff(SyntheticDiffText(path, "", true), false), nil
	}
	return OpenDiff(SyntheticDiffText(path, DecodeOutput(data), false), false), nil
}

func (r *Repository) CommitFiles(ctx context.Context, sha string) ([]CommitFile, error) {
	output, err := r.Runner.Text(ctx, []string{"show", "--format=", "--first-parent", "-M", "-z", "--name-status", sha, "--"}, r.opts(Background))
	if err != nil {
		return nil, err
	}
	return ParseCommitFiles(output), nil
}

func (r *Repository) DiffCommit(ctx context.Context, sha, path string) (*Diff, error) {
	paths := []string{}
	if path != "" {
		paths = []string{path}
	}
	return r.diff(ctx, []string{"show", "--format=", "--first-parent", sha}, paths)
}

func (r *Repository) Log(ctx context.Context, options LogOptions) ([]Commit, error) {
	limit := options.Limit
	if limit <= 0 {
		limit = 200
	}
	args := []string{"log", "-z", "--format=" + LogFormat, "--date-order", "-n", strconv.Itoa(limit)}
	if options.Skip > 0 {
		args = append(args, fmt.Sprintf("--skip=%d", options.Skip))
	}
	if options.All {
		args = append(args, "--all")
	}
	if options.Ref != "" {
		args = append(args, options.Ref)
	}
	args = append(args, "--")
	opts := r.opts(Background)
	opts.AllowExitCodes = []int{128}
	result, err := r.Runner.Run(ctx, args, opts)
	if err != nil {
		return nil, err
	}
	if result.ExitCode != 0 {
		return nil, nil
	}
	return ParseLog(DecodeOutput(result.Stdout)), nil
}

func (r *Repository) Refs(ctx context.Context) (RefCollections, error) {
	output, err := r.Runner.Text(ctx, []string{
		"for-each-ref", "--format=" + ForEachRefFormat, "--sort=-committerdate",
		"refs/heads", "refs/remotes", "refs/tags",
	}, r.opts(Background))
	if err != nil {
		return RefCollections{}, err
	}
	return ParseRefs(output), nil
}

func (r *Repository) Stashes(ctx context.Context) ([]StashEntry, error) {
	output, err := r.Runner.Text(ctx, []string{"stash", "list", "-z", "--format=" + StashFormat}, r.opts(Background))
	if err != nil {
		return nil, err
	}
	return ParseStashes(output), nil
}

func (r *Repository) Worktrees(ctx context.Context) ([]Worktree, error) {
	output, err := r.Runner.Text(ctx, []string{"worktree", "list", "--porcelain", "-z"}, r.opts(Background))
	if err != nil {
		return nil, err
	}
	return ParseWorktrees(output), nil
}

func (r *Repository) RecentSubjects(ctx context.Context, count int) ([]string, error) {
	opts := r.opts(Background)
	opts.AllowExitCodes = []int{128}
	result, err := r.Runner.Run(ctx, []string{"log", "-z", "--format=%s", "-n", strconv.Itoa(count), "--"}, opts)
	if err != nil {
		return nil, err
	}
	if result.ExitCode != 0 {
		return nil, nil
	}
	return SplitNul(DecodeOutput(result.Stdout)), nil
}

func (r *Repository) Config(ctx context.Context, key string) (string, error) {
	opts := r.opts(Background)
	opts.AllowExitCodes = []int{1}
	result, err := r.Runner.Run(ctx, []string{"config", "--get", key}, opts)
	if err != nil {
		return "", err
	}
	if result.ExitCode != 0 {
		return "", nil
	}
	return strings.TrimRight(DecodeOutput(result.Stdout), "\n"), nil
}

func (r *Repository) HasHead(ctx context.Context) (bool, error) {
	opts := r.opts(Interactive)
	opts.AllowExitCodes = []int{1}
	result, err := r.Runner.Run(ctx, []string{"rev-parse", "--verify", "--quiet", "HEAD"}, opts)
	if err != nil {
		return false, err
	}
	return result.ExitCode == 0, nil
}

func (r *Repository) ShowFile(ctx context.Context, ref, path string) ([]byte, error) {
	opts := r.opts(Interactive)
	opts.MaxOutputBytes = MaxDiffOutputBytes
	result, err := r.Runner.Run(ctx, []string{"show", ref + ":" + path}, opts)
	if err != nil {
		return nil, err
	}
	return result.Stdout, nil
}

func (r *Repository) Stage(ctx context.Context, paths []string) error {
	if len(paths) == 0 {
		return nil
	}
	return r.pathspec(ctx, []string{"add", "--all", "--"}, paths)
}

func (r *Repository) StageAll(ctx context.Context) error {
	_, err := r.Runner.Run(ctx, []string{"add", "--all"}, r.opts(Interactive))
	return err
}

func (r *Repository) Unstage(ctx context.Context, paths []string) error {
	if len(paths) == 0 {
		return nil
	}
	return r.pathspec(ctx, []string{"reset", "-q", "--"}, paths)
}

func (r *Repository) UnstageAll(ctx context.Context) error {
	_, err := r.Runner.Run(ctx, []string{"reset", "-q"}, r.opts(Interactive))
	return err
}

func (r *Repository) Discard(ctx context.Context, tracked, untracked []string) error {
	if len(tracked) > 0 {
		if err := r.pathspec(ctx, []string{"checkout", "-q", "--"}, tracked); err != nil {
			return err
		}
	}
	for _, path := range untracked {
		if ctx.Err() != nil {
			return ctx.Err()
		}
		if err := r.Trash(filepath.Join(r.Info.Root, path)); err != nil {
			return err
		}
	}
	return nil
}

func (r *Repository) StagePatch(ctx context.Context, file DiffFile, selections []HunkSelection) error {
	patch := FormatPatch(file, selections, false)
	if patch == "" {
		return nil
	}
	return r.apply(ctx, []string{"--cached"}, patch)
}

func (r *Repository) UnstagePatch(ctx context.Context, file DiffFile, selections []HunkSelection) error {
	patch := FormatPatch(file, selections, true)
	if patch == "" {
		return nil
	}
	return r.apply(ctx, []string{"--cached", "--reverse"}, patch)
}

func (r *Repository) DiscardPatch(ctx context.Context, file DiffFile, selections []HunkSelection) error {
	patch := FormatPatch(file, selections, true)
	if patch == "" {
		return nil
	}
	return r.apply(ctx, []string{"--reverse"}, patch)
}

func (r *Repository) Commit(ctx context.Context, message string, options CommitOptions) error {
	args := []string{"commit", "-q", "-F", "-", "--cleanup=strip"}
	if options.Amend {
		args = append(args, "--amend")
	}
	if options.Signoff {
		args = append(args, "--signoff")
	}
	if options.AllowEmpty {
		args = append(args, "--allow-empty")
	}
	if options.NoVerify {
		args = append(args, "--no-verify")
	}
	opts := r.opts(Interactive)
	opts.Stdin = []byte(message)
	opts.Timeout = NetworkTimeout
	_, err := r.Runner.Run(ctx, args, opts)
	return err
}

func (r *Repository) ResolveConflict(ctx context.Context, path, side string) error {
	if _, err := r.Runner.Run(ctx, []string{"checkout", "--" + side, "--", path}, r.opts(Interactive)); err != nil {
		return err
	}
	_, err := r.Runner.Run(ctx, []string{"add", "--", path}, r.opts(Interactive))
	return err
}

func (r *Repository) SwitchBranch(ctx context.Context, name string) error {
	_, err := r.Runner.Run(ctx, []string{"switch", name}, r.opts(Interactive))
	return err
}

func (r *Repository) CreateBranch(ctx context.Context, name string, from string, checkout bool) error {
	start := []string{}
	if from != "" {
		start = []string{from}
	}
	if checkout {
		_, err := r.Runner.Run(ctx, append([]string{"switch", "-c", name}, start...), r.opts(Interactive))
		return err
	}
	_, err := r.Runner.Run(ctx, append([]string{"branch", name}, start...), r.opts(Interactive))
	return err
}

func (r *Repository) DeleteBranch(ctx context.Context, name string, force bool) error {
	flag := "-d"
	if force {
		flag = "-D"
	}
	_, err := r.Runner.Run(ctx, []string{"branch", flag, name}, r.opts(Interactive))
	return err
}

func (r *Repository) RenameBranch(ctx context.Context, from, to string) error {
	_, err := r.Runner.Run(ctx, []string{"branch", "-m", from, to}, r.opts(Interactive))
	return err
}

func (r *Repository) CheckoutCommit(ctx context.Context, sha string) error {
	_, err := r.Runner.Run(ctx, []string{"switch", "--detach", sha}, r.opts(Interactive))
	return err
}

func (r *Repository) Fetch(ctx context.Context) error {
	opts := r.opts(Interactive)
	opts.Timeout = NetworkTimeout
	_, err := r.Runner.Run(ctx, []string{"fetch", "--prune", "--no-write-fetch-head"}, opts)
	return err
}

func (r *Repository) Pull(ctx context.Context) error {
	opts := r.opts(Interactive)
	opts.Timeout = NetworkTimeout
	_, err := r.Runner.Run(ctx, []string{"pull", "--no-edit"}, opts)
	return err
}

func (r *Repository) Push(ctx context.Context, remote, branch string, setUpstream, forceWithLease bool) error {
	args := []string{"push", "--porcelain"}
	if forceWithLease {
		args = append(args, "--force-with-lease")
	}
	if setUpstream && remote != "" && branch != "" {
		args = append(args, "--set-upstream", remote, branch)
	}
	opts := r.opts(Interactive)
	opts.Timeout = NetworkTimeout
	_, err := r.Runner.Run(ctx, args, opts)
	return err
}

func (r *Repository) Remotes(ctx context.Context) ([]string, error) {
	output, err := r.Runner.Text(ctx, []string{"remote"}, r.opts(Background))
	if err != nil {
		return nil, err
	}
	var names []string
	for _, line := range strings.Split(output, "\n") {
		if line != "" {
			names = append(names, line)
		}
	}
	return names, nil
}

func (r *Repository) StashPush(ctx context.Context, message string, includeUntracked, keepIndex bool) error {
	args := []string{"stash", "push", "-q"}
	if includeUntracked {
		args = append(args, "--include-untracked")
	}
	if keepIndex {
		args = append(args, "--keep-index")
	}
	if message != "" {
		args = append(args, "-m", message)
	}
	_, err := r.Runner.Run(ctx, args, r.opts(Interactive))
	return err
}

func (r *Repository) StashApply(ctx context.Context, ref string) error {
	_, err := r.Runner.Run(ctx, []string{"stash", "apply", "-q", ref}, r.opts(Interactive))
	return err
}

func (r *Repository) StashPop(ctx context.Context, ref string) error {
	_, err := r.Runner.Run(ctx, []string{"stash", "pop", "-q", ref}, r.opts(Interactive))
	return err
}

func (r *Repository) StashDrop(ctx context.Context, ref string) error {
	_, err := r.Runner.Run(ctx, []string{"stash", "drop", "-q", ref}, r.opts(Interactive))
	return err
}

func (r *Repository) AddWorktree(ctx context.Context, path, newBranch, branch, base string) error {
	args := []string{"worktree", "add"}
	if newBranch != "" {
		args = append(args, "-b", newBranch)
	}
	args = append(args, path)
	if branch != "" {
		args = append(args, branch)
	} else if base != "" {
		args = append(args, base)
	} else if newBranch == "" {
		args = append(args, "--detach", "HEAD")
	}
	_, err := r.Runner.Run(ctx, args, r.opts(Interactive))
	return err
}

func (r *Repository) RemoveWorktree(ctx context.Context, path string, force bool) error {
	args := []string{"worktree", "remove"}
	if force {
		args = append(args, "--force")
	}
	args = append(args, path)
	_, err := r.Runner.Run(ctx, args, r.opts(Interactive))
	return err
}

func (r *Repository) PruneWorktrees(ctx context.Context) error {
	_, err := r.Runner.Run(ctx, []string{"worktree", "prune"}, r.opts(Interactive))
	return err
}

func (r *Repository) diff(ctx context.Context, command, paths []string) (*Diff, error) {
	args := append([]string{"--no-optional-locks"}, command...)
	args = append(args, "--no-color", "--no-ext-diff", "--find-renames", "--unified=3", "--src-prefix=a/", "--dst-prefix=b/", "--")
	args = append(args, paths...)
	opts := r.opts(Interactive)
	opts.MaxOutputBytes = MaxDiffOutputBytes
	result, err := r.Runner.Run(ctx, args, opts)
	if err != nil {
		return nil, err
	}
	return OpenDiff(DecodeOutput(result.Stdout), result.Truncated), nil
}

func (r *Repository) apply(ctx context.Context, flags []string, patch string) error {
	args := append([]string{"apply"}, flags...)
	args = append(args, "--whitespace=nowarn", "-")
	opts := r.opts(Interactive)
	opts.Stdin = []byte(patch)
	_, err := r.Runner.Run(ctx, args, opts)
	return err
}

func (r *Repository) pathspec(ctx context.Context, command, paths []string) error {
	args := append([]string{}, command[:len(command)-1]...)
	args = append(args, "--pathspec-from-file=-", "--pathspec-file-nul", command[len(command)-1])
	opts := r.opts(Interactive)
	opts.Stdin = Pathspec(paths)
	_, err := r.Runner.Run(ctx, args, opts)
	return err
}

func ParseNumstat(output string) []NumstatEntry {
	var entries []NumstatEntry
	records := SplitNul(output)
	for index := 0; index < len(records); index++ {
		record := records[index]
		added, rest, ok := strings.Cut(record, "\t")
		if !ok {
			continue
		}
		removed, path, ok := strings.Cut(rest, "\t")
		if !ok {
			continue
		}
		if path == "" {
			index += 2
			if index < len(records) {
				path = records[index]
			}
		}
		entry := NumstatEntry{Path: path}
		if added != "-" {
			if n, err := strconv.Atoi(added); err == nil {
				entry.Added = &n
			}
		}
		if removed != "-" {
			if n, err := strconv.Atoi(removed); err == nil {
				entry.Removed = &n
			}
		}
		entries = append(entries, entry)
	}
	return entries
}

func ParseCommitFiles(output string) []CommitFile {
	var files []CommitFile
	records := SplitNul(output)
	for index := 0; index < len(records); index++ {
		status := records[index]
		if status == "" {
			continue
		}
		code := status[:1]
		if code == "R" || code == "C" {
			original, path := "", ""
			if index+1 < len(records) {
				original = records[index+1]
			}
			if index+2 < len(records) {
				path = records[index+2]
			}
			index += 2
			files = append(files, CommitFile{Status: code, Path: path, OriginalPath: original})
			continue
		}
		path := ""
		if index+1 < len(records) {
			path = records[index+1]
		}
		index++
		files = append(files, CommitFile{Status: code, Path: path})
	}
	return files
}

func CanonicalPath(path string) string {
	absolute, err := filepath.Abs(path)
	if err != nil {
		return path
	}
	if resolved, err := filepath.EvalSymlinks(absolute); err == nil {
		return resolved
	}
	return absolute
}

func SuggestWorktreePath(root, branch string) string {
	name := WorktreeDirectoryName(branch)
	return filepath.Join(filepath.Dir(root), filepath.Base(root)+"-"+name)
}
