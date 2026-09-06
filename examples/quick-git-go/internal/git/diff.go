package git

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
)

type DiffLineKind string

const (
	LineContext DiffLineKind = "context"
	LineAdded   DiffLineKind = "added"
	LineRemoved DiffLineKind = "removed"
)

type DiffLine struct {
	Kind          DiffLineKind
	Text          string
	OldLineNumber *int
	NewLineNumber *int
	NoNewline     bool
}

type DiffHunk struct {
	OldStart, OldLines, NewStart, NewLines int
	Heading                                string
	Lines                                  []DiffLine
}

type DiffFileKind string

const (
	FileModified DiffFileKind = "modified"
	FileAdded    DiffFileKind = "added"
	FileDeleted  DiffFileKind = "deleted"
	FileRenamed  DiffFileKind = "renamed"
	FileCopied   DiffFileKind = "copied"
)

type DiffFile struct {
	Kind       DiffFileKind
	OldPath    string
	NewPath    string
	Path       string
	Binary     bool
	OldMode    string
	NewMode    string
	Similarity *int
	Hunks      []DiffHunk
	Truncated  bool
}

type DiffRowKind string

type DiffRow struct {
	Kind          DiffRowKind
	FileIndex     int
	HunkIndex     int
	LineIndex     int
	LineKind      DiffLineKind
	Text          string
	OldLineNumber *int
	NewLineNumber *int
	NoNewline     bool
}

type Diff struct {
	Files    []DiffFile
	Rows     []DiffRow
	Added    int
	Removed  int
	Bytes    int
	RowCount int
}

var hunkHeaderRe = regexp.MustCompile(`^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@ ?(.*)$`)

