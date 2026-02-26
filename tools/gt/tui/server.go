package tui

import (
	"fmt"
	"os"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/lipgloss"

	"gt/config"
	"gt/core"
)

// ServerLogMsg carries fetched log output.
type ServerLogMsg struct {
	Logs string
	Err  error
}

// ServerRestartMsg indicates restart completed.
type ServerRestartMsg struct {
	Err error
}

// ServerTab represents active tab in server view.
type ServerTab int

const (
	ServerTabStatus ServerTab = iota
	ServerTabLogs
)

// ServerModel handles the server management view.
type ServerModel struct {
	config         config.DeployConfig
	status         core.ServerStatus
	statusFetched  bool
	tab            ServerTab
	logs           string
	logsFetched    bool
	logsErr        error
	restarting     bool
	restartErr     error
	restartDone    bool
	confirmRestart bool
	spinner        spinner.Model
	logScroll      int
}

// NewServer creates a new server model.
func NewServer() ServerModel {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = StyleSpinner
	return ServerModel{
		config:  config.DefaultDeployConfig(),
		spinner: s,
	}
}

// Start resets the server view for fresh display.
func (m ServerModel) Start() ServerModel {
	m.config = config.DefaultDeployConfig()
	m.tab = ServerTabStatus
	m.logs = ""
	m.logsFetched = false
	m.logsErr = nil
	m.restarting = false
	m.restartErr = nil
	m.restartDone = false
	m.confirmRestart = false
	m.logScroll = 0
	// Don't reset statusFetched — App may have already fetched it
	return m
}

// Init fetches server status.
func (m ServerModel) Init() tea.Cmd {
	cfg := m.config
	return tea.Batch(
		m.spinner.Tick,
		func() tea.Msg {
			adminKey := os.Getenv("GT_ADMIN_KEY")
			var status core.ServerStatus
			if adminKey != "" {
				status = core.FetchServerHealth(cfg, adminKey)
			} else {
				status = core.FetchServerInfo(cfg)
			}
			return ServerStatusMsg{Status: status}
		},
	)
}

func (m ServerModel) Update(msg tea.Msg) (ServerModel, tea.Cmd) {
	switch msg := msg.(type) {
	case spinner.TickMsg:
		if !m.statusFetched || m.restarting || (m.tab == ServerTabLogs && !m.logsFetched) {
			var cmd tea.Cmd
			m.spinner, cmd = m.spinner.Update(msg)
			return m, cmd
		}

	case ServerLogMsg:
		m.logs = msg.Logs
		m.logsFetched = true
		m.logsErr = msg.Err
		m.logScroll = 0
		return m, nil

	case ServerRestartMsg:
		m.restarting = false
		m.restartDone = true
		m.restartErr = msg.Err
		// Refresh status after restart
		cfg := m.config
		return m, func() tea.Msg {
			adminKey := os.Getenv("GT_ADMIN_KEY")
			if adminKey != "" {
				return ServerStatusMsg{Status: core.FetchServerHealth(cfg, adminKey)}
			}
			return ServerStatusMsg{Status: core.FetchServerInfo(cfg)}
		}

	case tea.KeyMsg:
		return m.handleKey(msg)
	}

	return m, nil
}

