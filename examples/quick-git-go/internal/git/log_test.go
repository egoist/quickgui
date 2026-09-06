package git

import (
	"strings"
	"testing"
	"time"
)

func TestParseLogIncludesRootCommits(t *testing.T) {
	output := strings.Join([]string{
		strings.Join([]string{
			strings.Repeat("a", 40), "aaaaaaa", strings.Repeat("b", 40) + " " + strings.Repeat("c", 40),
			"Ada", "ada@example.com", "1700000000", "Ada", "ada@example.com", "1700000001",
			"HEAD -> main, origin/main, tag: v1.0", "Merge feature", "Body line 1\n\nBody line 2\n",
		}, "\x1f"),
		strings.Join([]string{
			strings.Repeat("b", 40), "bbbbbbb", "",
			"Bob", "bob@example.com", "1600000000", "Bob", "bob@example.com", "1600000000",
			"", "Initial", "",
		}, "\x1f"),
	}, "\x00") + "\x00"
	commits := ParseLog(output)
	if len(commits) != 2 {
		t.Fatalf("commits %d", len(commits))
	}
	if commits[0].ShortSha != "aaaaaaa" || len(commits[0].Parents) != 2 || commits[0].Subject != "Merge feature" {
		t.Fatalf("%+v", commits[0])
	}
	if len(commits[0].Refs) != 3 || commits[0].Refs[0] != (CommitRef{Kind: "branch", Name: "main", Current: true}) {
		t.Fatalf("%+v", commits[0].Refs)
	}
	if len(commits[1].Parents) != 0 || commits[1].Subject != "Initial" || len(commits[1].Refs) != 0 {
		t.Fatalf("%+v", commits[1])
	}
}

func TestParseDecorationsDetachedHead(t *testing.T) {
	refs := ParseDecorations("HEAD, tag: v2")
	if len(refs) != 2 || refs[0] != (CommitRef{Kind: "head", Name: "HEAD", Current: true}) || refs[1] != (CommitRef{Kind: "tag", Name: "v2", Current: false}) {
		t.Fatalf("%+v", refs)
	}
}

func testCommit(sha string, parents []string) Commit {
	return Commit{Sha: sha, ShortSha: sha, Parents: parents, Subject: sha}
}

func TestLayoutGraphLinearHistoryIncludesRoot(t *testing.T) {
	rows := LayoutGraph([]Commit{testCommit("c", []string{"b"}), testCommit("b", []string{"a"}), testCommit("a", nil)})
	if len(rows) != 3 {
		t.Fatalf("rows %d", len(rows))
	}
	for i, row := range rows {
		if row.Lane != 0 || row.LaneCount != 1 {
			t.Fatalf("row %d %+v", i, row)
		}
	}
	if len(rows[2].Edges) != 0 {
		t.Fatalf("root edges %+v", rows[2].Edges)
	}
}

func TestLayoutGraphRootCommitAlone(t *testing.T) {
	rows := LayoutGraph([]Commit{testCommit("a", []string{})})
	if len(rows) != 1 || rows[0].Lane != 0 || len(rows[0].Edges) != 0 || rows[0].Merge {
		t.Fatalf("%+v", rows)
	}
}

func TestLayoutGraphMergeOpensASecondLane(t *testing.T) {
	rows := LayoutGraph([]Commit{
		testCommit("m", []string{"d", "f"}),
		testCommit("d", []string{"b"}),
		testCommit("f", []string{"b"}),
		testCommit("b", []string{"a"}),
		testCommit("a", []string{}),
	})
	if !rows[0].Merge || rows[0].LaneCount != 2 || rows[0].Incoming || len(rows[0].Joins) != 0 {
		t.Fatalf("%+v", rows[0])
	}
	if len(rows[0].Edges) != 2 || rows[0].Edges[0] != (GraphEdge{FromLane: 0, ToLane: 0, Color: 0}) || rows[0].Edges[1] != (GraphEdge{FromLane: 0, ToLane: 1, Color: 1}) {
		t.Fatalf("%+v", rows[0].Edges)
	}
	if !rows[1].Incoming || len(rows[1].Passing) != 1 || rows[1].Passing[0] != (GraphPassing{Lane: 1, Color: 1}) {
		t.Fatalf("%+v", rows[1])
	}
	if rows[2].Lane != 1 || rows[2].Color != 1 || !rows[2].Incoming || rows[2].LaneCount != 2 {
		t.Fatalf("%+v", rows[2])
	}
	if len(rows[3].Joins) != 1 || rows[3].Joins[0] != (GraphJoin{FromLane: 1, Color: 1}) {
		t.Fatalf("%+v", rows[3])
	}
	if rows[4].Lane != 0 || !rows[4].Incoming || rows[4].LaneCount != 1 || len(rows[4].Edges) != 0 {
		t.Fatalf("%+v", rows[4])
	}
}

func TestRelativeTime(t *testing.T) {
	now := time.Unix(1_700_000_000, 0)
	if got := RelativeTime(now.Unix()-10, now); got != "just now" {
		t.Fatal(got)
	}
	if got := RelativeTime(now.Unix()-300, now); got != "5 min ago" {
		t.Fatal(got)
	}
	if got := RelativeTime(now.Unix()-3600*5, now); got != "5 hours ago" {
		t.Fatal(got)
	}
	if got := RelativeTime(now.Unix()-86400*400, now); got != "1 year ago" {
		t.Fatal(got)
	}
}
