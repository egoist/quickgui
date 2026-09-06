package git

import (
	"regexp"
	"strconv"
	"strings"
)

type StatusCode string

type StatusEntryKind string

const (
	KindOrdinary  StatusEntryKind = "ordinary"
	KindRenamed   StatusEntryKind = "renamed"
	KindCopied    StatusEntryKind = "copied"
	KindUnmerged  StatusEntryKind = "unmerged"
	KindUntracked StatusEntryKind = "untracked"
	KindIgnored   StatusEntryKind = "ignored"
)

type ConflictInfo struct {
	Stage1Mode, Stage2Mode, Stage3Mode string
	Stage1Sha, Stage2Sha, Stage3Sha    string
}

type StatusEntry struct {
	Kind         StatusEntryKind
	Path         string
	OriginalPath string
	Index        StatusCode
	Worktree     StatusCode
	Submodule    bool
	Similarity   *int
	HeadMode     string
	IndexMode    string
	WorktreeMode string
	HeadSha      string
	IndexSha     string
	Conflict     *ConflictInfo
}

type RepositoryStatus struct {
	HeadSha           string
	Branch            string
	Detached          bool
	Upstream          string
	Ahead             int
	Behind            int
	HasUpstreamCounts bool
	StashCount        int
	Entries           []StatusEntry
}

func SplitNul(text string) []string {
	if text == "" {
		return nil
	}
	parts := strings.Split(text, "\x00")
	if len(parts) > 0 && parts[len(parts)-1] == "" {
		parts = parts[:len(parts)-1]
	}
	return parts
}

func ParseStatus(output string) RepositoryStatus {
	status := RepositoryStatus{}
	records := SplitNul(output)
	for index := 0; index < len(records); index++ {
		record := records[index]
		if strings.HasPrefix(record, "# ") {
			parseHeader(record[2:], &status)
			continue
		}
		if record == "" {
			continue
		}
		switch record[0] {
		case '1':
			if entry, ok := parseOrdinary(record); ok {
				status.Entries = append(status.Entries, entry)
			}
		case '2':
			index++
			original := ""
			if index < len(records) {
				original = records[index]
			}
			if entry, ok := parseRenamed(record, original); ok {
				status.Entries = append(status.Entries, entry)
			}
		case 'u':
			if entry, ok := parseUnmerged(record); ok {
				status.Entries = append(status.Entries, entry)
			}
		case '?':
			status.Entries = append(status.Entries, untrackedEntry(record[2:], KindUntracked, "?"))
		case '!':
			status.Entries = append(status.Entries, untrackedEntry(record[2:], KindIgnored, "!"))
		}
	}
	return status
}

var abHeader = regexp.MustCompile(`^\+(\d+) -(\d+)$`)

func parseHeader(line string, status *RepositoryStatus) {
	key, value, _ := strings.Cut(line, " ")
	switch key {
	case "branch.oid":
		if value != "(initial)" {
			status.HeadSha = value
		}
	case "branch.head":
		if value == "(detached)" {
			status.Detached = true
		} else {
			status.Branch = value
		}
	case "branch.upstream":
		status.Upstream = value
	case "branch.ab":
		if match := abHeader.FindStringSubmatch(value); match != nil {
			status.Ahead, _ = strconv.Atoi(match[1])
			status.Behind, _ = strconv.Atoi(match[2])
			status.HasUpstreamCounts = true
		}
	case "stash":
		if count, err := strconv.Atoi(value); err == nil && count >= 0 {
			status.StashCount = count
		}
	}
}

func fields(line string, count int) []string {
	parts := make([]string, 0, count+1)
	rest := line
	for i := 0; i < count; i++ {
		space := strings.IndexByte(rest, ' ')
		if space < 0 {
			return nil
		}
		parts = append(parts, rest[:space])
		rest = rest[space+1:]
	}
	return append(parts, rest)
}

func code(value string) StatusCode {
	switch value {
	case ".", "M", "T", "A", "D", "R", "C", "U", "?", "!":
		return StatusCode(value)
	default:
		return "."
	}
}

func xyCodes(xy string) (StatusCode, StatusCode) {
	if len(xy) < 2 {
		return ".", "."
	}
	return code(xy[:1]), code(xy[1:2])
}

