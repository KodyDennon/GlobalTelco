package cmd

import (
	"fmt"

	"github.com/spf13/cobra"

	"gt/config"
	"gt/core"
)

var deploySkipBuild bool

var deployCmd = &cobra.Command{
	Use:   "deploy [component]",
	Short: "Deploy server (Oracle) or admin (Cloudflare)",
	Long: `Deploy a component. Defaults to server.
Components:
  server - Cross-compile, upload, and deploy to Oracle Cloud
  admin  - Build and deploy to Cloudflare Pages

Examples:
  gt deploy
  gt deploy admin
  gt deploy --skip-build`,
	Args: cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		root, err := config.FindProjectRoot()
		if err != nil {
			return err
		}

		compID := "server"
		if len(args) > 0 {
			compID = args[0]
		}

		if compID != "server" && compID != "admin" {
			return fmt.Errorf("invalid component for deploy: %s (use 'server' or 'admin')", compID)
		}

		cfg := config.DefaultDeployConfig()

		if compID == "server" {
			fmt.Printf("\n  Deploying server to %s (%s)\n\n", cfg.Domain, cfg.Host)
		} else {
			fmt.Printf("\n  Deploying admin panel to Cloudflare Pages\n\n")
		}

		err = core.ExecuteDeploy(core.DeployOpts{
			Root:        root,
			Config:      cfg,
			ComponentID: compID,
			SkipBuild:   deploySkipBuild,
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
