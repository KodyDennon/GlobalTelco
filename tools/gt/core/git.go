package core

import (
	"fmt"
	"os/exec"
	"sort"
	"strings"

	"gt/config"
)

// GitExec runs a git command in the given directory and returns stdout.
func GitExec(dir string, args ...string) (string, error) {
	cmd := exec.Command("git", args...)
	cmd.Dir = dir
	out, err := cmd.Output()
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(string(out)), nil
}

// ListTags returns all tags matching a prefix, sorted by version (newest first).
func ListTags(dir, prefix string) []string {
	out, err := GitExec(dir, "tag", "--list", prefix+"*", "--sort=-v:refname")
	if err != nil || out == "" {
		return nil
	}
	return strings.Split(out, "\n")
}

// LatestTag returns the most recent tag for a component (checking legacy prefixes too).
func LatestTag(dir string, comp config.Component) string {
	prefixes := []string{comp.TagPrefix}
	prefixes = append(prefixes, comp.LegacyPrefixes...)
	return latestTagForPrefixes(dir, prefixes)
}

// latestTagForPrefixes finds the most recent tag across multiple prefixes.
func latestTagForPrefixes(dir string, prefixes []string) string {
	var allTags []string
	for _, prefix := range prefixes {
		tags := ListTags(dir, prefix)
		allTags = append(allTags, tags...)
	}
	if len(allTags) == 0 {
		return ""
	}

	// Pre-fetch commit timestamps to avoid O(n^2) git invocations during sort
	timestamps := make(map[string]string, len(allTags))
	for _, tag := range allTags {
		ts, _ := GitExec(dir, "log", "-1", "--format=%ct", tag)
		timestamps[tag] = ts
	}

	sort.Slice(allTags, func(i, j int) bool {
		return timestamps[allTags[i]] > timestamps[allTags[j]]
	})

	return allTags[0]
}

// DiffFilesSinceTag returns file paths changed between a tag and HEAD.
func DiffFilesSinceTag(dir, tag string) []string {
	out, err := GitExec(dir, "diff", "--name-only", tag+"..HEAD")
	if err != nil || out == "" {
		return nil
	}
	return strings.Split(out, "\n")
}

// LogSinceTag returns commit log lines (hash + subject) since a tag.
func LogSinceTag(dir, tag string) []string {
	out, err := GitExec(dir, "log", "--oneline", "--format=%h %s", tag+"..HEAD")
	if err != nil || out == "" {
		return nil
	}
	return strings.Split(out, "\n")
}

// LogAll returns recent commit log lines (hash + subject).
func LogAll(dir string, count int) []string {
	out, err := GitExec(dir, "log", "--oneline", "--format=%h %s", "-n", fmt.Sprintf("%d", count))
	if err != nil || out == "" {
		return nil
	}
	return strings.Split(out, "\n")
}

// StageFiles stages specific files for commit.
func StageFiles(dir string, files []string) error {
	args := append([]string{"add", "--"}, files...)
	_, err := GitExec(dir, args...)
	return err
}

// Commit creates a git commit with the given message.
func Commit(dir, message string) error {
	_, err := GitExec(dir, "commit", "-m", message)
	return err
}

// CreateTag creates an annotated git tag.
func CreateTag(dir, tag, message string) error {
	_, err := GitExec(dir, "tag", "-a", tag, "-m", message)
	return err
}

// Push pushes commits and optionally tags to remote.
func Push(dir string, tags bool) error {
	if _, err := GitExec(dir, "push"); err != nil {
		return err
	}
	if tags {
		if _, err := GitExec(dir, "push", "--tags"); err != nil {
			return err
		}
	}
	return nil
}

// PushTag pushes a specific tag to remote.
func PushTag(dir, tag string) error {
	_, err := GitExec(dir, "push", "origin", tag)
	return err
}

// IsClean returns true if the working tree has no uncommitted changes.
func IsClean(dir string) bool {
	out, err := GitExec(dir, "status", "--porcelain")
	return err == nil && out == ""
}

// CurrentBranch returns the current git branch name.
func CurrentBranch(dir string) string {
	out, err := GitExec(dir, "rev-parse", "--abbrev-ref", "HEAD")
	if err != nil {
		return "unknown"
	}
	return out
}

// HasRemote returns true if the current branch has a remote tracking branch.
func HasRemote(dir string) bool {
	_, err := GitExec(dir, "rev-parse", "--abbrev-ref", "@{u}")
	return err == nil
}

// TagExists returns true if a tag exists.
func TagExists(dir, tag string) bool {
	_, err := GitExec(dir, "rev-parse", tag)
	return err == nil
}

// AllTagsForComponent returns all tags for a component (including legacy).
func AllTagsForComponent(dir string, comp config.Component) []string {
	prefixes := []string{comp.TagPrefix}
	prefixes = append(prefixes, comp.LegacyPrefixes...)
	var allTags []string
	for _, prefix := range prefixes {
		tags := ListTags(dir, prefix)
		allTags = append(allTags, tags...)
	}
	return allTags
}
