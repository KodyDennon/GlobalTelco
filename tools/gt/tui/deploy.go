package tui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/bubbles/spinner"

	"gt/config"
	"gt/core"
)

// DeployDoneMsg indicates deployment finished.
type DeployDoneMsg struct {
	Output []string
	Err    error
}

// DeployModel handles the deploy view.
type DeployModel struct {
	root      string
	config    config.DeployConfig
	deploying bool
	done      bool
	err       error
	output    []string
	skipBuild bool
	cursor    int
	spinner   spinner.Model
}

// NewDeploy creates a new deploy model.
func NewDeploy() DeployModel {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = StyleSpinner
	return DeployModel{spinner: s}
}

func (d DeployModel) Start(root string) DeployModel {
	d.root = root
	d.config = config.DefaultDeployConfig()
	d.deploying = false
	d.done = false
	d.err = nil
	d.output = nil
	d.skipBuild = false
	d.cursor = 0
	return d
}

func (d DeployModel) Update(msg tea.Msg) (DeployModel, tea.Cmd) {
	switch msg := msg.(type) {
	case spinner.TickMsg:
		if d.deploying {
			var cmd tea.Cmd
			d.spinner, cmd = d.spinner.Update(msg)
			return d, cmd
		}

	case DeployDoneMsg:
		d.deploying = false
		d.done = true
		d.err = msg.Err
		d.output = msg.Output
		return d, nil

	case tea.KeyMsg:
		if d.deploying {
			return d, nil
		}
		if d.done {
			return d, nil
		}

		switch msg.String() {
		case "up", "k":
			if d.cursor > 0 {
				d.cursor--
			}
		case "down", "j":
			if d.cursor < 1 {
				d.cursor++
			}
		case " ":
			if d.cursor == 0 {
				d.skipBuild = !d.skipBuild
			}
		case "enter":
			d.deploying = true
			d.output = nil
			return d, tea.Batch(d.spinner.Tick, d.executeDeploy())
		}
	}

	return d, nil
}

func (d DeployModel) View(width, height int) string {
	var sb strings.Builder

	// Show deploy target info
	sb.WriteString(fmt.Sprintf("  Host:    %s\n", StyleBright.Render(d.config.Host)))
	sb.WriteString(fmt.Sprintf("  Domain:  %s\n", StyleBright.Render(d.config.Domain)))
	sb.WriteString(fmt.Sprintf("  Service: %s\n", StyleBright.Render(d.config.ServiceName)))
	sb.WriteString("\n")

	if d.deploying {
		sb.WriteString("  " + d.spinner.View() + " Deploying...\n\n")
		d.renderOutput(&sb, height-12)
	} else if d.done {
		if d.err != nil {
			sb.WriteString(StyleError.Render(fmt.Sprintf("  Deploy failed: %v", d.err)))
		} else {
			sb.WriteString(StyleSuccess.Render("  Deploy complete!"))
		}
		sb.WriteString("\n\n")
		d.renderOutput(&sb, height-14)
		sb.WriteString("\n")
		sb.WriteString(StyleDim.Render("  Press esc to return"))
	} else {
		// Options
		skipCheck := "[ ]"
		if d.skipBuild {
			skipCheck = StyleProfit.Render("[x]")
		}
		prefix0 := "  "
		if d.cursor == 0 {
			prefix0 = StyleAccent.Render("> ")
		}
		sb.WriteString(fmt.Sprintf("%s%s Skip build (upload existing binary)\n", prefix0, skipCheck))

		prefix1 := "  "
		if d.cursor == 1 {
			prefix1 = StyleAccent.Render("> ")
		}
		sb.WriteString(fmt.Sprintf("\n%s%s\n", prefix1, StyleAccent.Render("Deploy now")))

	}

	return sb.String()
}

func (d DeployModel) renderOutput(sb *strings.Builder, maxLines int) {
	start := 0
	if len(d.output) > maxLines && maxLines > 0 {
		start = len(d.output) - maxLines
	}
	for _, line := range d.output[start:] {
		style := StyleDim
		if strings.Contains(line, "ERROR") || strings.Contains(line, "failed") {
			style = StyleLoss
		} else if strings.Contains(line, "complete") || strings.Contains(line, "Health check") {
			style = StyleProfit
		}
		sb.WriteString("  " + style.Render(line) + "\n")
	}
}

func (d DeployModel) executeDeploy() tea.Cmd {
	root := d.root
	cfg := d.config
	skipBuild := d.skipBuild

	return func() tea.Msg {
		var output []string
		err := core.ExecuteDeploy(core.DeployOpts{
			Root:      root,
			Config:    cfg,
			SkipBuild: skipBuild,
			OnStep: func(step core.DeployStep, msg string) {
				output = append(output, fmt.Sprintf("[%s] %s", step, msg))
			},
			OnOutput: func(line string) {
				if line != "" {
					output = append(output, line)
				}
			},
		})
		return DeployDoneMsg{Output: output, Err: err}
	}
}
