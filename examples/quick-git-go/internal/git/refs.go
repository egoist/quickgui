package git

import (
	"regexp"
	"strconv"
	"strings"
)

const ForEachRefFormat = "%(refname)%00%(objectname)%00%(objectname:short)%00%(upstream:short)%00%(upstream:track)%00%(creatordate:unix)%00%(HEAD)%00%(subject)%00%(worktreepath)"
const StashFormat = "%gd%x1f%H%x1f%at%x1f%gs"

type BranchRef struct {
	Name          string
	FullName      string
	Sha           string
	ShortSha      string
	Remote        bool
	Upstream      string
	Ahead         int
	Behind        int
	UpstreamGone  bool
	CommitterTime int64
	Current       bool
	Subject       string
	WorktreePath  string
}

type TagRef struct {
	Name          string
	Sha           string
	ShortSha      string
	CommitterTime int64
	Subject       string
}

type RefCollections struct {
	Local  []BranchRef
	Remote []BranchRef
	Tags   []TagRef
}

func ParseRefs(output string) RefCollections {
	collections := RefCollections{}
	for _, line := range strings.Split(output, "\n") {
		if line == "" {
			continue
		}
		parts := strings.Split(line, "\x00")
		if len(parts) < 9 {
			continue
		}
		fullName, sha, shortSha, upstream, track, timeText, head, subject, worktreePath := parts[0], parts[1], parts[2], parts[3], parts[4], parts[5], parts[6], parts[7], parts[8]
		committerTime, _ := strconv.ParseInt(timeText, 10, 64)
		if strings.HasPrefix(fullName, "refs/tags/") {
			collections.Tags = append(collections.Tags, TagRef{
				Name: fullName[10:], Sha: sha, ShortSha: shortSha, CommitterTime: committerTime, Subject: subject,
			})
			continue
		}
		isRemote := strings.HasPrefix(fullName, "refs/remotes/")
		name := fullName
		if isRemote {
			name = strings.TrimPrefix(fullName, "refs/remotes/")
			if strings.HasSuffix(name, "/HEAD") {
				continue
			}
		} else {
			name = strings.TrimPrefix(fullName, "refs/heads/")
		}
		ahead, behind, gone := parseTrack(track)
		branch := BranchRef{
			Name: name, FullName: fullName, Sha: sha, ShortSha: shortSha, Remote: isRemote,
			Upstream: upstream, Ahead: ahead, Behind: behind, UpstreamGone: gone,
			CommitterTime: committerTime, Current: head == "*", Subject: subject, WorktreePath: worktreePath,
		}
		if isRemote {
			collections.Remote = append(collections.Remote, branch)
		} else {
			collections.Local = append(collections.Local, branch)
		}
	}
	return collections
}

func parseTrack(track string) (ahead, behind int, gone bool) {
	if track == "" {
		return
	}
	if strings.Contains(track, "gone") {
		gone = true
	}
	if match := regexp.MustCompile(`ahead (\d+)`).FindStringSubmatch(track); match != nil {
		ahead, _ = strconv.Atoi(match[1])
	}
	if match := regexp.MustCompile(`behind (\d+)`).FindStringSubmatch(track); match != nil {
		behind, _ = strconv.Atoi(match[1])
	}
	return
}

type StashEntry struct {
	Ref     string
	Index   int
	Sha     string
	Time    int64
	Message string
	Branch  string
	Summary string
}

var stashIndex = regexp.MustCompile(`\{(\d+)\}`)
var stashNamed = regexp.MustCompile(`(?s)^(?:WIP on|On) ([^:]+): (.*)$`)

func ParseStashes(output string) []StashEntry {
	var stashes []StashEntry
	for _, record := range SplitNul(output) {
		parts := strings.Split(record, "\x1f")
		if len(parts) < 4 || parts[0] == "" || parts[1] == "" {
			continue
		}
		index := len(stashes)
		if match := stashIndex.FindStringSubmatch(parts[0]); match != nil {
			index, _ = strconv.Atoi(match[1])
		}
		entry := StashEntry{Ref: parts[0], Index: index, Sha: parts[1], Message: parts[3], Summary: parts[3]}
		entry.Time, _ = strconv.ParseInt(parts[2], 10, 64)
		if named := stashNamed.FindStringSubmatch(parts[3]); named != nil {
			entry.Branch = named[1]
			entry.Summary = named[2]
		}
		stashes = append(stashes, entry)
	}
	return stashes
}

func BranchNameProblem(name string) string {
	if name == "" {
		return "Enter a branch name."
	}
	if strings.HasPrefix(name, "-") {
		return "A branch name cannot start with a dash."
	}
	if name == "HEAD" {
		return "HEAD is not a valid branch name."
	}
	if strings.ContainsAny(name, " \t\n~^:?*[\\\x7f") {
		return "Branch names cannot contain spaces or ~ ^ : ? * [ \\."
	}
	if strings.Contains(name, "..") || strings.Contains(name, "@{") {
		return "Branch names cannot contain .. or @{."
	}
	if strings.HasPrefix(name, "/") || strings.HasSuffix(name, "/") || strings.Contains(name, "//") {
		return "Branch names cannot start or end with a slash."
	}
	if strings.HasSuffix(name, ".") || strings.HasSuffix(name, ".lock") {
		return "Branch name components cannot start with a dot or end with .lock."
	}
	for _, part := range strings.Split(name, "/") {
		if strings.HasPrefix(part, ".") {
			return "Branch name components cannot start with a dot or end with .lock."
		}
	}
	return ""
}
