package cmd

import (
	"fmt"

	"github.com/spf13/cobra"

	"gt/config"
	"gt/core"
)

var deploySkipBuild bool

var deployCmd = &cobra.Command{
	Use:   "deploy",
	Short: "Deploy server to Oracle Cloud",
	Long: `Cross-compile, upload, and deploy gt-server to Oracle Cloud instance.

Examples:
  gt deploy
  gt deploy --skip-build`,
	RunE: func(cmd *cobra.Command, args []string) error {
		root, err := config.FindProjectRoot()
		if err != nil {
			return err
		}

		cfg := config.DefaultDeployConfig()

		fmt.Printf("\n  Deploying to %s (%s)\n\n", cfg.Domain, cfg.Host)

		err = core.ExecuteDeploy(core.DeployOpts{
			Root:      root,
			Config:    cfg,
			SkipBuild: deploySkipBuild,
			OnStep: func(step core.DeployStep, msg string) {
				fmt.Printf("  [%s] %s\n", step, msg)
			},
			OnOutput: func(line string) {
				if line != "" {
					fmt.Printf("    %s\n", line)
				}
			},
			OnError: func(step core.DeployStep, err error) {
				fmt.Printf("  [%s] ERROR: %v\n", step, err)
			},
		})

		if err != nil {
			return fmt.Errorf("deploy failed: %w", err)
		}

		fmt.Println()
		return nil
	},
}

func init() {
	deployCmd.Flags().BoolVar(&deploySkipBuild, "skip-build", false, "Skip cross-compilation, upload existing binary")
}
