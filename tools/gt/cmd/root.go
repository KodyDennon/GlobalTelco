package cmd

import (
	"fmt"
	"os"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/spf13/cobra"

	"gt/config"
	"gt/core"
	"gt/tui"
)

var rootCmd = &cobra.Command{
	Use:   "gt",
	Short: "GlobalTelco release manager",
	Long:  "Interactive TUI for managing releases, builds, deployments, and server operations across GlobalTelco components.\n\nRun without arguments to open the full-screen TUI dashboard.",
	RunE: func(cmd *cobra.Command, args []string) error {
		root, err := config.FindProjectRoot()
		if err != nil {
			return err
		}
		m := tui.NewApp(root)
		p := tea.NewProgram(m, tea.WithAltScreen())
		if _, err := p.Run(); err != nil {
			return fmt.Errorf("TUI error: %w", err)
		}
		return nil
	},
}

func Execute() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}

func init() {
	rootCmd.Version = readVersion()

	rootCmd.AddCommand(statusCmd)
	rootCmd.AddCommand(validateCmd)
	rootCmd.AddCommand(releaseCmd)
	rootCmd.AddCommand(buildCmd)
	rootCmd.AddCommand(deployCmd)
	rootCmd.AddCommand(serverCmd)
}

func readVersion() string {
	root, err := config.FindProjectRoot()
	if err != nil {
		return "dev"
	}
	comp := config.FindComponent("engine")
	if comp == nil {
		return "dev"
	}
	v, err := core.ReadComponentVersion(root, *comp)
	if err != nil {
		return "dev"
	}
	return v
}
