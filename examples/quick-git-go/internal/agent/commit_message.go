package agent

import (
	"context"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"os/exec"

	"github.com/egoist/quickgui/examples/quick-git-go/internal/git"
)

type Error struct {
	Agent   AgentID
	Aborted bool
	message string
}

func (e *Error) Error() string { return e.message }

type AgentID string

const (
	AgentCodex         AgentID = "codex"
	AgentClaude        AgentID = "claude"
	Timeout                    = 120 * time.Second
	MaxPromptDiffChars         = 60_000
	MaxPromptFiles             = 200
	MaxRecentSubjects          = 15
)

type Definition struct {
	ID          AgentID
	Label       string
	Command     string
	Description string
}

type Available struct {
	Definition
	Executable string
}

var Agents = []Definition{
	{ID: AgentCodex, Label: "Codex", Command: "codex", Description: "OpenAI Codex CLI (codex exec)"},
	{ID: AgentClaude, Label: "Claude", Command: "claude", Description: "Claude Code (claude -p)"},
}

type Request struct {
	RepositoryName string
	Branch         string
	Diff           string
	Files          []string
	RecentSubjects []string
	Amending       string
	Hint           string
}

type GeneratedMessage struct {
	Subject    string
	Body       string
	Raw        string
	Agent      AgentID
	DurationMs int64
}

func DetectAgents() []Available {
	var found []Available
	for _, agent := range Agents {
		if executable := git.LookPath(agent.Command); executable != "" {
			found = append(found, Available{Definition: agent, Executable: executable})
		}
	}
	return found
}

func BuildPrompt(request Request) string {
	var lines []string
	lines = append(lines,
		"You write git commit messages. Reply with only the commit message and nothing else: no preamble, no code fences, no quotes, no explanation.",
		"Rules: a subject line in the imperative mood of at most 72 characters; then, only when it adds real information, a blank line and a body wrapped at 72 columns that says what changed and why. Never restate the diff line by line. Do not add a sign-off or trailers.",
		"",
		"Repository: "+request.RepositoryName+branchSuffix(request.Branch)+".",
	)
	subjects := request.RecentSubjects
	if len(subjects) > MaxRecentSubjects {
		subjects = subjects[:MaxRecentSubjects]
	}
	if len(subjects) > 0 {
		lines = append(lines, "", "Recent commit subjects, newest first. Match their conventions (prefixes, tense, capitalization, punctuation):")
		for _, subject := range subjects {
			lines = append(lines, "- "+subject)
		}
	}
	if request.Amending != "" {
		lines = append(lines, "", "This commit amends the previous one, whose message was:", "", indent(request.Amending))
	}
	if strings.TrimSpace(request.Hint) != "" {
		lines = append(lines, "", "Instruction from the author: "+strings.TrimSpace(request.Hint))
	}
	files := request.Files
	if len(files) > MaxPromptFiles {
		files = files[:MaxPromptFiles]
	}
	lines = append(lines, "", "Files changed ("+itoa(len(request.Files))+"):")
	for _, file := range files {
		lines = append(lines, "- "+file)
	}
	if len(request.Files) > len(files) {
		lines = append(lines, "- … and "+itoa(len(request.Files)-len(files))+" more")
	}
	bound := BoundDiff(request.Diff, MaxPromptDiffChars)
	lines = append(lines, "", "Diff:", "```diff", strings.TrimRight(bound.Text, "\n"), "```")
	if bound.Omitted > 0 {
		noun := "files were"
		if bound.Omitted == 1 {
			noun = "file was"
		}
		lines = append(lines, "", "("+itoa(bound.Omitted)+" more changed "+noun+" left out of the diff for length; the file list above is complete.)")
	}
	return strings.Join(lines, "\n") + "\n"
}

func branchSuffix(branch string) string {
	if branch == "" {
		return ""
	}
	return " (branch " + branch + ")"
}

func indent(text string) string {
	parts := strings.Split(text, "\n")
	for i, line := range parts {
		parts[i] = "    " + line
	}
	return strings.Join(parts, "\n")
}

type Bound struct {
	Text    string
	Omitted int
}

func BoundDiff(diff string, budget int) Bound {
	if len(diff) <= budget {
		return Bound{Text: diff}
	}
	files := splitDiffFiles(diff)
	text := ""
	kept := 0
	for _, file := range files {
		if len(text)+len(file) > budget {
			break
		}
		text += file
		kept++
	}
	if kept == 0 && len(files) > 0 {
		cut := files[0]
		if len(cut) > budget {
			cut = cut[:budget]
		}
		text = cut + "\n… (diff truncated)\n"
		kept = 1
	}
	return Bound{Text: text, Omitted: len(files) - kept}
}

