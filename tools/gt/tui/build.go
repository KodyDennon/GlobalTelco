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
		if b.building {
			return b, nil
		}
		if b.done {
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
			// Toggle selection — delete key when off to keep map clean
			if b.selected[b.cursor] {
				delete(b.selected, b.cursor)
			} else {
				b.selected[b.cursor] = true
			}
		case "enter":
			if len(b.selected) == 0 {
				// Build currently highlighted
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
	var sb strings.Builder

	sb.WriteString(StyleTitle.Render("  Build"))
	sb.WriteString("\n\n")

	if b.building {
		sb.WriteString("  " + b.spinner.View() + " Building...\n\n")
		b.renderOutput(&sb, height-6)
	} else if b.done {
		if b.err != nil {
			sb.WriteString(StyleError.Render(fmt.Sprintf("  Build failed: %v", b.err)))
		} else {
			sb.WriteString(StyleSuccess.Render("  Build complete!"))
		}
		sb.WriteString("\n\n")
		b.renderOutput(&sb, height-8)
		sb.WriteString("\n")
		sb.WriteString(StyleDim.Render("  Press esc to return"))
	} else {
		sb.WriteString(StyleSubtitle.Render("  Select components to build:"))
		sb.WriteString("\n\n")

		for i, s := range b.statuses {
			prefix := "  "
			if i == b.cursor {
				prefix = StyleAccent.Render("> ")
			}

			check := "[ ]"
			if b.selected[i] {
				check = StyleProfit.Render("[x]")
			}

			name := ComponentStyle(s.Component.ID, s.Component.Name)
			sb.WriteString(fmt.Sprintf("%s%s %s\n", prefix, check, name))
		}

		sb.WriteString("\n")
		sb.WriteString("  " + StyleKey.Render("space") + " " + StyleDim.Render("toggle") +
			"  " + StyleKey.Render("enter") + " " + StyleDim.Render("build") +
			"  " + StyleKey.Render("esc") + " " + StyleDim.Render("back"))
	}

	return sb.String()
}

func (b BuildModel) renderOutput(sb *strings.Builder, maxLines int) {
	start := 0
	if len(b.output) > maxLines && maxLines > 0 {
		start = len(b.output) - maxLines
	}
	for _, line := range b.output[start:] {
		style := StyleDim
		if strings.HasPrefix(line, ">>>") {
			style = StyleAccent
		} else if strings.Contains(line, "FAILED") || strings.Contains(line, "error") {
			style = StyleLoss
		} else if strings.Contains(line, "complete") {
			style = StyleProfit
		}
		sb.WriteString("  " + style.Render(line) + "\n")
	}
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
