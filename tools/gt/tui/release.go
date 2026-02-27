package tui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/lipgloss"

	"gt/core"
)

// ReleaseStep tracks the release wizard step.
type ReleaseStep int

const (
	ReleaseSelectComponent ReleaseStep = iota
	ReleaseSelectBump
	ReleasePreview
	ReleaseExecuting
	ReleasePushConfirm
	ReleaseDone
)

func (s ReleaseStep) label() string {
	switch s {
	case ReleaseSelectComponent:
		return "Select Component"
	case ReleaseSelectBump:
		return "Select Bump"
	case ReleasePreview:
		return "Review"
	case ReleaseExecuting:
		return "Executing"
	case ReleasePushConfirm:
		return "Push"
	case ReleaseDone:
		return "Done"
	default:
		return ""
	}
}

// ReleaseFinishedMsg indicates the release completed.
type ReleaseFinishedMsg struct {
	Result *core.ReleaseResult
	Err    error
}

// PushDoneMsg indicates the push completed.
type PushDoneMsg struct {
	Err error
}

// ReleaseModel handles the multi-step release flow.
type ReleaseModel struct {
	root     string
	statuses []core.ComponentStatus
	step     ReleaseStep
	cursor   int
	compIdx  int
	bumpType core.BumpType
	preview  string
	result   *core.ReleaseResult
	err      error
	spinner  spinner.Model
	stepLog  []string
	Done     bool
}

// NewRelease creates a new release model.
func NewRelease() ReleaseModel {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = StyleSpinner
	return ReleaseModel{spinner: s}
}

func (r ReleaseModel) Start(root string, statuses []core.ComponentStatus) ReleaseModel {
	r.root = root
	r.statuses = statuses
	r.step = ReleaseSelectComponent
	r.cursor = 0
	r.compIdx = -1
	r.result = nil
	r.err = nil
	r.stepLog = nil
	r.Done = false
	return r
}

func (r ReleaseModel) Init() tea.Cmd {
	return r.spinner.Tick
}

func (r ReleaseModel) Update(msg tea.Msg) (ReleaseModel, tea.Cmd) {
	switch msg := msg.(type) {
	case spinner.TickMsg:
		var cmd tea.Cmd
		r.spinner, cmd = r.spinner.Update(msg)
		return r, cmd

	case ReleaseFinishedMsg:
		r.result = msg.Result
		r.err = msg.Err
		if msg.Err != nil {
			r.step = ReleaseDone
		} else {
			r.step = ReleasePushConfirm
		}
		return r, nil

	case PushDoneMsg:
		if msg.Err != nil {
			r.err = msg.Err
		}
		r.step = ReleaseDone
		return r, nil

	case tea.KeyMsg:
		return r.handleKey(msg)
	}

	return r, nil
}

func (r ReleaseModel) handleKey(msg tea.KeyMsg) (ReleaseModel, tea.Cmd) {
	switch r.step {
	case ReleaseSelectComponent:
		switch msg.String() {
		case "up", "k":
			if r.cursor > 0 {
				r.cursor--
			}
		case "down", "j":
			if r.cursor < len(r.statuses)-1 {
				r.cursor++
			}
		case "enter":
			r.compIdx = r.cursor
			r.step = ReleaseSelectBump
			if r.statuses[r.compIdx].IsDirty {
				r.bumpType = r.statuses[r.compIdx].SuggestedBump
				r.cursor = int(r.bumpType)
			} else {
				r.cursor = 0
			}
		}

	case ReleaseSelectBump:
		switch msg.String() {
		case "up", "k":
			if r.cursor > 0 {
				r.cursor--
			}
		case "down", "j":
			if r.cursor < 2 {
				r.cursor++
			}
		case "enter":
			r.bumpType = core.BumpType(r.cursor)
			r.step = ReleasePreview
			r.preview = r.buildPreview()
		case "esc":
			r.step = ReleaseSelectComponent
			r.cursor = r.compIdx
		}

	case ReleasePreview:
		switch msg.String() {
		case "enter", "y":
			r.step = ReleaseExecuting
			r.stepLog = nil
			return r, r.executeRelease()
		case "n", "esc":
			r.step = ReleaseSelectBump
		}

	case ReleasePushConfirm:
		switch msg.String() {
		case "y":
			r.stepLog = append(r.stepLog, "Pushing to remote...")
			root := r.root
			tag := r.result.Tag
			return r, func() tea.Msg {
				err := core.Push(root, false)
				if err == nil {
					err = core.PushTag(root, tag)
				}
				return PushDoneMsg{Err: err}
			}
		case "n":
			r.step = ReleaseDone
		}

	case ReleaseDone:
		switch msg.String() {
		case "enter", "q", "esc":
			r.Done = true
		}
	}

	return r, nil
}

