package tui

import "github.com/charmbracelet/lipgloss"

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

	// Styles
	StyleTitle = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorBright).
			MarginBottom(1)

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

	StylePanel = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorBorder).
			Padding(1, 2)

	StyleHeader = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorBright).
			Border(lipgloss.NormalBorder(), false, false, true, false).
			BorderForeground(ColorBorder).
			MarginBottom(1).
			PaddingBottom(0)

	StyleKey = lipgloss.NewStyle().
			Foreground(ColorAccent).
			Bold(true)

	StyleHelp = lipgloss.NewStyle().
			Foreground(ColorDim)

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

	StyleStatusBar = lipgloss.NewStyle().
			Foreground(ColorDim).
			Border(lipgloss.NormalBorder(), true, false, false, false).
			BorderForeground(ColorBorder).
			PaddingTop(0)
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
