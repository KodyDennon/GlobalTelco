package core

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os/exec"
	"strings"
	"time"

	"gt/config"
)

// ServerStatus represents live game server state.
type ServerStatus struct {
	Online           bool
	Version          string
	UptimeSecs       float64
	ActiveWorlds     int
	ConnectedPlayers int
	RegisteredAccts  int
	HasDatabase      bool
	Error            string
}

// UptimeString formats uptime as human-readable.
func (s ServerStatus) UptimeString() string {
	if s.UptimeSecs <= 0 {
		return "-"
	}
	d := time.Duration(s.UptimeSecs) * time.Second
	days := int(d.Hours()) / 24
	hours := int(d.Hours()) % 24
	mins := int(d.Minutes()) % 60
	if days > 0 {
		return fmt.Sprintf("%dd %dh %dm", days, hours, mins)
	}
	if hours > 0 {
		return fmt.Sprintf("%dh %dm", hours, mins)
	}
	return fmt.Sprintf("%dm", mins)
}

// FetchServerInfo fetches public server info from /api/info.
func FetchServerInfo(cfg config.DeployConfig) ServerStatus {
	client := &http.Client{Timeout: 5 * time.Second}

	url := fmt.Sprintf("https://%s/api/info", cfg.Domain)
	resp, err := client.Get(url)
	if err != nil {
		// Try HTTP fallback
		url = fmt.Sprintf("http://%s/api/info", cfg.Domain)
		resp, err = client.Get(url)
		if err != nil {
			return ServerStatus{Online: false, Error: "unreachable"}
		}
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return ServerStatus{Online: false, Error: fmt.Sprintf("HTTP %d", resp.StatusCode)}
	}

	var data struct {
		Version          string  `json:"version"`
		ActiveWorlds     int     `json:"active_worlds"`
		ConnectedPlayers int     `json:"connected_players"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&data); err != nil {
		return ServerStatus{Online: false, Error: "bad response"}
	}

	return ServerStatus{
		Online:           true,
		Version:          data.Version,
		ActiveWorlds:     data.ActiveWorlds,
		ConnectedPlayers: data.ConnectedPlayers,
	}
}

// FetchServerHealth fetches admin health data (requires admin key env var).
func FetchServerHealth(cfg config.DeployConfig, adminKey string) ServerStatus {
	if adminKey == "" {
		return FetchServerInfo(cfg)
	}

	client := &http.Client{Timeout: 5 * time.Second}
	url := fmt.Sprintf("https://%s/api/admin/health", cfg.Domain)

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return ServerStatus{Online: false, Error: err.Error()}
	}
	req.Header.Set("X-Admin-Key", adminKey)

	resp, err := client.Do(req)
	if err != nil {
		return ServerStatus{Online: false, Error: "unreachable"}
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return FetchServerInfo(cfg)
	}

	var data struct {
		Version          string  `json:"version"`
		UptimeSecs       float64 `json:"uptime_secs"`
		ActiveWorlds     int     `json:"active_worlds"`
		ConnectedPlayers int     `json:"connected_players"`
		RegisteredAccts  int     `json:"registered_accounts"`
		HasDatabase      bool    `json:"has_database"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&data); err != nil {
		return ServerStatus{Online: false, Error: "bad response"}
	}

	return ServerStatus{
		Online:           true,
		Version:          data.Version,
		UptimeSecs:       data.UptimeSecs,
		ActiveWorlds:     data.ActiveWorlds,
		ConnectedPlayers: data.ConnectedPlayers,
		RegisteredAccts:  data.RegisteredAccts,
		HasDatabase:      data.HasDatabase,
	}
}

// ServerLogs fetches recent logs via SSH.
func ServerLogs(cfg config.DeployConfig, lines int) (string, error) {
	remote := fmt.Sprintf("%s@%s", cfg.SSHUser, cfg.Host)
	logCmd := fmt.Sprintf("sudo journalctl -u %s --no-pager -n %d", cfg.ServiceName, lines)

	cmd := exec.Command("ssh",
		"-i", cfg.SSHKey,
		"-o", "StrictHostKeyChecking=no",
		"-o", "ConnectTimeout=10",
		remote,
		logCmd,
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("SSH failed: %w", err)
	}
	return strings.TrimSpace(string(out)), nil
}

// ServerRestart restarts the game server via SSH.
func ServerRestart(cfg config.DeployConfig) error {
	remote := fmt.Sprintf("%s@%s", cfg.SSHUser, cfg.Host)
	cmd := exec.Command("ssh",
		"-i", cfg.SSHKey,
		"-o", "StrictHostKeyChecking=no",
		"-o", "ConnectTimeout=10",
		remote,
		fmt.Sprintf("sudo systemctl restart %s", cfg.ServiceName),
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("restart failed: %s: %w", strings.TrimSpace(string(out)), err)
	}
	return nil
}