func (r ReleaseModel) View(width, height int) string {
	panelWidth := width - 6
	if panelWidth > 104 {
		panelWidth = 104
	}

	var sections []string

	// Step indicator
	sections = append(sections, Indent(r.renderStepIndicator(), 2))
	sections = append(sections, "")

	switch r.step {
	case ReleaseSelectComponent:
		sections = append(sections, r.viewSelectComponent(panelWidth))
	case ReleaseSelectBump:
		sections = append(sections, r.viewSelectBump(panelWidth))
	case ReleasePreview:
		sections = append(sections, r.viewPreview(panelWidth))
	case ReleaseExecuting:
		sections = append(sections, r.viewExecuting(panelWidth))
	case ReleasePushConfirm:
		sections = append(sections, r.viewPushConfirm(panelWidth))
	case ReleaseDone:
		sections = append(sections, r.viewDone(panelWidth))
	}

	return strings.Join(sections, "\n")
}

func (r ReleaseModel) renderStepIndicator() string {
	steps := []ReleaseStep{ReleaseSelectComponent, ReleaseSelectBump, ReleasePreview, ReleaseExecuting, ReleaseDone}
	var parts []string
	for _, s := range steps {
		label := s.label()
		if s == r.step || (r.step == ReleasePushConfirm && s == ReleaseDone) {
			parts = append(parts, StyleAccent.Render("["+label+"]"))
		} else if s < r.step {
			parts = append(parts, StyleProfit.Render(label))
		} else {
			parts = append(parts, StyleDim.Render(label))
		}
	}
	return strings.Join(parts, StyleDim.Render(" > "))
}

func (r ReleaseModel) viewSelectComponent(panelWidth int) string {
	var lines []string

	for i, s := range r.statuses {
		prefix := "  "
		if i == r.cursor {
			prefix = StyleAccent.Render("> ")
		}
		name := ComponentStyle(s.Component.ID, fmt.Sprintf("%-8s", s.Component.Name))
		ver := StyleBright.Render("v" + s.Version)
		tag := ""
		if s.LatestTag != "" {
			tag = "  " + StyleDim.Render(s.LatestTag)
		} else {
			tag = "  " + StyleDim.Render("(no releases)")
		}

		status := ""
		if s.IsDirty {
			status = "  " + StyleWarning.Render(fmt.Sprintf("%d changes", len(s.ChangedFiles)))
			switch s.SuggestedBump {
			case core.BumpMajor:
				status += "  " + StyleLoss.Render("MAJOR")
			case core.BumpMinor:
				status += "  " + StyleWarning.Render("minor")
			default:
				status += "  " + StyleDim.Render("patch")
			}
		} else {
			status = "  " + StyleProfit.Render("clean")
		}

		lines = append(lines, fmt.Sprintf("%s%s  %s%s%s", prefix, name, ver, tag, status))
	}

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Select Component", ColorAccent, content, panelWidth, false), 2)
}

func (r ReleaseModel) viewSelectBump(panelWidth int) string {
	comp := r.statuses[r.compIdx]

	var lines []string
	lines = append(lines, RenderLabelValue("Component", ComponentStyle(comp.Component.ID, comp.Component.Name)))
	lines = append(lines, RenderLabelValue("Current", StyleBright.Render("v"+comp.Version)))
	lines = append(lines, "")

	bumpTypes := []struct {
		bt   core.BumpType
		desc string
	}{
		{core.BumpPatch, "Bug fixes, small changes"},
		{core.BumpMinor, "New features, backwards compatible"},
		{core.BumpMajor, "Breaking changes"},
	}

	sv, _ := core.ParseSemVer(comp.Version)
	for i, bt := range bumpTypes {
		prefix := "  "
		if i == r.cursor {
			prefix = StyleAccent.Render("> ")
		}
		newVer := sv.Bump(bt.bt).String()
		name := StyleBright.Render(fmt.Sprintf("%-6s", bt.bt.String()))
		arrow := StyleDim.Render(" -> ")
		ver := StyleProfit.Render(newVer)
		desc := StyleDim.Render("  " + bt.desc)

		suggested := ""
		if comp.IsDirty && bt.bt == comp.SuggestedBump {
			suggested = StyleWarning.Render(" (suggested)")
		}

		lines = append(lines, fmt.Sprintf("%s%s%s%s%s%s", prefix, name, arrow, ver, suggested, desc))
	}

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Select Version Bump", ColorAccent, content, panelWidth, false), 2)
}

func (r ReleaseModel) viewPreview(panelWidth int) string {
	content := r.preview + "\n" + StyleAccent.Render("Press enter to confirm, n to go back")
	return Indent(RenderCard("Review Changes", ColorWarning, content, panelWidth, false), 2)
}

