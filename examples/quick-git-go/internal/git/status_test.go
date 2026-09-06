package git

import (
	"strings"
	"testing"
)

func TestParseStatus(t *testing.T) {
	output := strings.Join([]string{
		"# branch.oid 1156c997a6001274d2579d00a5dbb606add3f528",
		"# branch.head main",
		"# branch.upstream origin/main",
		"# branch.ab +2 -1",
		"# stash 3",
		"1 .M N... 100644 100644 100644 422c2b7ab3b3c668038da977e4e93a5fc623169c 422c2b7ab3b3c668038da977e4e93a5fc623169c src/with space.txt",
		"1 A. N... 000000 100644 100644 0000000000000000000000000000000000000000 6372083f6a5f1b6d3e4c8f2a3b4c5d6e7f8a9b0c added.txt",
		"2 R. N... 100644 100644 100644 422c2b7ab3b3c668038da977e4e93a5fc623169c 422c2b7ab3b3c668038da977e4e93a5fc623169c R100 new name.txt",
		"old name.txt",
		"u UU N... 100644 100644 100644 100644 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb cccccccccccccccccccccccccccccccccccccccc conflict.txt",
		"1 .M S.M. 160000 160000 160000 dddddddddddddddddddddddddddddddddddddddd dddddddddddddddddddddddddddddddddddddddd vendor/lib",
		"? notes.md",
		"! build/out.o",
	}, "\x00") + "\x00"
	status := ParseStatus(output)
	if status.HeadSha != "1156c997a6001274d2579d00a5dbb606add3f528" || status.Branch != "main" || status.Detached {
		t.Fatalf("%+v", status)
	}
	if status.Upstream != "origin/main" || status.Ahead != 2 || status.Behind != 1 || !status.HasUpstreamCounts || status.StashCount != 3 {
		t.Fatalf("%+v", status)
	}
	if len(status.Entries) != 7 {
		t.Fatalf("entries %d", len(status.Entries))
	}
	if status.Entries[0].Path != "src/with space.txt" || status.Entries[2].OriginalPath != "old name.txt" {
		t.Fatal(status.Entries[0], status.Entries[2])
	}
	if status.Entries[2].Similarity == nil || *status.Entries[2].Similarity != 100 {
		t.Fatal(status.Entries[2].Similarity)
	}
	if status.Entries[3].Conflict == nil || status.Entries[3].Conflict.Stage3Sha != "cccccccccccccccccccccccccccccccccccccccc" {
		t.Fatal(status.Entries[3].Conflict)
	}
	if !status.Entries[4].Submodule {
		t.Fatal("submodule")
	}
	if IsClean(status) {
		t.Fatal("clean")
	}
	unstaged := UnstagedChanges(status)
	if len(unstaged) != 4 || unstaged[0].Code != "M" || unstaged[1].Code != "U" || !unstaged[1].Conflicted {
		t.Fatalf("%+v", unstaged)
	}
	staged := StagedChanges(status)
	if len(staged) != 2 || staged[0].Code != "A" || staged[1].ID != "staged:new name.txt" {
		t.Fatalf("%+v", staged)
	}
}

func TestParseStatusUnbornAndDetached(t *testing.T) {
	unborn := ParseStatus("# branch.oid (initial)\x00# branch.head main\x00? a\x00")
	if unborn.HeadSha != "" || unborn.Branch != "main" || unborn.HasUpstreamCounts {
		t.Fatalf("%+v", unborn)
	}
	detached := ParseStatus("# branch.oid abc\x00# branch.head (detached)\x00")
	if !detached.Detached || detached.Branch != "" || !IsClean(detached) {
		t.Fatalf("%+v", detached)
	}
}

func TestDescribeStatusCode(t *testing.T) {
	if DescribeStatusCode("?") != "Untracked" || DescribeStatusCode("R") != "Renamed" || DescribeStatusCode(".") != "Unchanged" {
		t.Fatal("labels")
	}
}
