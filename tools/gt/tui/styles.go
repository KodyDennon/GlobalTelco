package tui

import (
	"strings"

	"github.com/charmbracelet/lipgloss"
)

// Bloomberg Terminal dark theme
var (
	// Base colors
	ColorBg        = lipgloss.Color("#0a0e14")
	ColorFg        = lipgloss.Color("#b3b1ad")
	ColorBright    = lipgloss.Color("#e6e1cf")
	ColorDim       = lipgloss.Color("#5c6773")
	ColorBorder    = lipgloss.Color("#2d3640")
	ColorAccent    = lipgloss.Color("#39bae6")
	ColorProfit    = lipgloss.Color("#7fd962")
	ColorLoss      = lipgloss.Color("#f07178")
	ColorWarning   = lipgloss.Color("#ffb454")
	ColorNeutral   = lipgloss.Color("#59c2ff")
	ColorHighlight = lipgloss.Color("#1a1f29")

	// Component colors
	ColorEngine  = lipgloss.Color("#00BFFF")
	ColorServer  = lipgloss.Color("#00FF7F")
	ColorWeb     = lipgloss.Color("#FFD700")
	ColorDesktop = lipgloss.Color("#FF69B4")

	// Text styles
	StyleTitle = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorBright)

	StyleSubtitle = lipgloss.NewStyle().
			Foreground(ColorAccent).
			Bold(true)

	StyleDim = lipgloss.NewStyle().
			Foreground(ColorDim)

	StyleBright = lipgloss.NewStyle().
			Foreground(ColorBright)

	StyleProfit = lipgloss.NewStyle().
			Foreground(ColorProfit)

	StyleLoss = lipgloss.NewStyle().
			Foreground(ColorLoss)

	StyleWarning = lipgloss.NewStyle().
			Foreground(ColorWarning)

	StyleNeutral = lipgloss.NewStyle().
			Foreground(ColorNeutral)

	StyleAccent = lipgloss.NewStyle().
			Foreground(ColorAccent)

	StyleKey = lipgloss.NewStyle().
			Foreground(ColorAccent).
			Bold(true)

	StyleSuccess = lipgloss.NewStyle().
			Foreground(ColorProfit).
			Bold(true)

	StyleError = lipgloss.NewStyle().
			Foreground(ColorLoss).
			Bold(true)

	StyleSpinner = lipgloss.NewStyle().
			Foreground(ColorAccent)

	StyleSelected = lipgloss.NewStyle().
			Foreground(ColorBright).
			Bold(true)

	StyleUnselected = lipgloss.NewStyle().
			Foreground(ColorDim)

	// Panel/card styles
	StyleCard = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorBorder).
			Padding(1, 2)

	StyleCardSelected = lipgloss.NewStyle().
				Border(lipgloss.RoundedBorder()).
				BorderForeground(ColorAccent).
				Padding(1, 2)

	StyleHeader = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorBright).
			Padding(0, 2)

	StyleStatusBar = lipgloss.NewStyle().
			Foreground(ColorDim).
			Padding(0, 2)

	StyleLabel = lipgloss.NewStyle().
			Foreground(ColorDim).
			Width(12)

	StyleValue = lipgloss.NewStyle().
			Foreground(ColorBright)

	StyleViewport = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorBorder).
			Padding(0, 1)
)

// ComponentColor returns the lipgloss color for a component ID.
func ComponentColor(id string) lipgloss.Color {
	switch id {
	case "engine":
		return ColorEngine
	case "server":
		return ColorServer
	case "web":
		return ColorWeb
	case "desktop":
		return ColorDesktop
	default:
		return ColorFg
	}
}

// ComponentStyle returns a styled component name.
func ComponentStyle(id, name string) string {
	return lipgloss.NewStyle().Foreground(ComponentColor(id)).Bold(true).Render(name)
}

// RenderCard renders a bordered card with a colored title.
func RenderCard(title string, titleColor lipgloss.Color, content string, width int, selected bool) string {
	style := StyleCard.Width(width)
	if selected {
		style = StyleCardSelected.Width(width)
	}

	titleStyle := lipgloss.NewStyle().Foreground(titleColor).Bold(true)
	header := titleStyle.Render(title)

	return style.Render(header + "\n" + content)
}

// RenderLabelValue renders a label: value pair.
func RenderLabelValue(label, value string) string {
	return StyleLabel.Render(label) + StyleValue.Render(value)
}

// RenderStatusDot renders a colored status indicator.
func RenderStatusDot(ok bool) string {
	if ok {
		return StyleProfit.Render("●")
	}
	return StyleLoss.Render("●")
}

// RenderHelpBar renders the bottom help bar with key bindings.
func RenderHelpBar(keys []KeyBind, width int) string {
	var parts []string
	for _, k := range keys {
		parts = append(parts, StyleKey.Render(k.Key)+" "+StyleDim.Render(k.Desc))
	}
	content := strings.Join(parts, "   ")
	bar := lipgloss.NewStyle().
		Width(width).
		Foreground(ColorDim).
		Border(lipgloss.NormalBorder(), true, false, false, false).
		BorderForeground(ColorBorder).
		Padding(0, 1).
		Render(content)
	return bar
}

// RenderHeaderBar renders the top header bar.
func RenderHeaderBar(left, right string, width int) string {
	rightLen := lipgloss.Width(right)
	leftLen := lipgloss.Width(left)
	gap := width - leftLen - rightLen - 4
	if gap < 1 {
		gap = 1
	}
	content := left + strings.Repeat(" ", gap) + right

	bar := lipgloss.NewStyle().
		Width(width).
		Bold(true).
		Foreground(ColorBright).
		Border(lipgloss.NormalBorder(), false, false, true, false).
		BorderForeground(ColorBorder).
		Padding(0, 1).
		Render(content)
	return bar
}

// KeyBind represents a help bar key binding.
type KeyBind struct {
	Key  string
	Desc string
}

// Indent adds left padding to each line.
func Indent(s string, n int) string {
	pad := strings.Repeat(" ", n)
	lines := strings.Split(s, "\n")
	for i, l := range lines {
		lines[i] = pad + l
	}
	return strings.Join(lines, "\n")
}
