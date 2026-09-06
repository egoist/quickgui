package git

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

const (
	DefaultTimeout        = 60 * time.Second
	NetworkTimeout        = 10 * time.Minute
	DefaultMaxOutputBytes = 64 * 1024 * 1024
	stderrCap             = 1 * 1024 * 1024
)

type Priority string

const (
	Interactive Priority = "interactive"
	Background  Priority = "background"
)

type Result struct {
	Stdout    []byte
	Stderr    string
	ExitCode  int
	Truncated bool
	Aborted   bool
	TimedOut  bool
}

type Error struct {
	Command  []string
	Cwd      string
	ExitCode int
	Stderr   string
	Aborted  bool
	TimedOut bool
	message  string
}

func (e *Error) Error() string {
	if strings.TrimSpace(e.Stderr) != "" {
		return strings.TrimSpace(e.Stderr)
	}
	return e.message
}

func (e *Error) Summary() string {
	for i := len(strings.Split(e.Stderr, "\n")) - 1; i >= 0; i-- {
		line := strings.TrimSpace(strings.Split(e.Stderr, "\n")[i])
		if line == "" || strings.HasPrefix(line, "hint:") {
			continue
		}
		return strings.TrimPrefix(strings.TrimPrefix(line, "fatal: "), "error: ")
	}
	if e.TimedOut {
		return "git timed out"
	}
	if e.Aborted {
		return "git was cancelled"
	}
	return fmt.Sprintf("git exited with code %d", e.ExitCode)
}

func inheritedEnv() []string {
	return os.Environ()
}

func executableExists(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return !info.IsDir() && info.Mode()&0o111 != 0
}

func ResolveGitExecutable() string {
	if override := strings.TrimSpace(os.Getenv("QUICK_GIT_EXECUTABLE")); override != "" {
		return override
	}
	if path, err := exec.LookPath("git"); err == nil {
		return path
	}
	for _, candidate := range []string{"/opt/homebrew/bin/git", "/usr/local/bin/git", "/usr/bin/git"} {
		if executableExists(candidate) {
			return candidate
		}
	}
	return "git"
}

func runProcess(ctx context.Context, command []string, cwd string, stdin []byte, env []string, timeout time.Duration, maxOutput int) Result {
	if len(command) == 0 {
		return Result{ExitCode: -1, Stderr: "A process executable is required"}
	}
	if ctx == nil {
		ctx = context.Background()
	}
	if timeout <= 0 {
		timeout = DefaultTimeout
	}
	if maxOutput <= 0 {
		maxOutput = DefaultMaxOutputBytes
	}
	runCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	cmd := exec.CommandContext(runCtx, command[0], command[1:]...)
	cmd.Dir = cwd
	if len(env) > 0 {
		cmd.Env = env
	}
	if len(stdin) > 0 {
		cmd.Stdin = bytes.NewReader(stdin)
	}
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	cmd.Stdout = limitWriter{buf: &stdout, limit: maxOutput}
	cmd.Stderr = limitWriter{buf: &stderr, limit: stderrCap}
	err := cmd.Run()
	result := Result{Stdout: stdout.Bytes(), Stderr: stderr.String()}
	if cmd.ProcessState != nil {
		result.ExitCode = cmd.ProcessState.ExitCode()
	}
	if errors.Is(runCtx.Err(), context.DeadlineExceeded) {
		result.TimedOut = true
		result.ExitCode = -1
	} else if errors.Is(runCtx.Err(), context.Canceled) {
		result.Aborted = true
		result.ExitCode = -1
	} else if err != nil && result.ExitCode == 0 {
		result.ExitCode = -1
		if result.Stderr == "" {
			result.Stderr = err.Error()
		}
	}
	if stdout.Len() >= maxOutput {
		result.Truncated = true
	}
	return result
}

type limitWriter struct {
	buf   *bytes.Buffer
	limit int
}

func (w limitWriter) Write(p []byte) (int, error) {
	room := w.limit - w.buf.Len()
	if room <= 0 {
		return len(p), nil
	}
	if len(p) > room {
		w.buf.Write(p[:room])
		return len(p), nil
	}
	return w.buf.Write(p)
}

type CommandOptions struct {
	Cwd            string
	Stdin          []byte
	Timeout        time.Duration
	MaxOutputBytes int
	Env            map[string]string
	AllowExitCodes []int
	Priority       Priority
}

