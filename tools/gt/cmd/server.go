package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"

	"gt/config"
	"gt/core"
)

var serverCmd = &cobra.Command{
	Use:   "server",
	Short: "Game server management",
	Long:  "Check status, view logs, and restart the game server.",
}

var serverStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show server status",
	RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.DefaultDeployConfig()
		fmt.Printf("Checking %s...\n", cfg.Domain)

		adminKey := os.Getenv("GT_ADMIN_KEY")
		var status core.ServerStatus
		if adminKey != "" {
			status = core.FetchServerHealth(cfg, adminKey)
		} else {
			status = core.FetchServerInfo(cfg)
		}

		if !status.Online {
			errMsg := status.Error
			if errMsg == "" {
				errMsg = "unreachable"
			}
			fmt.Printf("  Status:   OFFLINE (%s)\n", errMsg)
			fmt.Printf("  Host:     %s\n", cfg.Host)
			fmt.Printf("  Domain:   %s\n", cfg.Domain)
			return nil
		}

		fmt.Printf("  Status:   ONLINE\n")
		if status.Version != "" {
			fmt.Printf("  Version:  v%s\n", status.Version)
		}
		if status.UptimeSecs > 0 {
			fmt.Printf("  Uptime:   %s\n", status.UptimeString())
		}
		fmt.Printf("  Players:  %d online\n", status.ConnectedPlayers)
		fmt.Printf("  Worlds:   %d active\n", status.ActiveWorlds)
		if status.RegisteredAccts > 0 {
			fmt.Printf("  Accounts: %d registered\n", status.RegisteredAccts)
		}
		if status.HasDatabase {
			fmt.Printf("  Database: connected\n")
		}
		fmt.Printf("  Host:     %s\n", cfg.Host)
		fmt.Printf("  Domain:   %s\n", cfg.Domain)
		return nil
	},
}

var serverLogsLines int
var serverLogsDownload bool

var serverLogsCmd = &cobra.Command{
	Use:   "logs",
	Short: "View or download server logs via SSH",
	RunE: func(cmd *cobra.Command, args []string) error {
		root, err := config.FindProjectRoot()
		if err != nil {
			return err
		}
		cfg := config.DefaultDeployConfig()

		if serverLogsDownload {
			fmt.Fprintf(os.Stderr, "Downloading %d lines to logs/...\n", serverLogsLines)
			path, err := core.SaveServerLogs(root, cfg, serverLogsLines)
			if err != nil {
				return fmt.Errorf("failed to download logs: %w", err)
			}
			fmt.Printf("Logs saved to: %s\n", path)
			return nil
		}

		fmt.Fprintf(os.Stderr, "Fetching %d lines from %s...\n\n", serverLogsLines, cfg.Host)
		logs, err := core.ServerLogs(cfg, serverLogsLines)
		if err != nil {
			return fmt.Errorf("failed to fetch logs: %w", err)
		}

		fmt.Println(logs)
		return nil
	},
}

var serverRestartCmd = &cobra.Command{
	Use:   "restart",
	Short: "Restart the game server",
	RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.DefaultDeployConfig()

		fmt.Printf("Restart %s on %s? [y/N] ", cfg.ServiceName, cfg.Host)
		var answer string
		fmt.Scanln(&answer)
		if strings.ToLower(answer) != "y" {
			fmt.Println("Cancelled.")
			return nil
		}

		fmt.Printf("Restarting %s...\n", cfg.ServiceName)
		if err := core.ServerRestart(cfg); err != nil {
			return fmt.Errorf("restart failed: %w", err)
		}

		fmt.Println("Server restarted successfully.")

		fmt.Println("Checking status...")
		adminKey := os.Getenv("GT_ADMIN_KEY")
		var status core.ServerStatus
		if adminKey != "" {
			status = core.FetchServerHealth(cfg, adminKey)
		} else {
			status = core.FetchServerInfo(cfg)
		}
		if status.Online {
			fmt.Printf("  Status: ONLINE (v%s)\n", status.Version)
		} else {
			errMsg := status.Error
			if errMsg == "" {
				errMsg = "starting up"
			}
			fmt.Printf("  Status: OFFLINE (%s)\n", errMsg)
		}

		return nil
	},
}

func init() {
	serverLogsCmd.Flags().IntVarP(&serverLogsLines, "lines", "n", 50, "Number of log lines to fetch")
	serverLogsCmd.Flags().BoolVarP(&serverLogsDownload, "download", "d", false, "Download logs to a local file in logs/ dir")

	serverCmd.AddCommand(serverStatusCmd)
	serverCmd.AddCommand(serverLogsCmd)
	serverCmd.AddCommand(serverRestartCmd)
}
