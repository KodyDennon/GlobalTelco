package tui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/bubbles/spinner"

	"gt/config"
	"gt/core"
)

// ValidateResultMsg carries validation results.
type ValidateResultMsg struct {
	Results []validateResult
}

type validateResult struct {
	Component  config.Component
	Version    string
	Mismatches []string
	Err        error
}

// ValidateModel handles the validation view.
type ValidateModel struct {
	root    string
	results []validateResult
	done    bool
	spinner spinner.Model
}

// NewValidate creates a new validate model.
func NewValidate() ValidateModel {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = StyleSpinner
	return ValidateModel{spinner: s}
}

func (v ValidateModel) Start(root string) ValidateModel {
	v.root = root
	v.results = nil
	v.done = false
	return v
}

func (v ValidateModel) Init() tea.Cmd {
	root := v.root
	return tea.Batch(
		v.spinner.Tick,
		func() tea.Msg {
			var results []validateResult
			for _, comp := range config.Components() {
				version, mismatches, err := core.ValidateComponentVersions(root, comp)
				results = append(results, validateResult{
					Component:  comp,
					Version:    version,
					Mismatches: mismatches,
					Err:        err,
				})
			}
			return ValidateResultMsg{Results: results}
		},
	)
}

func (v ValidateModel) Update(msg tea.Msg) (ValidateModel, tea.Cmd) {
	switch msg := msg.(type) {
	case spinner.TickMsg:
		if !v.done {
			var cmd tea.Cmd
			v.spinner, cmd = v.spinner.Update(msg)
			return v, cmd
		}

	case ValidateResultMsg:
		v.results = msg.Results
		v.done = true
	}
	return v, nil
}

func (v ValidateModel) View(width, height int) string {
	panelWidth := width - 6
	if panelWidth > 104 {
		panelWidth = 104
	}

	if !v.done {
		content := v.spinner.View() + " " + StyleDim.Render("Checking version consistency...")
		return Indent(RenderCard("Validate Versions", ColorAccent, content, panelWidth, false), 2)
	}

	var sections []string

	// Results card
	var lines []string
	allOk := true
	for _, r := range v.results {
		name := ComponentStyle(r.Component.ID, fmt.Sprintf("%-8s", r.Component.Name))

		if r.Err != nil {
			lines = append(lines, fmt.Sprintf("%s %s  %s",
				StyleError.Render("FAIL"),
				name,
				StyleLoss.Render(r.Err.Error()),
			))
			allOk = false
			continue
		}

		if len(r.Mismatches) > 0 {
			lines = append(lines, fmt.Sprintf("%s %s  v%s",
				StyleError.Render("FAIL"),
				name,
				r.Version,
			))
			for _, m := range r.Mismatches {
				lines = append(lines, "       "+StyleLoss.Render(m))
			}
			allOk = false
		} else {
			lines = append(lines, fmt.Sprintf("%s %s  v%s",
				StyleSuccess.Render(" OK "),
				name,
				r.Version,
			))

			// Show which files were checked
			fileCount := len(r.Component.Files)
			lines = append(lines, "       "+StyleDim.Render(fmt.Sprintf("%d version files consistent", fileCount)))
		}
	}

	lines = append(lines, "")
	if allOk {
		lines = append(lines, StyleSuccess.Render("All components consistent."))
	} else {
		lines = append(lines, StyleError.Render("Version mismatches detected!"))
	}

	titleColor := ColorProfit
	if !allOk {
		titleColor = ColorLoss
	}

	content := strings.Join(lines, "\n")
	sections = append(sections, Indent(RenderCard("Version Consistency", titleColor, content, panelWidth, false), 2))

	// Git status card
	root := v.root
	branch := core.CurrentBranch(root)
	clean := core.IsClean(root)

	var gitLines []string
	gitLines = append(gitLines, RenderLabelValue("Branch", StyleBright.Render(branch)))
	if clean {
		gitLines = append(gitLines, RenderLabelValue("Working tree", StyleProfit.Render("clean")))
	} else {
		gitLines = append(gitLines, RenderLabelValue("Working tree", StyleWarning.Render("dirty")))
	}

	gitContent := strings.Join(gitLines, "\n")
	sections = append(sections, Indent(RenderCard("Git Status", ColorAccent, gitContent, panelWidth, false), 2))

	return strings.Join(sections, "\n")
}
