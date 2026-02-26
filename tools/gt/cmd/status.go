package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"

	"gt/config"
	"gt/core"
)

var statusJSON bool

var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show component version status",
	RunE: func(cmd *cobra.Command, args []string) error {
		root, err := config.FindProjectRoot()
		if err != nil {
			return err
		}

		statuses, err := core.DetectDirtyComponents(root)
		if err != nil {
			return err
		}

		if statusJSON {
			return printStatusJSON(root, statuses)
		}

		branch := core.CurrentBranch(root)
		clean := core.IsClean(root)

		fmt.Printf("\n  GlobalTelco Version Status\n")
		fmt.Printf("  Branch: %s", branch)
		if clean {
			fmt.Printf("  (clean)\n")
		} else {
			fmt.Printf("  (dirty)\n")
		}
		fmt.Println()

		// Header
		fmt.Printf("  %-10s %-12s %-20s %-8s %-10s %s\n",
			"Component", "Version", "Latest Tag", "Dirty", "Suggest", "Changes")
		fmt.Printf("  %s\n", strings.Repeat("-", 80))

		for _, s := range statuses {
			dirty := " "
			if s.IsDirty {
				dirty = "*"
			}

			tag := s.LatestTag
			if tag == "" {
				tag = "(none)"
			}

			suggest := ""
			if s.IsDirty {
				suggest = s.SuggestedBump.String()
			}

			changes := ""
			if len(s.ChangedFiles) > 0 {
				changes = fmt.Sprintf("%d files", len(s.ChangedFiles))
			}

			fmt.Printf("  %-10s %-12s %-20s %-8s %-10s %s\n",
				s.Component.Name, s.Version, tag, dirty, suggest, changes)
		}

		fmt.Println()
		return nil
	},
}

type jsonStatus struct {
	Branch     string            `json:"branch"`
	Clean      bool              `json:"clean"`
	Components []jsonComponent   `json:"components"`
}

type jsonComponent struct {
	ID            string `json:"id"`
	Name          string `json:"name"`
	Version       string `json:"version"`
	LatestTag     string `json:"latest_tag"`
	Dirty         bool   `json:"dirty"`
	SuggestedBump string `json:"suggested_bump,omitempty"`
	ChangedFiles  int    `json:"changed_files"`
	Commits       int    `json:"commits"`
}

func printStatusJSON(root string, statuses []core.ComponentStatus) error {
	result := jsonStatus{
		Branch: core.CurrentBranch(root),
		Clean:  core.IsClean(root),
	}

	for _, s := range statuses {
		jc := jsonComponent{
			ID:           s.Component.ID,
			Name:         s.Component.Name,
			Version:      s.Version,
			LatestTag:    s.LatestTag,
			Dirty:        s.IsDirty,
			ChangedFiles: len(s.ChangedFiles),
			Commits:      len(s.Commits),
		}
		if s.IsDirty {
			jc.SuggestedBump = s.SuggestedBump.String()
		}
		result.Components = append(result.Components, jc)
	}

	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	return enc.Encode(result)
}

func init() {
	statusCmd.Flags().BoolVar(&statusJSON, "json", false, "Output as JSON (for CI/scripting)")
}
