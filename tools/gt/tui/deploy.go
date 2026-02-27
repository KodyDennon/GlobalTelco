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
		if d.deploying || d.done {
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
	panelWidth := width - 6
	if panelWidth > 104 {
		panelWidth = 104
	}

	var sections []string

	// Target info card (always shown)
	sections = append(sections, d.viewTarget(panelWidth))

	if d.deploying {
		sections = append(sections, d.viewDeploying(panelWidth, height-10))
	} else if d.done {
		sections = append(sections, d.viewDone(panelWidth, height-10))
	} else {
		sections = append(sections, d.viewOptions(panelWidth))
	}

	return strings.Join(sections, "\n")
}

func (d DeployModel) viewTarget(panelWidth int) string {
	var lines []string
	lines = append(lines, RenderLabelValue("Host", StyleBright.Render(d.config.Host)))
	lines = append(lines, RenderLabelValue("Domain", StyleBright.Render(d.config.Domain)))
	lines = append(lines, RenderLabelValue("Service", StyleBright.Render(d.config.ServiceName)))
	lines = append(lines, RenderLabelValue("Binary", StyleDim.Render(d.config.BinaryPath)))

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Deploy Target", ColorServer, content, panelWidth, false), 2)
}

func (d DeployModel) viewOptions(panelWidth int) string {
	var lines []string

	// Skip build toggle
	skipCheck := "[ ]"
	if d.skipBuild {
		skipCheck = StyleProfit.Render("[x]")
	}
	prefix0 := "  "
	if d.cursor == 0 {
		prefix0 = StyleAccent.Render("> ")
	}
	lines = append(lines, fmt.Sprintf("%s%s Skip build (upload existing binary)", prefix0, skipCheck))

	// Deploy button
	prefix1 := "  "
	if d.cursor == 1 {
		prefix1 = StyleAccent.Render("> ")
	}
	lines = append(lines, "")
	lines = append(lines, fmt.Sprintf("%s%s", prefix1, StyleAccent.Render("Deploy now")))

	// Pipeline info
	lines = append(lines, "")
	if d.skipBuild {
		lines = append(lines, StyleDim.Render("Pipeline: Upload -> Install -> Health check"))
	} else {
		lines = append(lines, StyleDim.Render("Pipeline: Cross-compile -> Upload -> Install -> Health check"))
	}

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Deploy Options", ColorAccent, content, panelWidth, false), 2)
}

func (d DeployModel) viewDeploying(panelWidth, maxHeight int) string {
	var lines []string
	lines = append(lines, d.spinner.View()+" "+StyleDim.Render("Deploying to "+d.config.Host+"..."))
	lines = append(lines, "")

	maxLines := maxHeight - 8
	if maxLines < 5 {
		maxLines = 5
	}
	lines = append(lines, styledDeployOutput(d.output, maxLines)...)

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Deploying", ColorWarning, content, panelWidth, false), 2)
}

func (d DeployModel) viewDone(panelWidth, maxHeight int) string {
	var lines []string
	if d.err != nil {
		lines = append(lines, StyleError.Render("Deploy failed: "+d.err.Error()))
	} else {
		lines = append(lines, StyleSuccess.Render("Deploy complete!"))
	}
	lines = append(lines, "")

	maxLines := maxHeight - 8
	if maxLines < 5 {
		maxLines = 5
	}
	lines = append(lines, styledDeployOutput(d.output, maxLines)...)

	titleColor := ColorProfit
	if d.err != nil {
		titleColor = ColorLoss
	}
	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Deploy Result", titleColor, content, panelWidth, false), 2)
}

func styledDeployOutput(output []string, maxLines int) []string {
	start := 0
	if len(output) > maxLines && maxLines > 0 {
		start = len(output) - maxLines
	}
	var lines []string
	for _, line := range output[start:] {
		style := StyleDim
		if strings.Contains(line, "ERROR") || strings.Contains(line, "failed") {
			style = StyleLoss
		} else if strings.Contains(line, "complete") || strings.Contains(line, "Health check") {
			style = StyleProfit
		} else if strings.HasPrefix(line, "[") {
			style = StyleAccent
		}
		lines = append(lines, style.Render(line))
	}
	return lines
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
