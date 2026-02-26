package cmd

import (
	"fmt"
	"strings"

	"github.com/spf13/cobra"

	"gt/config"
	"gt/core"
)

var (
	releaseType   string
	releaseDryRun bool
	releasePush   bool
)

var releaseCmd = &cobra.Command{
	Use:   "release <component>",
	Short: "Release a component (engine, server, web, desktop)",
	Long: `Release a component by bumping its version, generating a changelog,
creating a git commit and tag, and optionally pushing.

Examples:
  gt release engine --type patch
  gt release server --type minor --push
  gt release web --type patch --dry-run`,
	Args: cobra.ExactArgs(1),
	ValidArgs: config.ComponentIDs(),
	RunE: func(cmd *cobra.Command, args []string) error {
		root, err := config.FindProjectRoot()
		if err != nil {
			return err
		}

		comp := config.FindComponent(args[0])
		if comp == nil {
			return fmt.Errorf("unknown component: %s (valid: %s)", args[0], strings.Join(config.ComponentIDs(), ", "))
		}

		bt, err := core.ParseBumpType(releaseType)
		if err != nil {
			return err
		}

		opts := core.ReleaseOpts{
			Root:      root,
			Component: *comp,
			BumpType:  bt,
			DryRun:    releaseDryRun,
			Push:      releasePush,
			OnStep: func(step core.ReleaseStep, msg string) {
				fmt.Printf("  [%s] %s\n", step, msg)
			},
			OnError: func(step core.ReleaseStep, err error) {
				fmt.Printf("  [%s] ERROR: %v\n", step, err)
			},
		}

		result, err := core.ExecuteRelease(opts)
		if err != nil {
			return err
		}

		fmt.Println()
		fmt.Printf("  Released %s: %s -> %s\n", comp.Name, result.OldVersion, result.NewVersion)
		fmt.Printf("  Tag: %s\n", result.Tag)
		if result.Pushed {
			fmt.Printf("  Pushed to remote\n")
		} else {
			fmt.Printf("  Not pushed (use --push to push)\n")
		}
		fmt.Println()

		return nil
	},
}

func init() {
	releaseCmd.Flags().StringVarP(&releaseType, "type", "t", "patch", "Bump type: patch, minor, or major")
	releaseCmd.Flags().BoolVar(&releaseDryRun, "dry-run", false, "Preview changes without modifying files")
	releaseCmd.Flags().BoolVar(&releasePush, "push", false, "Push commit and tag to remote after release")
}
