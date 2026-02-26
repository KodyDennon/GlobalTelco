package cmd

import (
	"fmt"
	"strings"

	"github.com/spf13/cobra"

	"gt/config"
	"gt/core"
)

var buildCmd = &cobra.Command{
	Use:   "build [component]",
	Short: "Build a component (engine, server, web, desktop)",
	Long: `Run the build pipeline for a component.

Examples:
  gt build web
  gt build engine
  gt build server`,
	Args:      cobra.ExactArgs(1),
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

		fmt.Printf("\n  Building %s...\n\n", comp.Name)

		err = core.ExecuteBuildSync(root, comp.ID, func(line string) {
			fmt.Println("  " + line)
		})

		if err != nil {
			fmt.Println()
			return fmt.Errorf("build failed: %w", err)
		}

		fmt.Printf("\n  Build complete: %s\n\n", comp.Name)
		return nil
	},
}