func (r ReleaseModel) viewExecuting(panelWidth int) string {
	var lines []string
	lines = append(lines, r.spinner.View()+" "+StyleDim.Render("Releasing..."))
	lines = append(lines, "")
	for _, line := range r.stepLog {
		lines = append(lines, line)
	}
	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Executing Release", ColorWarning, content, panelWidth, false), 2)
}

func (r ReleaseModel) viewPushConfirm(panelWidth int) string {
	var lines []string
	if r.result != nil {
		lines = append(lines, StyleSuccess.Render("Release created successfully!"))
		lines = append(lines, "")
		lines = append(lines, RenderLabelValue("Tag", StyleBright.Render(r.result.Tag)))
		lines = append(lines, RenderLabelValue("Version", StyleDim.Render(r.result.OldVersion)+" "+StyleDim.Render("->")+" "+StyleProfit.Render(r.result.NewVersion)))
		if len(r.result.FilesChanged) > 0 {
			lines = append(lines, RenderLabelValue("Files", StyleDim.Render(strings.Join(r.result.FilesChanged, ", "))))
		}
		lines = append(lines, "")
	}
	lines = append(lines, StyleAccent.Render("Push to remote? (y/n)"))

	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Push Confirmation", ColorAccent, content, panelWidth, false), 2)
}

func (r ReleaseModel) viewDone(panelWidth int) string {
	var lines []string

	if r.err != nil {
		lines = append(lines, StyleError.Render("Error: "+r.err.Error()))
	} else if r.result != nil {
		lines = append(lines, StyleSuccess.Render("Release complete!"))
		lines = append(lines, "")
		lines = append(lines, RenderLabelValue("Tag", StyleBright.Render(r.result.Tag)))
		lines = append(lines, RenderLabelValue("Version", StyleDim.Render(r.result.OldVersion)+" "+StyleDim.Render("->")+" "+StyleProfit.Render(r.result.NewVersion)))
		if len(r.result.FilesChanged) > 0 {
			lines = append(lines, RenderLabelValue("Files", StyleDim.Render(strings.Join(r.result.FilesChanged, ", "))))
		}
		if r.result.Pushed {
			lines = append(lines, RenderLabelValue("Pushed", StyleProfit.Render("yes")))
		}
	}

	titleColor := ColorProfit
	if r.err != nil {
		titleColor = ColorLoss
	}
	content := strings.Join(lines, "\n")
	return Indent(RenderCard("Release Result", titleColor, content, panelWidth, false), 2)
}

func (r ReleaseModel) buildPreview() string {
	comp := r.statuses[r.compIdx]
	sv, _ := core.ParseSemVer(comp.Version)
	newVer := sv.Bump(r.bumpType).String()
	tag := comp.Component.TagPrefix + newVer

	var lines []string

	lines = append(lines, RenderLabelValue("Component", ComponentStyle(comp.Component.ID, comp.Component.Name)))
	lines = append(lines, RenderLabelValue("Bump", StyleBright.Render(r.bumpType.String())))
	lines = append(lines, RenderLabelValue("Version", StyleDim.Render(comp.Version)+" "+StyleDim.Render("->")+" "+StyleProfit.Render(newVer)))
	lines = append(lines, RenderLabelValue("Tag", StyleBright.Render(tag)))
	lines = append(lines, "")

	lines = append(lines, StyleDim.Render("Files to update:"))
	for _, vf := range comp.Component.Files {
		lines = append(lines, "  "+StyleDim.Render(vf.RelPath))
	}

	if len(comp.Commits) > 0 {
		lines = append(lines, "")
		lines = append(lines, StyleDim.Render("Commits since last release:"))
		limit := len(comp.Commits)
		if limit > 10 {
			limit = 10
		}
		for _, c := range comp.Commits[:limit] {
			cat := lipgloss.NewStyle().Foreground(ColorDim).Render(c.Category.Emoji())
			lines = append(lines, "  "+cat+" "+c.Subject)
		}
		if len(comp.Commits) > 10 {
			lines = append(lines, StyleDim.Render(fmt.Sprintf("  ... and %d more", len(comp.Commits)-10)))
		}
	}

	return strings.Join(lines, "\n")
}

func (r ReleaseModel) executeRelease() tea.Cmd {
	comp := r.statuses[r.compIdx].Component
	root := r.root
	bt := r.bumpType

	return func() tea.Msg {
		result, err := core.ExecuteRelease(core.ReleaseOpts{
			Root:      root,
			Component: comp,
			BumpType:  bt,
			DryRun:    false,
			Push:      false,
		})
		return ReleaseFinishedMsg{Result: result, Err: err}
	}
}