func (m ServerModel) handleKey(msg tea.KeyMsg) (ServerModel, tea.Cmd) {
	if m.confirmRestart {
		switch msg.String() {
		case "y":
			m.confirmRestart = false
			m.restarting = true
			m.restartDone = false
			m.restartErr = nil
			cfg := m.config
			return m, tea.Batch(
				m.spinner.Tick,
				func() tea.Msg {
					err := core.ServerRestart(cfg)
					return ServerRestartMsg{Err: err}
				},
			)
		case "n", "esc":
			m.confirmRestart = false
		}
		return m, nil
	}

	switch msg.String() {
	case "tab":
		if m.tab == ServerTabStatus {
			m.tab = ServerTabLogs
			if !m.logsFetched {
				return m, m.fetchLogs()
			}
		} else {
			m.tab = ServerTabStatus
		}

	case "l":
		m.tab = ServerTabLogs
		m.logsFetched = false
		return m, m.fetchLogs()

	case "r":
		if !m.restarting {
			m.confirmRestart = true
		}

	case "R":
		m.statusFetched = false
		cfg := m.config
		return m, tea.Batch(
			m.spinner.Tick,
			func() tea.Msg {
				adminKey := os.Getenv("GT_ADMIN_KEY")
				if adminKey != "" {
					return ServerStatusMsg{Status: core.FetchServerHealth(cfg, adminKey)}
				}
				return ServerStatusMsg{Status: core.FetchServerInfo(cfg)}
			},
		)

	case "j", "down":
		if m.tab == ServerTabLogs && m.logs != "" {
			maxScroll := len(strings.Split(m.logs, "\n")) - 5
			if maxScroll < 0 {
				maxScroll = 0
			}
			if m.logScroll < maxScroll {
				m.logScroll++
			}
		}
	case "k", "up":
		if m.tab == ServerTabLogs && m.logScroll > 0 {
			m.logScroll--
		}
	}

	return m, nil
}

func (m ServerModel) fetchLogs() tea.Cmd {
	cfg := m.config
	return tea.Batch(
		m.spinner.Tick,
		func() tea.Msg {
			logs, err := core.ServerLogs(cfg, 100)
			return ServerLogMsg{Logs: logs, Err: err}
		},
	)
}

func (m ServerModel) View(width, height int) string {
	if width < 40 {
		width = 80
	}

	var sections []string

	// Tab bar
	sections = append(sections, Indent(m.renderTabs(), 2))
	sections = append(sections, "")

	switch m.tab {
	case ServerTabStatus:
		sections = append(sections, m.renderStatus(width))

		// Actions area
		sections = append(sections, "")
		if m.confirmRestart {
			sections = append(sections, Indent(
				StyleWarning.Render("Restart server? ")+
					StyleKey.Render("y")+StyleDim.Render("es / ")+StyleKey.Render("n")+StyleDim.Render("o"),
				2,
			))
		} else if m.restarting {
			sections = append(sections, Indent(m.spinner.View()+" Restarting server...", 2))
		} else if m.restartDone {
			if m.restartErr != nil {
				sections = append(sections, Indent(StyleError.Render("Restart failed: "+m.restartErr.Error()), 2))
			} else {
				sections = append(sections, Indent(StyleSuccess.Render("Server restarted successfully"), 2))
			}
		}

	case ServerTabLogs:
		sections = append(sections, m.renderLogs(width, height-6))
	}

	return lipgloss.JoinVertical(lipgloss.Left, sections...)
}

func (m ServerModel) renderTabs() string {
	statusTab := " Status "
	logsTab := " Logs "

	if m.tab == ServerTabStatus {
		statusTab = StyleSelected.Render("[Status]")
		logsTab = StyleUnselected.Render(" Logs ")
	} else {
		statusTab = StyleUnselected.Render(" Status ")
		logsTab = StyleSelected.Render("[Logs]")
	}

	return statusTab + "  " + logsTab
}

