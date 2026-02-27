package tui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/bubbles/spinner"

	"gt/core"
)

// BuildDoneMsg indicates the build completed.
type BuildDoneMsg struct {
	Output []string
	Err    error
}

// BuildModel handles the build view.
type BuildModel struct {
	root     string
	statuses []core.ComponentStatus
	cursor   int
	selected map[int]bool
	building bool
	done     bool
	err      error
	output   []string
	spinner  spinner.Model
}

// NewBuild creates a new build model.
func NewBuild() BuildModel {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = StyleSpinner
	return BuildModel{selected: make(map[int]bool)}
}

func (b BuildModel) Start(root string, statuses []core.ComponentStatus) BuildModel {
	b.root = root
	b.statuses = statuses
	b.cursor = 0
	b.selected = make(map[int]bool)
	b.building = false
	b.done = false
	b.err = nil
	b.output = nil
	return b
}

func (b BuildModel) Update(msg tea.Msg) (BuildModel, tea.Cmd) {
	switch msg := msg.(type) {
	case spinner.TickMsg:
		if b.building {
			var cmd tea.Cmd
			b.spinner, cmd = b.spinner.Update(msg)
			return b, cmd
		}

	case BuildDoneMsg:
		b.building = false
		b.done = true
		b.err = msg.Err
		b.output = msg.Output
		return b, nil

	case tea.KeyMsg:
		if b.building || b.done {
			return b, nil
		}

		switch msg.String() {
		case "up", "k":
			if b.cursor > 0 {
				b.cursor--
			}
		case "down", "j":
			if b.cursor < len(b.statuses)-1 {
				b.cursor++
			}
		case " ":
			if b.selected[b.cursor] {
				delete(b.selected, b.cursor)
			} else {
				b.selected[b.cursor] = true
			}
		case "a":
			// Toggle all
			if len(b.selected) == len(b.statuses) {
				b.selected = make(map[int]bool)
			} else {
				for i := range b.statuses {
					b.selected[i] = true
				}
			}
		case "enter":
			if len(b.selected) == 0 {
				b.selected[b.cursor] = true
			}
			b.building = true
			b.output = nil
			return b, tea.Batch(b.spinner.Tick, b.runBuilds())
		}
	}

	return b, nil
}

func (b BuildModel) View(width, height int) string {
	panelWidth := width - 6
	if panelWidth > 104 {
		panelWidth = 104
	}

	if b.building {
		return b.viewBuilding(panelWidth, height)
	}
	if b.done {
		return b.viewDone(panelWidth, height)
	}
	return b.viewSelection(panelWidth, height)
}

func (b BuildModel) viewSelection(panelWidth, height int) string {
	var lines []string

	for i, s := range b.statuses {
		prefix := "  "
		if i == b.cursor {
			prefix = StyleAccent.Render("> ")
		}

		check := "[ ]"
		if b.selected[i] {
			check = StyleProfit.Render("[x]")
		}

		name := ComponentStyle(s.Component.ID, fmt.Sprintf("%-8s", s.Component.Name))
		ver := StyleBright.Render("v" + s.Version)
		dirty := ""
		if s.IsDirty {
			dirty = "  " + StyleWarning.Render(fmt.Sprintf("(%d changed)", len(s.ChangedFiles)))
		}
		lines = append(lines, fmt.Sprintf("%s%s %s  %s%s", prefix, check, name, ver, dirty))

		// Show build steps for this component
		steps := core.GetBuildSteps(b.root, s.Component.ID)
		if len(steps) > 0 {
			names := make([]string, len(steps))
			for j, step := range steps {
				names[j] = step.Name
			}
			lines = append(lines, "       "+StyleDim.Render(strings.Join(names, " -> ")))
		}
	}

	// Selection summary
	count := len(b.selected)
	if count > 0 {
		lines = append(lines, "")
		var names []string
		for i, s := range b.statuses {
			if b.selected[i] {
				names = append(names, s.Component.Name)
			}
		}
		lines = append(lines, StyleDim.Render(fmt.Sprintf("Selected: %s", strings.Join(names, ", "))))
	}

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Build Components", ColorAccent, content, panelWidth, false), 2)
}

func (b BuildModel) viewBuilding(panelWidth, height int) string {
	var lines []string
	lines = append(lines, b.spinner.View()+" "+StyleDim.Render("Building..."))
	lines = append(lines, "")

	maxLines := height - 12
	if maxLines < 5 {
		maxLines = 5
	}
	lines = append(lines, styledOutput(b.output, maxLines)...)

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Building", ColorWarning, content, panelWidth, false), 2)
}

func (b BuildModel) viewDone(panelWidth, height int) string {
	var lines []string
	if b.err != nil {
		lines = append(lines, StyleError.Render("Build failed: "+b.err.Error()))
	} else {
		lines = append(lines, StyleSuccess.Render("Build complete!"))
	}
	lines = append(lines, "")

	maxLines := height - 12
	if maxLines < 5 {
		maxLines = 5
	}
	lines = append(lines, styledOutput(b.output, maxLines)...)

	titleColor := ColorProfit
	if b.err != nil {
		titleColor = ColorLoss
	}
	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Build Result", titleColor, content, panelWidth, false), 2)
}

// styledOutput returns styled and truncated output lines for display in cards.
func styledOutput(output []string, maxLines int) []string {
	start := 0
	if len(output) > maxLines && maxLines > 0 {
		start = len(output) - maxLines
	}
	var lines []string
	for _, line := range output[start:] {
		style := StyleDim
		if strings.HasPrefix(line, ">>>") {
			style = StyleAccent
		} else if strings.Contains(line, "FAILED") || strings.Contains(line, "error") {
			style = StyleLoss
		} else if strings.Contains(line, "complete") {
			style = StyleProfit
		}
		lines = append(lines, style.Render(line))
	}
	return lines
}

func (b BuildModel) runBuilds() tea.Cmd {
	root := b.root
	var componentIDs []string
	for i, s := range b.statuses {
		if b.selected[i] {
			componentIDs = append(componentIDs, s.Component.ID)
		}
	}

	return func() tea.Msg {
		var output []string
		for _, id := range componentIDs {
			err := core.ExecuteBuildSync(root, id, func(line string) {
				output = append(output, line)
			})
			if err != nil {
				return BuildDoneMsg{Output: output, Err: err}
			}
		}
		return BuildDoneMsg{Output: output, Err: nil}
	}
}