type Runner struct {
	executable  string
	concurrency int
	env         []string
	mu          sync.Mutex
	inFlight    int
	waiters     []waiter
}

type waiter struct {
	interactive bool
	ready       chan struct{}
}

func NewRunner(concurrency int) *Runner {
	if concurrency < 1 {
		concurrency = 4
	}
	return &Runner{
		executable:  ResolveGitExecutable(),
		concurrency: concurrency,
		env: append(inheritedEnv(),
			"GIT_TERMINAL_PROMPT=0",
			"GIT_OPTIONAL_LOCKS=0",
			"LC_ALL=C",
		),
	}
}

func (r *Runner) Run(ctx context.Context, args []string, options CommandOptions) (Result, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	if err := r.acquire(ctx, options.Priority != Background); err != nil {
		return Result{}, &Error{Command: args, Cwd: options.Cwd, ExitCode: -1, Aborted: true, message: "git was cancelled"}
	}
	defer r.release()
	command := append([]string{r.executable, "-c", "core.quotepath=false", "-c", "color.ui=never"}, args...)
	env := append([]string{}, r.env...)
	for key, value := range options.Env {
		env = append(env, key+"="+value)
	}
	result := runProcess(ctx, command, options.Cwd, options.Stdin, env, options.Timeout, options.MaxOutputBytes)
	if result.Aborted || result.TimedOut {
		return result, &Error{
			Command: args, Cwd: options.Cwd, ExitCode: result.ExitCode, Stderr: result.Stderr,
			Aborted: result.Aborted, TimedOut: result.TimedOut, message: "git failed",
		}
	}
	allowed := false
	for _, code := range options.AllowExitCodes {
		if code == result.ExitCode {
			allowed = true
			break
		}
	}
	if result.ExitCode != 0 && !allowed && !result.Truncated {
		return result, &Error{Command: args, Cwd: options.Cwd, ExitCode: result.ExitCode, Stderr: result.Stderr, message: "git failed"}
	}
	return result, nil
}

func (r *Runner) acquire(ctx context.Context, interactive bool) error {
	r.mu.Lock()
	if r.inFlight < r.concurrency {
		r.inFlight++
		r.mu.Unlock()
		return nil
	}
	ready := make(chan struct{})
	entry := waiter{interactive: interactive, ready: ready}
	if interactive {
		inserted := false
		for i, existing := range r.waiters {
			if !existing.interactive {
				r.waiters = append(r.waiters[:i], append([]waiter{entry}, r.waiters[i:]...)...)
				inserted = true
				break
			}
		}
		if !inserted {
			r.waiters = append(r.waiters, entry)
		}
	} else {
		r.waiters = append(r.waiters, entry)
	}
	r.mu.Unlock()
	select {
	case <-ready:
		return nil
	case <-ctx.Done():
		r.mu.Lock()
		for i, existing := range r.waiters {
			if existing.ready == ready {
				r.waiters = append(r.waiters[:i], r.waiters[i+1:]...)
				break
			}
		}
		r.mu.Unlock()
		return ctx.Err()
	}
}

func (r *Runner) release() {
	r.mu.Lock()
	defer r.mu.Unlock()
	if len(r.waiters) > 0 {
		next := r.waiters[0]
		r.waiters = r.waiters[1:]
		close(next.ready)
		return
	}
	r.inFlight--
}

func (r *Runner) Text(ctx context.Context, args []string, options CommandOptions) (string, error) {
	result, err := r.Run(ctx, args, options)
	if err != nil {
		return "", err
	}
	return DecodeOutput(result.Stdout), nil
}

func DecodeOutput(data []byte) string {
	return string(data)
}

func Pathspec(paths []string) []byte {
	if len(paths) == 0 {
		return nil
	}
	return []byte(strings.Join(paths, "\x00") + "\x00")
}

func LookPath(command string) string {
	if path, err := exec.LookPath(command); err == nil {
		return path
	}
	home, _ := os.UserHomeDir()
	extras := []string{
		filepath.Join(home, ".local/bin"),
		filepath.Join(home, ".bun/bin"),
		filepath.Join(home, ".cargo/bin"),
		"/opt/homebrew/bin",
		"/usr/local/bin",
		"/usr/bin",
	}
	for _, dir := range extras {
		candidate := filepath.Join(dir, command)
		if executableExists(candidate) {
			return candidate
		}
	}
	return ""
}

var _ = io.Discard
