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
	root       string
	statuses   []core.ComponentStatus
	step       ReleaseStep
	cursor     int
	compIdx    int
	bumpType   core.BumpType
	preview    string
	result     *core.ReleaseResult
	err        error
	spinner    spinner.Model
	stepLog    []string
	Done       bool
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
			// Pre-select suggested bump
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
		}

	case ReleasePreview:
		switch msg.String() {
		case "enter", "y":
			r.step = ReleaseExecuting
			r.stepLog = nil
			return r, r.executeRelease()
		case "n":
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
	var sb strings.Builder

	switch r.step {
	case ReleaseSelectComponent:
		sb.WriteString(StyleSubtitle.Render("  Select component to release:"))
		sb.WriteString("\n\n")
		for i, s := range r.statuses {
			prefix := "  "
			if i == r.cursor {
				prefix = StyleAccent.Render("> ")
			}
			name := ComponentStyle(s.Component.ID, s.Component.Name)
			ver := StyleDim.Render("v" + s.Version)
			status := ""
			if s.IsDirty {
				status = StyleWarning.Render(" (dirty)")
			} else {
				status = StyleDim.Render(" (clean)")
			}
			sb.WriteString(fmt.Sprintf("%s%s  %s%s\n", prefix, name, ver, status))
		}

	case ReleaseSelectBump:
		comp := r.statuses[r.compIdx]
		sb.WriteString(fmt.Sprintf("  %s v%s\n\n", ComponentStyle(comp.Component.ID, comp.Component.Name), comp.Version))
		sb.WriteString(StyleSubtitle.Render("  Select version bump:"))
		sb.WriteString("\n\n")

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

			sb.WriteString(fmt.Sprintf("%s%s%s%s%s%s\n", prefix, name, arrow, ver, suggested, desc))
		}

	case ReleasePreview:
		sb.WriteString(StyleSubtitle.Render("  Review changes:"))
		sb.WriteString("\n\n")
		sb.WriteString(r.preview)
		sb.WriteString("\n")
		sb.WriteString(StyleAccent.Render("  Press enter to confirm, n to go back"))

	case ReleaseExecuting:
		sb.WriteString("  " + r.spinner.View() + " Releasing...\n\n")
		for _, line := range r.stepLog {
			sb.WriteString("  " + line + "\n")
		}

	case ReleasePushConfirm:
		if r.result != nil {
			sb.WriteString(StyleSuccess.Render("  Release complete!"))
			sb.WriteString("\n\n")
			sb.WriteString(fmt.Sprintf("  %s: %s -> %s\n", r.result.Tag, r.result.OldVersion, r.result.NewVersion))
			sb.WriteString("\n")
		}
		sb.WriteString(StyleAccent.Render("  Push to remote? (y/n)"))

	case ReleaseDone:
		if r.err != nil {
			sb.WriteString(StyleError.Render(fmt.Sprintf("  Error: %v", r.err)))
		} else if r.result != nil {
			sb.WriteString(StyleSuccess.Render("  Release complete!"))
			sb.WriteString("\n\n")
			sb.WriteString(fmt.Sprintf("  Tag: %s\n", r.result.Tag))
			sb.WriteString(fmt.Sprintf("  Version: %s -> %s\n", r.result.OldVersion, r.result.NewVersion))
			if len(r.result.FilesChanged) > 0 {
				sb.WriteString(fmt.Sprintf("  Files: %s\n", strings.Join(r.result.FilesChanged, ", ")))
			}
		}
		sb.WriteString("\n")
		sb.WriteString(StyleDim.Render("  Press enter to return to dashboard"))
	}

	return sb.String()
}

func (r ReleaseModel) buildPreview() string {
	comp := r.statuses[r.compIdx]
	sv, _ := core.ParseSemVer(comp.Version)
	newVer := sv.Bump(r.bumpType).String()
	tag := comp.Component.TagPrefix + newVer

	var sb strings.Builder

	sb.WriteString(fmt.Sprintf("  Component:   %s\n", ComponentStyle(comp.Component.ID, comp.Component.Name)))
	sb.WriteString(fmt.Sprintf("  Bump:        %s\n", r.bumpType.String()))
	sb.WriteString(fmt.Sprintf("  Version:     %s -> %s\n", comp.Version, StyleProfit.Render(newVer)))
	sb.WriteString(fmt.Sprintf("  Tag:         %s\n", tag))
	sb.WriteString("\n")

	sb.WriteString(StyleDim.Render("  Files to update:"))
	sb.WriteString("\n")
	for _, vf := range comp.Component.Files {
		sb.WriteString(fmt.Sprintf("    %s\n", vf.RelPath))
	}

	if len(comp.Commits) > 0 {
		sb.WriteString("\n")
		sb.WriteString(StyleDim.Render("  Commits since last release:"))
		sb.WriteString("\n")
		limit := len(comp.Commits)
		if limit > 10 {
			limit = 10
		}
		for _, c := range comp.Commits[:limit] {
			cat := lipgloss.NewStyle().Foreground(ColorDim).Render(c.Category.Emoji())
			sb.WriteString(fmt.Sprintf("    %s %s\n", cat, c.Subject))
		}
		if len(comp.Commits) > 10 {
			sb.WriteString(StyleDim.Render(fmt.Sprintf("    ... and %d more\n", len(comp.Commits)-10)))
		}
	}

	return sb.String()
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