func ParseDiff(output string, truncated bool) []DiffFile {
	lines := strings.Split(strings.ReplaceAll(output, "\r\n", "\n"), "\n")
	var files []DiffFile
	var current *DiffFile
	var hunk *DiffHunk
	oldNext, newNext := 1, 1
	flushHunk := func() {
		if hunk != nil && current != nil {
			current.Hunks = append(current.Hunks, *hunk)
			hunk = nil
		}
	}
	flushFile := func() {
		flushHunk()
		if current != nil {
			if current.Kind == FileAdded {
				current.OldPath = ""
			}
			if current.Kind == FileDeleted {
				current.NewPath = ""
			}
			if current.NewPath != "" {
				current.Path = current.NewPath
			} else {
				current.Path = current.OldPath
			}
			files = append(files, *current)
			current = nil
		}
	}
	for _, raw := range lines {
		if strings.HasPrefix(raw, "diff --git ") {
			flushFile()
			file := DiffFile{Kind: FileModified}
			paths := parseGitHeaderPaths(strings.TrimPrefix(raw, "diff --git "))
			file.OldPath = paths[0]
			file.NewPath = paths[1]
			current = &file
			continue
		}
		if current == nil {
			continue
		}
		if match := hunkHeaderRe.FindStringSubmatch(raw); match != nil {
			flushHunk()
			oldStart, _ := strconv.Atoi(match[1])
			oldLines := 1
			if match[2] != "" {
				oldLines, _ = strconv.Atoi(match[2])
			}
			newStart, _ := strconv.Atoi(match[3])
			newLines := 1
			if match[4] != "" {
				newLines, _ = strconv.Atoi(match[4])
			}
			hunk = &DiffHunk{OldStart: oldStart, OldLines: oldLines, NewStart: newStart, NewLines: newLines, Heading: match[5]}
			oldNext, newNext = oldStart, newStart
			if oldStart == 0 {
				oldNext = 1
			}
			if newStart == 0 {
				newNext = 1
			}
			continue
		}
		if hunk != nil {
			if strings.HasPrefix(raw, "\\") {
				if len(hunk.Lines) > 0 {
					hunk.Lines[len(hunk.Lines)-1].NoNewline = true
				}
				continue
			}
			if raw == "" {
				continue
			}
			marker := raw[0]
			text := ""
			if len(raw) > 1 {
				text = raw[1:]
			}
			line := DiffLine{Text: text, Kind: LineContext}
			switch marker {
			case '+':
				line.Kind = LineAdded
				n := newNext
				line.NewLineNumber = &n
				newNext++
			case '-':
				line.Kind = LineRemoved
				n := oldNext
				line.OldLineNumber = &n
				oldNext++
			case ' ':
				o, n := oldNext, newNext
				line.OldLineNumber = &o
				line.NewLineNumber = &n
				oldNext++
				newNext++
			default:
				continue
			}
			hunk.Lines = append(hunk.Lines, line)
			continue
		}
		switch {
		case strings.HasPrefix(raw, "new file mode "):
			current.Kind = FileAdded
			current.NewMode = strings.TrimPrefix(raw, "new file mode ")
		case strings.HasPrefix(raw, "deleted file mode "):
			current.Kind = FileDeleted
			current.OldMode = strings.TrimPrefix(raw, "deleted file mode ")
		case strings.HasPrefix(raw, "old mode "):
			current.OldMode = strings.TrimPrefix(raw, "old mode ")
		case strings.HasPrefix(raw, "new mode "):
			current.NewMode = strings.TrimPrefix(raw, "new mode ")
		case strings.HasPrefix(raw, "similarity index "):
			n, _ := strconv.Atoi(strings.TrimSuffix(strings.TrimPrefix(raw, "similarity index "), "%"))
			current.Similarity = &n
		case strings.HasPrefix(raw, "rename from "):
			current.Kind = FileRenamed
			current.OldPath = strings.TrimPrefix(raw, "rename from ")
		case strings.HasPrefix(raw, "rename to "):
			current.Kind = FileRenamed
			current.NewPath = strings.TrimPrefix(raw, "rename to ")
		case strings.HasPrefix(raw, "copy from "):
			current.Kind = FileCopied
			current.OldPath = strings.TrimPrefix(raw, "copy from ")
		case strings.HasPrefix(raw, "copy to "):
			current.Kind = FileCopied
			current.NewPath = strings.TrimPrefix(raw, "copy to ")
		case strings.HasPrefix(raw, "Binary files ") || strings.HasPrefix(raw, "GIT binary patch"):
			current.Binary = true
		case strings.HasPrefix(raw, "--- "):
			path := strings.TrimPrefix(raw, "--- ")
			if path == "/dev/null" {
				current.OldPath = ""
			} else {
				current.OldPath = strings.TrimPrefix(path, "a/")
			}
		case strings.HasPrefix(raw, "+++ "):
			path := strings.TrimPrefix(raw, "+++ ")
			if path == "/dev/null" {
				current.NewPath = ""
			} else {
				current.NewPath = strings.TrimPrefix(path, "b/")
			}
		}
	}
	flushFile()
	if truncated && len(files) > 0 {
		files[len(files)-1].Truncated = true
	}
	return files
}

func parseGitHeaderPaths(pair string) [2]string {
	at := strings.Index(pair, " b/")
	if at >= 0 && strings.HasPrefix(pair, "a/") {
		return [2]string{strings.TrimPrefix(pair[:at], "a/"), strings.TrimPrefix(pair[at+1:], "b/")}
	}
	return [2]string{pair, pair}
}

func OpenDiff(output string, truncated bool) *Diff {
	files := ParseDiff(output, truncated)
	diff := &Diff{Files: files, Bytes: len(output)}
	for fileIndex, file := range files {
		if len(files) > 1 {
			diff.Rows = append(diff.Rows, DiffRow{Kind: "file", FileIndex: fileIndex, Text: file.Path, LineKind: LineContext})
		}
		if file.Binary {
			diff.Rows = append(diff.Rows, DiffRow{Kind: "notice", FileIndex: fileIndex, Text: "Binary file", LineKind: LineContext})
			continue
		}
		for hunkIndex, hunk := range file.Hunks {
			diff.Rows = append(diff.Rows, DiffRow{
				Kind: "hunk", FileIndex: fileIndex, HunkIndex: hunkIndex,
				Text: formatHunkHeader(hunk), LineKind: LineContext,
			})
			for lineIndex, line := range hunk.Lines {
				if line.Kind == LineAdded {
					diff.Added++
				} else if line.Kind == LineRemoved {
					diff.Removed++
				}
				diff.Rows = append(diff.Rows, DiffRow{
					Kind: "line", FileIndex: fileIndex, HunkIndex: hunkIndex, LineIndex: lineIndex,
					LineKind: line.Kind, Text: line.Text, OldLineNumber: line.OldLineNumber,
					NewLineNumber: line.NewLineNumber, NoNewline: line.NoNewline,
				})
			}
		}
		if file.Truncated {
			diff.Rows = append(diff.Rows, DiffRow{Kind: "notice", FileIndex: fileIndex, Text: "Diff truncated: the file is too large to show in full.", LineKind: LineContext})
		}
	}
	diff.RowCount = len(diff.Rows)
	return diff
}

