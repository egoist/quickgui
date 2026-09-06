package git

import "testing"

func TestParseNumstat(t *testing.T) {
	output := "12\t3\tsrc/app.go\x00-\t-\tlogo.png\x00"
	entries := ParseNumstat(output)
	if len(entries) != 2 || entries[0].Path != "src/app.go" || entries[0].Added == nil || *entries[0].Added != 12 {
		t.Fatalf("%+v", entries)
	}
	if entries[1].Added != nil || entries[1].Path != "logo.png" {
		t.Fatalf("%+v", entries[1])
	}
}

func TestOpenDiffAndSelectedLines(t *testing.T) {
	text := "diff --git a/a.go b/a.go\n--- a/a.go\n+++ b/a.go\n@@ -1,1 +1,2 @@\n context\n+added\n"
	diff := OpenDiff(text, false)
	if diff.Added != 1 || diff.RowCount == 0 {
		t.Fatalf("%+v", diff)
	}
	var lineIndex int
	for i, row := range diff.Rows {
		if row.Kind == "line" && row.LineKind == LineAdded {
			lineIndex = i
			break
		}
	}
	selected := diff.SelectedLines([][]int{{lineIndex, lineIndex}})
	if len(selected) != 1 || len(selected[0].Lines) != 1 {
		t.Fatalf("%+v", selected)
	}
	if diff.Render() == "" {
		t.Fatal("render")
	}
}

func TestBranchNameProblem(t *testing.T) {
	if BranchNameProblem("") == "" || BranchNameProblem("ok") != "" || BranchNameProblem("bad name") == "" {
		t.Fatal(BranchNameProblem("bad name"))
	}
}
