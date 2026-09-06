package agent

import (
	"strings"
	"testing"
)

func TestParseMessage(t *testing.T) {
	subject, body, ok := ParseMessage("```\nFix the parser\n\nHandle quoted paths.\n```")
	if !ok || subject != "Fix the parser" || body != "Handle quoted paths." {
		t.Fatalf("%q %q %v", subject, body, ok)
	}
	subject, body, ok = ParseMessage("Subject: Ship it")
	if !ok || subject != "Ship it" || body != "" {
		t.Fatalf("%q %q %v", subject, body, ok)
	}
}

func TestBoundDiff(t *testing.T) {
	diff := "diff --git a/one b/one\n+one\ndiff --git a/two b/two\n+two\n"
	bound := BoundDiff(diff, 40)
	if bound.Omitted != 1 || bound.Text == "" {
		t.Fatalf("%+v", bound)
	}
}

func TestBuildPrompt(t *testing.T) {
	prompt := BuildPrompt(Request{RepositoryName: "quick-git", Branch: "main", Diff: "diff --git a/x b/x\n+x\n", Files: []string{"M x"}, RecentSubjects: []string{"Ship it"}})
	if prompt == "" || !strings.Contains(prompt, "Repository: quick-git (branch main).") {
		t.Fatal(prompt)
	}
}