func formatHunkHeader(hunk DiffHunk) string {
	return fmt.Sprintf("@@ -%d,%d +%d,%d @@ %s", hunk.OldStart, hunk.OldLines, hunk.NewStart, hunk.NewLines, hunk.Heading)
}

type HunkSelection struct {
	HunkIndex int
	Lines     map[int]struct{}
}

func FormatPatch(file DiffFile, selections []HunkSelection, reverse bool) string {
	var body strings.Builder
	anyLine := false
	for _, selection := range selections {
		if selection.HunkIndex < 0 || selection.HunkIndex >= len(file.Hunks) {
			continue
		}
		hunk := file.Hunks[selection.HunkIndex]
		var lines []string
		oldCount, newCount := 0, 0
		for i, line := range hunk.Lines {
			_, selected := selection.Lines[i]
			if len(selection.Lines) == 0 {
				selected = true
			}
			switch line.Kind {
			case LineContext:
				lines = append(lines, " "+line.Text)
				oldCount++
				newCount++
			case LineAdded:
				if selected && !reverse {
					lines = append(lines, "+"+line.Text)
					newCount++
					anyLine = true
				} else if selected && reverse {
					lines = append(lines, "-"+line.Text)
					oldCount++
					anyLine = true
				} else if reverse {
					lines = append(lines, " "+line.Text)
					oldCount++
					newCount++
				}
			case LineRemoved:
				if selected && !reverse {
					lines = append(lines, "-"+line.Text)
					oldCount++
					anyLine = true
				} else if selected && reverse {
					lines = append(lines, "+"+line.Text)
					newCount++
					anyLine = true
				} else if !reverse {
					lines = append(lines, " "+line.Text)
					oldCount++
					newCount++
				}
			}
		}
		if len(lines) == 0 {
			continue
		}
		oldStart := hunk.OldStart
		if oldCount == 0 {
			oldStart = 0
		}
		newStart := hunk.NewStart
		if reverse {
			newStart = hunk.NewStart
		}
		if newCount == 0 {
			newStart = 0
		}
		fmt.Fprintf(&body, "@@ -%s +%s @@ %s\n", countSuffix(oldStart, oldCount), countSuffix(newStart, newCount), hunk.Heading)
		for _, line := range lines {
			body.WriteString(line)
			body.WriteByte('\n')
		}
	}
	if !anyLine {
		return ""
	}
	var header strings.Builder
	path := file.Path
	oldPath, newPath := file.OldPath, file.NewPath
	if oldPath == "" {
		oldPath = "/dev/null"
	} else {
		oldPath = "a/" + oldPath
	}
	if newPath == "" {
		newPath = "/dev/null"
	} else {
		newPath = "b/" + newPath
	}
	fmt.Fprintf(&header, "diff --git a/%s b/%s\n", path, path)
	if file.Kind == FileAdded && !reverse {
		fmt.Fprintf(&header, "new file mode %s\n", valueOr(file.NewMode, "100644"))
	}
	if file.Kind == FileDeleted && !reverse {
		fmt.Fprintf(&header, "deleted file mode %s\n", valueOr(file.OldMode, "100644"))
	}
	if file.Kind == FileRenamed && file.Similarity != nil {
		fmt.Fprintf(&header, "similarity index %d%%\nrename from %s\nrename to %s\n", *file.Similarity, file.OldPath, file.NewPath)
	}
	fmt.Fprintf(&header, "--- %s\n+++ %s\n", oldPath, newPath)
	return header.String() + body.String()
}

