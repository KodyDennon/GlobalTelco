package tui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"

	"gt/core"
)

// DashboardModel displays the component status table.
type DashboardModel struct {
	statuses []core.ComponentStatus
	cursor   int
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
		switch msg.String() {
		case "up", "k":
			if d.cursor > 0 {
				d.cursor--
			}
		case "down", "j":
			if d.cursor < len(d.statuses)-1 {
				d.cursor++
			}
		}
	}
	return d, nil
}

func (d DashboardModel) View(width, height int) string {
	var sb strings.Builder

	// Title
	title := StyleTitle.Render("  GlobalTelco Release Manager")
	sb.WriteString(title)
	sb.WriteString("\n\n")

	// Component table
	if len(d.statuses) == 0 {
		sb.WriteString(StyleDim.Render("  Loading component status..."))
	} else {
		sb.WriteString(d.renderTable(width))
	}

	sb.WriteString("\n\n")

	// Help bar
	sb.WriteString(d.renderHelp())

	return sb.String()
}

func (d DashboardModel) renderTable(width int) string {
	var sb strings.Builder

	// Header
	headerStyle := lipgloss.NewStyle().
		Foreground(ColorDim).
		Bold(true)

	header := fmt.Sprintf("  %-12s %-12s %-22s %-8s %-10s %s",
		"Component", "Version", "Latest Tag", "Status", "Suggest", "Changes")
	sb.WriteString(headerStyle.Render(header))
	sb.WriteString("\n")

	// Separator
	sepWidth := 82
	if width > 0 && width-4 < sepWidth {
		sepWidth = width - 4
	}
	sb.WriteString(StyleDim.Render("  " + strings.Repeat("─", sepWidth)))
	sb.WriteString("\n")

	// Rows
	for i, s := range d.statuses {
		sb.WriteString(d.renderRow(i, s))
		sb.WriteString("\n")
	}

	return sb.String()
}

func (d DashboardModel) renderRow(idx int, s core.ComponentStatus) string {
	isSelected := idx == d.cursor

	// Component name with color
	nameStyle := lipgloss.NewStyle().Foreground(ComponentColor(s.Component.ID))
	if isSelected {
		nameStyle = nameStyle.Bold(true)
	}
	name := nameStyle.Render(fmt.Sprintf("%-12s", s.Component.Name))

	// Version
	versionStyle := StyleBright
	version := versionStyle.Render(fmt.Sprintf("%-12s", s.Version))

	// Latest tag
	tag := s.LatestTag
	if tag == "" {
		tag = "(none)"
	}
	tagStyle := StyleDim
	tagStr := tagStyle.Render(fmt.Sprintf("%-22s", tag))

	// Status indicator
	var status string
	if s.IsDirty {
		status = StyleWarning.Render(fmt.Sprintf("%-8s", "dirty"))
	} else {
		status = StyleProfit.Render(fmt.Sprintf("%-8s", "clean"))
	}

	// Suggested bump
	var suggest string
	if s.IsDirty && len(s.Commits) > 0 {
		switch s.SuggestedBump {
		case core.BumpMajor:
			suggest = StyleLoss.Render(fmt.Sprintf("%-10s", "MAJOR"))
		case core.BumpMinor:
			suggest = StyleWarning.Render(fmt.Sprintf("%-10s", "minor"))
		case core.BumpPatch:
			suggest = StyleNeutral.Render(fmt.Sprintf("%-10s", "patch"))
		}
	} else {
		suggest = StyleDim.Render(fmt.Sprintf("%-10s", "-"))
	}

	// Changes count
	var changes string
	if len(s.ChangedFiles) > 0 {
		changes = StyleDim.Render(fmt.Sprintf("%d files, %d commits", len(s.ChangedFiles), len(s.Commits)))
	}

	// Row prefix
	prefix := "  "
	if isSelected {
		prefix = StyleAccent.Render("> ")
	}

	return prefix + name + " " + version + " " + tagStr + " " + status + " " + suggest + " " + changes
}

func (d DashboardModel) renderHelp() string {
	keys := []struct {
		key  string
		desc string
	}{
		{"r", "Release"},
		{"b", "Build"},
		{"d", "Deploy"},
		{"h", "History"},
		{"v", "Validate"},
		{"R", "Refresh"},
		{"q", "Quit"},
	}

	var parts []string
	for _, k := range keys {
		parts = append(parts, StyleKey.Render(k.key)+" "+StyleDim.Render(k.desc))
	}

	return "  " + strings.Join(parts, "  ")
}