func parseOrdinary(record string) (StatusEntry, bool) {
	parts := fields(record, 8)
	if parts == nil {
		return StatusEntry{}, false
	}
	index, worktree := xyCodes(parts[1])
	return StatusEntry{
		Kind: KindOrdinary, Path: parts[8], Index: index, Worktree: worktree,
		Submodule: parts[2] != "N...", HeadMode: parts[3], IndexMode: parts[4],
		WorktreeMode: parts[5], HeadSha: parts[6], IndexSha: parts[7],
	}, true
}

func parseRenamed(record, originalPath string) (StatusEntry, bool) {
	parts := fields(record, 9)
	if parts == nil {
		return StatusEntry{}, false
	}
	score := parts[8]
	index, worktree := xyCodes(parts[1])
	entry := StatusEntry{
		Kind: KindRenamed, Path: parts[9], OriginalPath: originalPath,
		Index: index, Worktree: worktree, Submodule: parts[2] != "N...",
		HeadMode: parts[3], IndexMode: parts[4], WorktreeMode: parts[5],
		HeadSha: parts[6], IndexSha: parts[7],
	}
	if strings.HasPrefix(score, "C") {
		entry.Kind = KindCopied
	}
	if len(score) > 1 {
		if n, err := strconv.Atoi(score[1:]); err == nil {
			entry.Similarity = &n
		}
	}
	return entry, true
}

func parseUnmerged(record string) (StatusEntry, bool) {
	parts := fields(record, 10)
	if parts == nil {
		return StatusEntry{}, false
	}
	index, worktree := xyCodes(parts[1])
	return StatusEntry{
		Kind: KindUnmerged, Path: parts[10], Index: index, Worktree: worktree,
		Submodule: parts[2] != "N...", HeadMode: parts[4], IndexMode: parts[4],
		WorktreeMode: parts[6], HeadSha: parts[8], IndexSha: parts[8],
		Conflict: &ConflictInfo{
			Stage1Mode: parts[3], Stage2Mode: parts[4], Stage3Mode: parts[5],
			Stage1Sha: parts[7], Stage2Sha: parts[8], Stage3Sha: parts[9],
		},
	}, true
}

func untrackedEntry(path string, kind StatusEntryKind, mark StatusCode) StatusEntry {
	zeros := strings.Repeat("0", 40)
	return StatusEntry{
		Kind: kind, Path: path, Index: mark, Worktree: mark,
		HeadMode: "000000", IndexMode: "000000", WorktreeMode: "000000",
		HeadSha: zeros, IndexSha: zeros,
	}
}

type ChangeItem struct {
	ID           string
	Entry        StatusEntry
	Path         string
	OriginalPath string
	Code         StatusCode
	Staged       bool
	Conflicted   bool
}

func UnstagedChanges(status RepositoryStatus) []ChangeItem {
	var items []ChangeItem
	for _, entry := range status.Entries {
		switch entry.Kind {
		case KindIgnored:
			continue
		case KindUnmerged:
			items = append(items, changeItem(entry, "U", false, true))
		case KindUntracked:
			items = append(items, changeItem(entry, "?", false, false))
		default:
			if entry.Worktree != "." {
				items = append(items, changeItem(entry, entry.Worktree, false, false))
			}
		}
	}
	return items
}

func StagedChanges(status RepositoryStatus) []ChangeItem {
	var items []ChangeItem
	for _, entry := range status.Entries {
		if entry.Kind == KindUntracked || entry.Kind == KindIgnored || entry.Kind == KindUnmerged {
			continue
		}
		if entry.Index != "." {
			items = append(items, changeItem(entry, entry.Index, true, false))
		}
	}
	return items
}

func changeItem(entry StatusEntry, code StatusCode, staged, conflicted bool) ChangeItem {
	prefix := "unstaged:"
	if staged {
		prefix = "staged:"
	}
	return ChangeItem{
		ID: prefix + entry.Path, Entry: entry, Path: entry.Path,
		OriginalPath: entry.OriginalPath, Code: code, Staged: staged, Conflicted: conflicted,
	}
}

func DescribeStatusCode(code StatusCode) string {
	switch code {
	case "M":
		return "Modified"
	case "T":
		return "Type changed"
	case "A":
		return "Added"
	case "D":
		return "Deleted"
	case "R":
		return "Renamed"
	case "C":
		return "Copied"
	case "U":
		return "Conflicted"
	case "?":
		return "Untracked"
	case "!":
		return "Ignored"
	default:
		return "Unchanged"
	}
}

func IsClean(status RepositoryStatus) bool {
	for _, entry := range status.Entries {
		if entry.Kind != KindIgnored {
			return false
		}
	}
	return true
}
