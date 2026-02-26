package tui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"

	"gt/config"
	"gt/core"
)

// ServerStatusMsg carries fetched server status.
type ServerStatusMsg struct {
	Status core.ServerStatus
}

// DashboardModel displays component cards and server status.
type DashboardModel struct {
	statuses     []core.ComponentStatus
	serverStatus core.ServerStatus
	cursor       int
	serverFetched bool
}

// NewDashboard creates a new dashboard model.
func NewDashboard() DashboardModel {
	return DashboardModel{}
}

func (d DashboardModel) Init() tea.Cmd {
	return nil
}

func (d DashboardModel) SetStatuses(statuses []core.ComponentStatus) DashboardModel {
	d.statuses = statuses
	return d
}

func (d DashboardModel) Update(msg tea.Msg) (DashboardModel, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		maxIdx := len(d.statuses) // components + server panel
		switch msg.String() {
		case "up", "k":
			if d.cursor > 0 {
				d.cursor--
			}
		case "down", "j":
			if d.cursor < maxIdx {
				d.cursor++
			}
		case "left", "h":
			if d.cursor >= 2 && d.cursor <= 3 {
				d.cursor -= 2
			}
		case "right", "l":
			if d.cursor <= 1 {
				d.cursor += 2
			}
		}
	}
	return d, nil
}

func (d DashboardModel) FetchServer() tea.Cmd {
	cfg := config.DefaultDeployConfig()
	return func() tea.Msg {
		status := core.FetchServerInfo(cfg)
		return ServerStatusMsg{Status: status}
	}
}

func (d DashboardModel) View(width, height int) string {
	if width < 40 {
		width = 80
	}
	if height < 10 {
		height = 24
	}

	var sections []string

	// Component cards (2x2 grid)
	sections = append(sections, d.renderCards(width))

	// Server status panel
	sections = append(sections, d.renderServerPanel(width))

	content := lipgloss.JoinVertical(lipgloss.Left, sections...)
	return content
}

func (d DashboardModel) renderCards(width int) string {
	if len(d.statuses) == 0 {
		return Indent(StyleDim.Render("Loading component status..."), 2)
	}

	cardWidth := (width - 8) / 2
	if cardWidth < 30 {
		cardWidth = 30
	}
	if cardWidth > 50 {
		cardWidth = 50
	}

	var cards []string
	for i, s := range d.statuses {
		cards = append(cards, d.renderComponentCard(i, s, cardWidth))
	}

	// Arrange in 2x2 grid
	var rows []string
	for i := 0; i < len(cards); i += 2 {
		if i+1 < len(cards) {
			row := lipgloss.JoinHorizontal(lipgloss.Top, cards[i], " ", cards[i+1])
			rows = append(rows, row)
		} else {
			rows = append(rows, cards[i])
		}
	}

	return Indent(lipgloss.JoinVertical(lipgloss.Left, rows...), 2)
}

func (d DashboardModel) renderComponentCard(idx int, s core.ComponentStatus, width int) string {
	selected := idx == d.cursor

	// Version line
	verStyle := lipgloss.NewStyle().Foreground(ColorBright).Bold(true)
	version := verStyle.Render("v" + s.Version)

	// Status
	var status string
	if s.IsDirty {
		status = RenderStatusDot(false) + " " + StyleWarning.Render("dirty")
	} else {
		status = RenderStatusDot(true) + " " + StyleProfit.Render("clean")
	}

	// Tag
	tag := s.LatestTag
	if tag == "" {
		tag = StyleDim.Render("(none)")
	}

	// Build content
	var lines []string
	lines = append(lines, RenderLabelValue("Version", version))
	lines = append(lines, RenderLabelValue("Status", status))
	lines = append(lines, RenderLabelValue("Tag", tag))

	if s.IsDirty {
		changes := StyleWarning.Render(fmt.Sprintf("%d files", len(s.ChangedFiles)))
		lines = append(lines, RenderLabelValue("Changes", changes))

		var suggest string
		switch s.SuggestedBump {
		case core.BumpMajor:
			suggest = StyleLoss.Render("MAJOR")
		case core.BumpMinor:
			suggest = StyleWarning.Render("minor")
		default:
			suggest = StyleNeutral.Render("patch")
		}
		lines = append(lines, RenderLabelValue("Suggest", suggest))
	}

	content := strings.Join(lines, "\n")
	return RenderCard(s.Component.Name, ComponentColor(s.Component.ID), content, width, selected)
}

func (d DashboardModel) renderServerPanel(width int) string {
	panelWidth := width - 6
	if panelWidth > 104 {
		panelWidth = 104
	}

	selected := d.cursor == len(d.statuses)

	var lines []string

	if !d.serverFetched {
		lines = append(lines, StyleDim.Render("Fetching server status..."))
	} else if !d.serverStatus.Online {
		errMsg := d.serverStatus.Error
		if errMsg == "" {
			errMsg = "unreachable"
		}
		lines = append(lines, RenderLabelValue("Status", RenderStatusDot(false)+" "+StyleLoss.Render("Offline")+" "+StyleDim.Render("("+errMsg+")")))
	} else {
		lines = append(lines, RenderLabelValue("Status", RenderStatusDot(true)+" "+StyleProfit.Render("Online")))

		if d.serverStatus.Version != "" {
			lines = append(lines, RenderLabelValue("Version", StyleBright.Render("v"+d.serverStatus.Version)))
		}
		if d.serverStatus.UptimeSecs > 0 {
			lines = append(lines, RenderLabelValue("Uptime", StyleBright.Render(d.serverStatus.UptimeString())))
		}

		playersStyle := StyleBright
		if d.serverStatus.ConnectedPlayers > 0 {
			playersStyle = StyleProfit
		}
		lines = append(lines, RenderLabelValue("Players", playersStyle.Render(fmt.Sprintf("%d online", d.serverStatus.ConnectedPlayers))))
		lines = append(lines, RenderLabelValue("Worlds", StyleBright.Render(fmt.Sprintf("%d active", d.serverStatus.ActiveWorlds))))
	}

	cfg := config.DefaultDeployConfig()
	lines = append(lines, RenderLabelValue("Host", StyleDim.Render(cfg.Host)))
	lines = append(lines, RenderLabelValue("Domain", StyleDim.Render(cfg.Domain)))

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Game Server", ColorServer, content, panelWidth, selected), 2)
}
