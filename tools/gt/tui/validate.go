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
	var sb strings.Builder

	sb.WriteString(StyleTitle.Render("  Validate"))
	sb.WriteString("\n\n")

	if !v.done {
		sb.WriteString("  " + v.spinner.View() + " Checking version consistency...\n")
	} else {
		allOk := true
		for _, r := range v.results {
			name := ComponentStyle(r.Component.ID, r.Component.Name)

			if r.Err != nil {
				sb.WriteString(fmt.Sprintf("  %s %s  %s\n",
					StyleError.Render("FAIL"),
					name,
					StyleLoss.Render(r.Err.Error()),
				))
				allOk = false
				continue
			}

			if len(r.Mismatches) > 0 {
				sb.WriteString(fmt.Sprintf("  %s %s  v%s\n",
					StyleError.Render("FAIL"),
					name,
					r.Version,
				))
				for _, m := range r.Mismatches {
					sb.WriteString(fmt.Sprintf("         %s\n", StyleLoss.Render(m)))
				}
				allOk = false
			} else {
				sb.WriteString(fmt.Sprintf("  %s %s  v%s\n",
					StyleSuccess.Render(" OK "),
					name,
					r.Version,
				))
			}
		}

		sb.WriteString("\n")
		if allOk {
			sb.WriteString(StyleSuccess.Render("  All components consistent."))
		} else {
			sb.WriteString(StyleError.Render("  Version mismatches detected!"))
		}

		// Git status
		sb.WriteString("\n\n")
		root := v.root
		branch := core.CurrentBranch(root)
		clean := core.IsClean(root)
		sb.WriteString(fmt.Sprintf("  Branch: %s", StyleBright.Render(branch)))
		if clean {
			sb.WriteString(StyleProfit.Render("  (clean)"))
		} else {
			sb.WriteString(StyleWarning.Render("  (dirty)"))
		}
		sb.WriteString("\n")
	}

	sb.WriteString("\n")
	sb.WriteString("  " + StyleKey.Render("esc") + " " + StyleDim.Render("back"))

	return sb.String()
}
