package git

import (
	"fmt"
	"strconv"
	"strings"
	"time"
)

const LogFormat = "%H%x1f%h%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ce%x1f%ct%x1f%D%x1f%s%x1f%b"

type CommitRefKind string

type CommitRef struct {
	Kind    CommitRefKind
	Name    string
	Current bool
}

type Commit struct {
	Sha            string
	ShortSha       string
	Parents        []string
	AuthorName     string
	AuthorEmail    string
	AuthorTime     int64
	CommitterName  string
	CommitterEmail string
	CommitterTime  int64
	Refs           []CommitRef
	Subject        string
	Body           string
}

func ParseLog(output string) []Commit {
	var commits []Commit
	for _, record := range SplitNul(output) {
		parts := strings.Split(record, "\x1f")
		if len(parts) < 12 {
			continue
		}
		parents := []string{}
		for _, parent := range strings.Split(parts[2], " ") {
			if parent != "" {
				parents = append(parents, parent)
			}
		}
		authorTime, _ := strconv.ParseInt(parts[5], 10, 64)
		committerTime, _ := strconv.ParseInt(parts[8], 10, 64)
		commits = append(commits, Commit{
			Sha: strings.TrimSpace(parts[0]), ShortSha: parts[1], Parents: parents,
			AuthorName: parts[3], AuthorEmail: parts[4], AuthorTime: authorTime,
			CommitterName: parts[6], CommitterEmail: parts[7], CommitterTime: committerTime,
			Refs: ParseDecorations(parts[9]), Subject: parts[10],
			Body: strings.TrimRight(strings.Join(parts[11:], "\x1f"), "\n"),
		})
	}
	return commits
}

func ParseDecorations(decorations string) []CommitRef {
	var refs []CommitRef
	trimmed := strings.TrimSpace(decorations)
	if trimmed == "" {
		return refs
	}
	for _, raw := range strings.Split(trimmed, ", ") {
		name := strings.TrimSpace(raw)
		if name == "" {
			continue
		}
		if name == "HEAD" {
			refs = append(refs, CommitRef{Kind: "head", Name: "HEAD", Current: true})
			continue
		}
		current := false
		if arrow := strings.Index(name, " -> "); arrow >= 0 {
			current = true
			name = name[arrow+4:]
		}
		switch {
		case strings.HasPrefix(name, "tag: "):
			refs = append(refs, CommitRef{Kind: "tag", Name: name[5:], Current: current})
		case strings.HasPrefix(name, "refs/remotes/") || (strings.Contains(name, "/") && !strings.HasPrefix(name, "refs/")):
			refs = append(refs, CommitRef{Kind: "remote", Name: strings.TrimPrefix(name, "refs/remotes/"), Current: current})
		case strings.HasPrefix(name, "refs/"):
			refs = append(refs, CommitRef{Kind: "other", Name: name, Current: current})
		default:
			refs = append(refs, CommitRef{Kind: "branch", Name: name, Current: current})
		}
	}
	return refs
}

type GraphEdge struct {
	FromLane, ToLane, Color int
}

type GraphJoin struct {
	FromLane, Color int
}

type GraphPassing struct {
	Lane, Color int
}

type GraphRow struct {
	Lane, Color, LaneCount int
	Incoming, Merge        bool
	Joins                  []GraphJoin
	Passing                []GraphPassing
	Edges                  []GraphEdge
}

func LayoutGraph(commits []Commit) []GraphRow {
	type laneEntry struct {
		sha   string
		color int
	}
	rows := make([]GraphRow, 0, len(commits))
	lanes := []*laneEntry{}
	nextColor := 0
	claim := func(sha string, color int) int {
		for i, entry := range lanes {
			if entry == nil {
				lanes[i] = &laneEntry{sha, color}
				return i
			}
		}
		lanes = append(lanes, &laneEntry{sha, color})
		return len(lanes) - 1
	}
	for _, commit := range commits {
		lane := -1
		for i, entry := range lanes {
			if entry != nil && entry.sha == commit.Sha {
				lane = i
				break
			}
		}
		incoming := lane >= 0
		var color int
		if lane < 0 {
			color = nextColor
			nextColor++
			lane = claim(commit.Sha, color)
		} else {
			color = lanes[lane].color
		}
		var joins []GraphJoin
		for i, entry := range lanes {
			if i != lane && entry != nil && entry.sha == commit.Sha {
				joins = append(joins, GraphJoin{FromLane: i, Color: entry.color})
				lanes[i] = nil
			}
		}
		var passing []GraphPassing
		for i, entry := range lanes {
			if entry != nil && i != lane {
				passing = append(passing, GraphPassing{Lane: i, Color: entry.color})
			}
		}
		laneCountBefore := len(lanes)
		var edges []GraphEdge
		if len(commit.Parents) == 0 {
			lanes[lane] = nil
		} else {
			lanes[lane] = &laneEntry{sha: commit.Parents[0], color: color}
			edges = append(edges, GraphEdge{FromLane: lane, ToLane: lane, Color: color})
		}
		for _, parent := range commit.Parents[1:] {
			existing := -1
			for i, entry := range lanes {
				if entry != nil && entry.sha == parent {
					existing = i
					break
				}
			}
			if existing >= 0 {
				edges = append(edges, GraphEdge{FromLane: lane, ToLane: existing, Color: lanes[existing].color})
			} else {
				parentColor := nextColor
				nextColor++
				parentLane := claim(parent, parentColor)
				edges = append(edges, GraphEdge{FromLane: lane, ToLane: parentLane, Color: parentColor})
			}
		}
		for len(lanes) > 0 && lanes[len(lanes)-1] == nil {
			lanes = lanes[:len(lanes)-1]
		}
		laneCount := laneCountBefore
		if len(lanes) > laneCount {
			laneCount = len(lanes)
		}
		if lane+1 > laneCount {
			laneCount = lane + 1
		}
		rows = append(rows, GraphRow{
			Lane: lane, Color: color, Incoming: incoming, Joins: joins, Passing: passing,
			Edges: edges, LaneCount: laneCount, Merge: len(commit.Parents) > 1,
		})
	}
	return rows
}

func RelativeTime(unixSeconds int64, now time.Time) string {
	seconds := int(now.Unix() - unixSeconds)
	if seconds < 0 {
		seconds = 0
	}
	if seconds < 45 {
		return "just now"
	}
	minutes := int(float64(seconds)/60 + 0.5)
	if minutes < 60 {
		return fmt.Sprintf("%d min ago", minutes)
	}
	hours := int(float64(minutes)/60 + 0.5)
	if hours < 24 {
		unit := "hours"
		if hours == 1 {
			unit = "hour"
		}
		return fmt.Sprintf("%d %s ago", hours, unit)
	}
	days := int(float64(hours)/24 + 0.5)
	if days < 30 {
		unit := "days"
		if days == 1 {
			unit = "day"
		}
		return fmt.Sprintf("%d %s ago", days, unit)
	}
	months := int(float64(days)/30 + 0.5)
	if months < 12 {
		unit := "months"
		if months == 1 {
			unit = "month"
		}
		return fmt.Sprintf("%d %s ago", months, unit)
	}
	years := int(float64(days)/365 + 0.5)
	unit := "years"
	if years == 1 {
		unit = "year"
	}
	return fmt.Sprintf("%d %s ago", years, unit)
}

func AbsoluteTime(unixSeconds int64) string {
	date := time.Unix(unixSeconds, 0)
	return date.Format("2006-01-02 15:04")
}