func countSuffix(start, count int) string {
	if count == 1 {
		return strconv.Itoa(start)
	}
	return fmt.Sprintf("%d,%d", start, count)
}

func valueOr(value, fallback string) string {
	if value == "" {
		return fallback
	}
	return value
}

func SyntheticDiffText(path, content string, binary bool) string {
	var b strings.Builder
	fmt.Fprintf(&b, "diff --git a/%s b/%s\nnew file mode 100644\n--- /dev/null\n+++ b/%s\n", path, path, path)
	if binary {
		b.WriteString("Binary files /dev/null and b/" + path + " differ\n")
		return b.String()
	}
	if content == "" {
		return b.String()
	}
	lines := strings.Split(strings.TrimSuffix(content, "\n"), "\n")
	fmt.Fprintf(&b, "@@ -0,0 +1,%d @@\n", len(lines))
	for _, line := range lines {
		b.WriteString("+")
		b.WriteString(line)
		b.WriteByte('\n')
	}
	if !strings.HasSuffix(content, "\n") && content != "" {
		b.WriteString("\\ No newline at end of file\n")
	}
	return b.String()
}

func LooksBinary(data []byte) bool {
	limit := len(data)
	if limit > 8000 {
		limit = 8000
	}
	for _, b := range data[:limit] {
		if b == 0 {
			return true
		}
	}
	return false
}

type SelectedLines struct {
	FileIndex int
	HunkIndex int
	Lines     map[int]struct{}
}

func (d *Diff) SelectedLines(ranges [][]int) []SelectedLines {
	if d == nil || len(ranges) == 0 {
		return nil
	}
	groups := map[string]int{}
	var found []SelectedLines
	for _, r := range ranges {
		if len(r) < 2 {
			continue
		}
		last := r[1]
		if last >= len(d.Rows) {
			last = len(d.Rows) - 1
		}
		start := r[0]
		if start < 0 {
			start = 0
		}
		for index := start; index <= last; index++ {
			row := d.Rows[index]
			if row.Kind != "line" || row.LineKind == LineContext {
				continue
			}
			key := fmt.Sprintf("%d:%d", row.FileIndex, row.HunkIndex)
			pos, ok := groups[key]
			if !ok {
				found = append(found, SelectedLines{FileIndex: row.FileIndex, HunkIndex: row.HunkIndex, Lines: map[int]struct{}{}})
				pos = len(found) - 1
				groups[key] = pos
			}
			found[pos].Lines[row.LineIndex] = struct{}{}
		}
	}
	return found
}

func (d *Diff) Render() string {
	if d == nil {
		return ""
	}
	var out []string
	for _, file := range d.Files {
		oldPath, newPath := file.OldPath, file.NewPath
		if oldPath == "" {
			oldPath = file.Path
		}
		if newPath == "" {
			newPath = file.Path
		}
		out = append(out, fmt.Sprintf("diff --git a/%s b/%s", oldPath, newPath))
		if file.Kind == FileAdded {
			out = append(out, "new file mode "+valueOr(file.NewMode, "100644"), "--- /dev/null", "+++ b/"+file.Path)
		}
		for _, hunk := range file.Hunks {
			out = append(out, formatHunkHeader(hunk))
			for _, line := range hunk.Lines {
				marker := " "
				switch line.Kind {
				case LineAdded:
					marker = "+"
				case LineRemoved:
					marker = "-"
				}
				out = append(out, marker+line.Text)
			}
		}
	}
	return strings.Join(out, "\n")
}

func (d *Diff) FileAt(index int) *DiffFile {
	if d == nil || index < 0 || index >= len(d.Files) {
		return nil
	}
	return &d.Files[index]
}

func (d *Diff) File(path string) *DiffFile {
	for i := range d.Files {
		if d.Files[i].Path == path {
			return &d.Files[i]
		}
	}
	if len(d.Files) == 1 {
		return &d.Files[0]
	}
	return nil
}
