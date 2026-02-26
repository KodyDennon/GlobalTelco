package core

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// GenerateChangelog creates a markdown changelog section from categorized commits.
func GenerateChangelog(version string, commits []CategorizedCommit) string {
	if len(commits) == 0 {
		return ""
	}

	// Group commits by category
	groups := make(map[CommitCategory][]CategorizedCommit)
	order := []CommitCategory{CatBreaking, CatFeature, CatFix, CatPerf, CatRefactor, CatDocs, CatChore, CatOther}
	for _, c := range commits {
		groups[c.Category] = append(groups[c.Category], c)
	}

	var sb strings.Builder
	sb.WriteString(fmt.Sprintf("## %s (%s)\n\n", version, time.Now().Format("2006-01-02")))

	for _, cat := range order {
		items, ok := groups[cat]
		if !ok || len(items) == 0 {
			continue
		}
		sb.WriteString(fmt.Sprintf("### %s\n\n", cat.String()))
		for _, c := range items {
			scope := ""
			if c.Scope != "" {
				scope = fmt.Sprintf("**%s:** ", c.Scope)
			}
			sb.WriteString(fmt.Sprintf("- %s%s (%s)\n", scope, c.Subject, c.Hash))
		}
		sb.WriteString("\n")
	}

	return sb.String()
}

// UpdateChangelogFile prepends a new section to the component's CHANGELOG.md.
func UpdateChangelogFile(root string, componentID string, section string) error {
	if section == "" {
		return nil
	}

	filename := "CHANGELOG.md"
	if componentID != "engine" {
		filename = fmt.Sprintf("CHANGELOG-%s.md", componentID)
	}
	path := filepath.Join(root, filename)

	existing := ""
	if data, err := os.ReadFile(path); err == nil {
		existing = string(data)
	}

	var content string
	if existing == "" {
		content = fmt.Sprintf("# Changelog\n\n%s", section)
	} else {
		// Insert after the first "# Changelog" header line
		headerEnd := strings.Index(existing, "\n")
		if headerEnd == -1 {
			content = existing + "\n\n" + section
		} else {
			content = existing[:headerEnd+1] + "\n" + section + existing[headerEnd+1:]
		}
	}

	return os.WriteFile(path, []byte(content), 0644)
}
