package core

import (
	"fmt"
	"regexp"
	"strings"

	"gt/config"
)

// BumpType represents a semantic version bump.
type BumpType int

const (
	BumpPatch BumpType = iota
	BumpMinor
	BumpMajor
)

func (b BumpType) String() string {
	switch b {
	case BumpPatch:
		return "patch"
	case BumpMinor:
		return "minor"
	case BumpMajor:
		return "major"
	default:
		return "unknown"
	}
}

// ParseBumpType parses a string into a BumpType.
func ParseBumpType(s string) (BumpType, error) {
	switch strings.ToLower(s) {
	case "patch":
		return BumpPatch, nil
	case "minor":
		return BumpMinor, nil
	case "major":
		return BumpMajor, nil
	default:
		return BumpPatch, fmt.Errorf("unknown bump type: %s (expected patch, minor, or major)", s)
	}
}

// SemVer represents a semantic version.
type SemVer struct {
	Major int
	Minor int
	Patch int
	Pre   string // e.g. "alpha.1", "beta.2", "rc.1", or ""
}

// ParseSemVer parses a semver string like "1.2.3" or "1.2.3-beta.1".
func ParseSemVer(s string) (SemVer, error) {
	s = strings.TrimPrefix(s, "v")
	s = strings.TrimSpace(s)

	var v SemVer
	parts := strings.SplitN(s, "-", 2)
	if len(parts) == 2 {
		v.Pre = parts[1]
	}

	mainParts := strings.Split(parts[0], ".")
	if len(mainParts) != 3 {
		return v, fmt.Errorf("invalid semver: %s", s)
	}

	if _, err := fmt.Sscanf(mainParts[0], "%d", &v.Major); err != nil {
		return v, fmt.Errorf("invalid major version: %s", mainParts[0])
	}
	if _, err := fmt.Sscanf(mainParts[1], "%d", &v.Minor); err != nil {
		return v, fmt.Errorf("invalid minor version: %s", mainParts[1])
	}
	if _, err := fmt.Sscanf(mainParts[2], "%d", &v.Patch); err != nil {
		return v, fmt.Errorf("invalid patch version: %s", mainParts[2])
	}

	return v, nil
}

// String formats a SemVer as "X.Y.Z" or "X.Y.Z-pre".
func (v SemVer) String() string {
	base := fmt.Sprintf("%d.%d.%d", v.Major, v.Minor, v.Patch)
	if v.Pre != "" {
		return base + "-" + v.Pre
	}
	return base
}

// Bump returns a new SemVer bumped by the given type.
func (v SemVer) Bump(bt BumpType) SemVer {
	next := SemVer{Major: v.Major, Minor: v.Minor, Patch: v.Patch}
	switch bt {
	case BumpMajor:
		next.Major++
		next.Minor = 0
		next.Patch = 0
	case BumpMinor:
		next.Minor++
		next.Patch = 0
	case BumpPatch:
		next.Patch++
	}
	return next
}

// CommitCategory represents a conventional commit type.
type CommitCategory int

const (
	CatFeature CommitCategory = iota
	CatFix
	CatPerf
	CatRefactor
	CatDocs
	CatChore
	CatBreaking
	CatOther
)

func (c CommitCategory) String() string {
	switch c {
	case CatFeature:
		return "Features"
	case CatFix:
		return "Bug Fixes"
	case CatPerf:
		return "Performance"
	case CatRefactor:
		return "Refactoring"
	case CatDocs:
		return "Documentation"
	case CatChore:
		return "Chores"
	case CatBreaking:
		return "BREAKING CHANGES"
	case CatOther:
		return "Other"
	default:
		return "Other"
	}
}

// Emoji returns the category emoji for changelog formatting.
func (c CommitCategory) Emoji() string {
	switch c {
	case CatFeature:
		return "+"
	case CatFix:
		return "~"
	case CatPerf:
		return "^"
	case CatRefactor:
		return "*"
	case CatDocs:
		return "#"
	case CatChore:
		return "-"
	case CatBreaking:
		return "!"
	case CatOther:
		return " "
	default:
		return " "
	}
}