func (m ServerModel) renderStatus(width int) string {
	panelWidth := width - 6
	if panelWidth > 104 {
		panelWidth = 104
	}

	var lines []string

	if !m.statusFetched {
		lines = append(lines, m.spinner.View()+" "+StyleDim.Render("Fetching server status..."))
	} else if !m.status.Online {
		errMsg := m.status.Error
		if errMsg == "" {
			errMsg = "unreachable"
		}
		lines = append(lines, RenderLabelValue("Status", RenderStatusDot(false)+" "+StyleLoss.Render("Offline")+" "+StyleDim.Render("("+errMsg+")")))
	} else {
		lines = append(lines, RenderLabelValue("Status", RenderStatusDot(true)+" "+StyleProfit.Render("Online")))

		if m.status.Version != "" {
			lines = append(lines, RenderLabelValue("Version", StyleBright.Render("v"+m.status.Version)))
		}
		if m.status.UptimeSecs > 0 {
			lines = append(lines, RenderLabelValue("Uptime", StyleBright.Render(m.status.UptimeString())))
		}

		playersStyle := StyleBright
		if m.status.ConnectedPlayers > 0 {
			playersStyle = StyleProfit
		}
		lines = append(lines, RenderLabelValue("Players", playersStyle.Render(fmt.Sprintf("%d online", m.status.ConnectedPlayers))))
		lines = append(lines, RenderLabelValue("Worlds", StyleBright.Render(fmt.Sprintf("%d active", m.status.ActiveWorlds))))

		if m.status.RegisteredAccts > 0 {
			lines = append(lines, RenderLabelValue("Accounts", StyleBright.Render(fmt.Sprintf("%d registered", m.status.RegisteredAccts))))
		}
		if m.status.HasDatabase {
			lines = append(lines, RenderLabelValue("Database", RenderStatusDot(true)+" "+StyleProfit.Render("Connected")))
		}
	}

	lines = append(lines, "")
	lines = append(lines, RenderLabelValue("Host", StyleDim.Render(m.config.Host)))
	lines = append(lines, RenderLabelValue("Domain", StyleDim.Render(m.config.Domain)))
	lines = append(lines, RenderLabelValue("Service", StyleDim.Render(m.config.ServiceName)))

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Game Server", ColorServer, content, panelWidth, false), 2)
}

func (m ServerModel) renderLogs(width, maxHeight int) string {
	panelWidth := width - 6
	if panelWidth > 120 {
		panelWidth = 120
	}

	if !m.logsFetched {
		content := m.spinner.View() + " " + StyleDim.Render("Fetching logs...")
		return Indent(RenderCard("Server Logs", ColorServer, content, panelWidth, false), 2)
	}

	if m.logsErr != nil {
		content := StyleLoss.Render("Error: " + m.logsErr.Error())
		return Indent(RenderCard("Server Logs", ColorServer, content, panelWidth, false), 2)
	}

	if m.logs == "" {
		content := StyleDim.Render("No logs available")
		return Indent(RenderCard("Server Logs", ColorServer, content, panelWidth, false), 2)
	}

	logLines := strings.Split(m.logs, "\n")

	// Scroll offset
	start := m.logScroll
	if start >= len(logLines) {
		start = len(logLines) - 1
	}
	if start < 0 {
		start = 0
	}

	visibleLines := maxHeight - 6
	if visibleLines < 5 {
		visibleLines = 5
	}

	end := start + visibleLines
	if end > len(logLines) {
		end = len(logLines)
	}

	var styledLines []string
	for _, line := range logLines[start:end] {
		style := StyleDim
		if strings.Contains(line, "ERROR") || strings.Contains(line, "error") || strings.Contains(line, "panic") {
			style = StyleLoss
		} else if strings.Contains(line, "WARN") || strings.Contains(line, "warn") {
			style = StyleWarning
		} else if strings.Contains(line, "INFO") {
			style = lipgloss.NewStyle().Foreground(ColorFg)
		}
		// Truncate long lines
		maxLen := panelWidth - 6
		if maxLen > 0 && len(line) > maxLen {
			line = line[:maxLen] + "..."
		}
		styledLines = append(styledLines, style.Render(line))
	}

	scrollInfo := StyleDim.Render(fmt.Sprintf("Lines %d-%d of %d  (j/k to scroll)", start+1, end, len(logLines)))
	content := strings.Join(styledLines, "\n") + "\n\n" + scrollInfo

	return Indent(RenderCard("Server Logs", ColorServer, content, panelWidth, false), 2)
}
