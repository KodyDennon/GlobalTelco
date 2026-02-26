package cmd

import (
	"fmt"

	"github.com/spf13/cobra"

	"gt/config"
	"gt/core"
)

var validateCmd = &cobra.Command{
	Use:   "validate",
	Short: "Check version consistency across all components",
	RunE: func(cmd *cobra.Command, args []string) error {
		root, err := config.FindProjectRoot()
		if err != nil {
			return err
		}

		fmt.Print("\n  Validating version consistency...\n\n")

		allOk := true
		for _, comp := range config.Components() {
			version, mismatches, err := core.ValidateComponentVersions(root, comp)
			if err != nil {
				fmt.Printf("  [FAIL] %s: %v\n", comp.Name, err)
				allOk = false
				continue
			}

			if len(mismatches) > 0 {
				fmt.Printf("  [FAIL] %s (v%s):\n", comp.Name, version)
				for _, m := range mismatches {
					fmt.Printf("         %s\n", m)
				}
				allOk = false
			} else {
				fmt.Printf("  [ OK ] %s: v%s\n", comp.Name, version)
			}
		}

		fmt.Println()

		if !allOk {
			return fmt.Errorf("validation failed — version mismatches detected")
		}

		fmt.Println("  All components consistent.")
		fmt.Println()
		return nil
	},
}