// CategorizedCommit is a commit with its parsed category.
type CategorizedCommit struct {
	Hash     string
	Subject  string
	Category CommitCategory
	Scope    string // optional scope from "feat(scope): ..."
	Breaking bool   // "!" suffix or BREAKING CHANGE footer
}

var conventionalRe = regexp.MustCompile(`^(\w+)(?:\(([^)]+)\))?(!)?:\s*(.+)`)

// CategorizeCommit parses a conventional commit message.
func CategorizeCommit(hash, subject string) CategorizedCommit {
	cc := CategorizedCommit{Hash: hash, Subject: subject, Category: CatOther}

	matches := conventionalRe.FindStringSubmatch(subject)
	if matches == nil {
		return cc
	}

	commitType := strings.ToLower(matches[1])
	cc.Scope = matches[2]
	cc.Breaking = matches[3] == "!"
	cc.Subject = matches[4]

	switch commitType {
	case "feat":
		cc.Category = CatFeature
	case "fix":
		cc.Category = CatFix
	case "perf":
		cc.Category = CatPerf
	case "refactor":
		cc.Category = CatRefactor
	case "docs", "doc":
		cc.Category = CatDocs
	case "chore", "ci", "build", "style", "test":
		cc.Category = CatChore
	case "config":
		cc.Category = CatChore
	}

	if cc.Breaking {
		cc.Category = CatBreaking
	}

	return cc
}

// SuggestBump analyzes commits and suggests a bump type.
func SuggestBump(commits []CategorizedCommit) BumpType {
	hasFeature := false
	for _, c := range commits {
		if c.Breaking || c.Category == CatBreaking {
			return BumpMajor
		}
		if c.Category == CatFeature {
			hasFeature = true
		}
	}
	if hasFeature {
		return BumpMinor
	}
	return BumpPatch
}

// DetectDirtyComponents checks which components have changes since their last tag.
func DetectDirtyComponents(root string) ([]ComponentStatus, error) {
	comps := config.Components()
	statuses := make([]ComponentStatus, len(comps))

	for i, comp := range comps {
		version, err := ReadComponentVersion(root, comp)
		if err != nil {
			version = "?.?.?"
		}

		latestTag := LatestTag(root, comp)
		var changedFiles []string
		var commits []CategorizedCommit

		if latestTag != "" {
			changedFiles = DiffFilesSinceTag(root, latestTag)
			logLines := LogSinceTag(root, latestTag)
			for _, line := range logLines {
				parts := strings.SplitN(line, " ", 2)
				if len(parts) == 2 {
					commits = append(commits, CategorizeCommit(parts[0], parts[1]))
				}
			}
		}

		// Filter changed files to this component's dirty paths
		dirty := filterDirtyFiles(changedFiles, comp.DirtyPaths)

		statuses[i] = ComponentStatus{
			Component:    comp,
			Version:      version,
			LatestTag:    latestTag,
			ChangedFiles: dirty,
			Commits:      commits,
			IsDirty:      len(dirty) > 0,
		}

		if statuses[i].IsDirty && len(commits) > 0 {
			statuses[i].SuggestedBump = SuggestBump(commits)
		}
	}

	return statuses, nil
}

// ComponentStatus represents the current state of a component.
type ComponentStatus struct {
	Component     config.Component
	Version       string
	LatestTag     string
	ChangedFiles  []string
	Commits       []CategorizedCommit
	IsDirty       bool
	SuggestedBump BumpType
}

func filterDirtyFiles(files []string, dirtyPaths []string) []string {
	var result []string
	for _, f := range files {
		for _, dp := range dirtyPaths {
			if strings.HasPrefix(f, dp) || f == strings.TrimSuffix(dp, "/") {
				result = append(result, f)
				break
			}
		}
	}
	return result
}
