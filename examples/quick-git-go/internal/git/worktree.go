package git

import (
	"regexp"
	"strings"
)

type Worktree struct {
	Path        string
	HeadSha     string
	Branch      string
	BranchName  string
	Detached    bool
	Bare        bool
	Locked      bool
	LockReason  string
	Prunable    bool
	PruneReason string
	Main        bool
}

func ParseWorktrees(output string) []Worktree {
	var worktrees []Worktree
	for _, entry := range strings.Split(output, "\x00\x00") {
		attributes := []string{}
		for _, line := range strings.Split(entry, "\x00") {
			if line != "" {
				attributes = append(attributes, line)
			}
		}
		if len(attributes) == 0 {
			continue
		}
		worktree := Worktree{Main: len(worktrees) == 0}
		for _, attribute := range attributes {
			key, value, _ := strings.Cut(attribute, " ")
			switch key {
			case "worktree":
				worktree.Path = value
			case "HEAD":
				worktree.HeadSha = value
			case "branch":
				worktree.Branch = value
				worktree.BranchName = strings.TrimPrefix(value, "refs/heads/")
			case "detached":
				worktree.Detached = true
			case "bare":
				worktree.Bare = true
			case "locked":
				worktree.Locked = true
				worktree.LockReason = value
			case "prunable":
				worktree.Prunable = true
				worktree.PruneReason = value
			}
		}
		if worktree.Path != "" {
			worktrees = append(worktrees, worktree)
		}
	}
	return worktrees
}

var worktreeName = regexp.MustCompile(`[\\/:\s]+`)
var trimDashes = regexp.MustCompile(`^-+|-+$`)

func WorktreeDirectoryName(branch string) string {
	name := trimDashes.ReplaceAllString(worktreeName.ReplaceAllString(branch, "-"), "")
	if name == "" {
		return "worktree"
	}
	return name
}