func splitDiffFiles(diff string) []string {
	var files []string
	current := ""
	for _, line := range strings.SplitAfter(diff, "\n") {
		if strings.HasPrefix(line, "diff --git ") && current != "" {
			files = append(files, current)
			current = ""
		}
		current += line
	}
	if current != "" {
		files = append(files, current)
	}
	return files
}

var fence = regexp.MustCompile("(?s)^```[a-z]*\n(.*)\n```$")

func ParseMessage(raw string) (subject, body string, ok bool) {
	text := strings.TrimSpace(strings.ReplaceAll(raw, "\r\n", "\n"))
	if match := fence.FindStringSubmatch(text); match != nil {
		text = strings.TrimSpace(match[1])
	}
	text = regexp.MustCompile(`(?i)^(?:commit message|message|subject)\s*:\s*`).ReplaceAllString(text, "")
	if len(text) >= 2 && ((text[0] == '"' && text[len(text)-1] == '"') || (strings.HasPrefix(text, "“") && strings.HasSuffix(text, "”"))) && !strings.Contains(text, "\n") {
		text = text[1 : len(text)-1]
	}
	lines := strings.Split(text, "\n")
	for len(lines) > 0 && strings.TrimSpace(lines[0]) == "" {
		lines = lines[1:]
	}
	if len(lines) == 0 {
		return "", "", false
	}
	subject = strings.Trim(strings.TrimSpace(lines[0]), "`'\"")
	body = strings.TrimRight(strings.TrimLeft(strings.Join(lines[1:], "\n"), "\n"), " \t\n")
	if subject == "" {
		return "", "", false
	}
	return subject, body, true
}

func AgentCommand(agent Available, cwd, outputFile string) []string {
	switch agent.ID {
	case AgentCodex:
		return []string{agent.Executable, "exec", "--skip-git-repo-check", "--sandbox", "read-only", "--color", "never", "--ephemeral", "-C", cwd, "-o", outputFile, "-"}
	default:
		return []string{agent.Executable, "-p", "--output-format", "text", "--tools", "", "--no-session-persistence", "--disable-slash-commands"}
	}
}

func Generate(ctx context.Context, agent Available, request Request, cwd string) (GeneratedMessage, error) {
	started := time.Now()
	scratch, err := os.MkdirTemp("", "quick-git-agent-")
	if err != nil {
		return GeneratedMessage{}, err
	}
	defer os.RemoveAll(scratch)
	outputFile := filepath.Join(scratch, "message.txt")
	prompt := BuildPrompt(request)
	command := AgentCommand(agent, cwd, outputFile)
	result := runAgent(ctx, command, cwd, []byte(prompt))
	if ctx.Err() != nil {
		return GeneratedMessage{}, ctx.Err()
	}
	raw := string(result.Stdout)
	if agent.ID == AgentCodex {
		if data, err := os.ReadFile(outputFile); err == nil {
			raw = string(data)
		}
	}
	if ctx.Err() != nil {
		return GeneratedMessage{}, &Error{Agent: agent.ID, Aborted: true, message: "cancelled"}
	}
	if result.ExitCode != 0 && strings.TrimSpace(raw) == "" {
		return GeneratedMessage{}, &Error{Agent: agent.ID, message: agent.Label + " failed"}
	}
	subject, body, ok := ParseMessage(raw)
	if !ok {
		return GeneratedMessage{}, &Error{Agent: agent.ID, message: agent.Label + " returned an empty message"}
	}
	return GeneratedMessage{Subject: subject, Body: body, Raw: raw, Agent: agent.ID, DurationMs: time.Since(started).Milliseconds()}, nil
}

func runAgent(ctx context.Context, command []string, cwd string, stdin []byte) git.Result {
	if len(command) == 0 {
		return git.Result{ExitCode: -1, Stderr: "agent executable is required"}
	}
	runCtx, cancel := context.WithTimeout(ctx, Timeout)
	defer cancel()
	cmd := exec.CommandContext(runCtx, command[0], command[1:]...)
	cmd.Dir = cwd
	cmd.Env = append(os.Environ(), "NO_COLOR=1", "TERM=dumb")
	if len(stdin) > 0 {
		cmd.Stdin = strings.NewReader(string(stdin))
	}
	stdout, err := cmd.Output()
	result := git.Result{Stdout: stdout}
	if cmd.ProcessState != nil {
		result.ExitCode = cmd.ProcessState.ExitCode()
	}
	if err != nil {
		if exit, ok := err.(*exec.ExitError); ok {
			result.Stderr = string(exit.Stderr)
			result.ExitCode = exit.ExitCode()
		} else if result.Stderr == "" {
			result.Stderr = err.Error()
			if result.ExitCode == 0 {
				result.ExitCode = -1
			}
		}
	}
	return result
}

func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	var buf [20]byte
	i := len(buf)
	for n > 0 {
		i--
		buf[i] = byte('0' + n%10)
		n /= 10
	}
	return string(buf[i:])
}
