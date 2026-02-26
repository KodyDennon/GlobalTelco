package tui

import (
	"gt/core"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// View represents the current TUI view.
type View int

const (
	ViewDashboard View = iota
	ViewRelease
	ViewBuild
	ViewDeploy
	ViewHistory
	ViewValidate
)

// StatusMsg carries refreshed component status data.
type StatusMsg struct {
	Statuses []core.ComponentStatus
	Err      error
}

// App is the root bubbletea model.
type App struct {
	root      string
	view      View
	width     int
	height    int
	statuses  []core.ComponentStatus
	dashboard DashboardModel
	release   ReleaseModel
	build     BuildModel
	deploy    DeployModel
	history   HistoryModel
	validate  ValidateModel
	err       error
}

// NewApp creates the root TUI model.
func NewApp(root string) App {
	return App{
		root:      root,
		view:      ViewDashboard,
		dashboard: NewDashboard(),
		release:   NewRelease(),
		build:     NewBuild(),
		deploy:    NewDeploy(),
		history:   NewHistory(),
		validate:  NewValidate(),
	}
}

func (a App) Init() tea.Cmd {
	return tea.Batch(
		a.dashboard.Init(),
		a.refreshStatus(),
	)
}

func (a App) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		a.width = msg.Width
		a.height = msg.Height

	case tea.KeyMsg:
		switch msg.String() {
		case "ctrl+c":
			return a, tea.Quit
		case "q":
			if a.view == ViewDashboard {
				return a, tea.Quit
			}
			// Return to dashboard from any view
			a.view = ViewDashboard
			return a, a.refreshStatus()
		case "esc":
			if a.view != ViewDashboard {
				a.view = ViewDashboard
				return a, a.refreshStatus()
			}
		}

	case StatusMsg:
		a.statuses = msg.Statuses
		a.err = msg.Err
		a.dashboard = a.dashboard.SetStatuses(msg.Statuses)
	}

	// Route to current view
	var cmd tea.Cmd
	switch a.view {
	case ViewDashboard:
		a, cmd = a.updateDashboard(msg)
	case ViewRelease:
		a, cmd = a.updateRelease(msg)
	case ViewBuild:
		a, cmd = a.updateBuild(msg)
	case ViewDeploy:
		a, cmd = a.updateDeploy(msg)
	case ViewHistory:
		a, cmd = a.updateHistory(msg)
	case ViewValidate:
		a, cmd = a.updateValidate(msg)
	}

	return a, cmd
}

func (a App) View() string {
	var content string

	switch a.view {
	case ViewDashboard:
		content = a.dashboard.View(a.width, a.height)
	case ViewRelease:
		content = a.release.View(a.width, a.height)
	case ViewBuild:
		content = a.build.View(a.width, a.height)
	case ViewDeploy:
		content = a.deploy.View(a.width, a.height)
	case ViewHistory:
		content = a.history.View(a.width, a.height)
	case ViewValidate:
		content = a.validate.View(a.width, a.height)
	}

	return lipgloss.Place(a.width, a.height, lipgloss.Left, lipgloss.Top, content)
}

func (a App) refreshStatus() tea.Cmd {
	root := a.root
	return func() tea.Msg {
		statuses, err := core.DetectDirtyComponents(root)
		return StatusMsg{Statuses: statuses, Err: err}
	}
}

func (a App) updateDashboard(msg tea.Msg) (App, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "r":
			a.view = ViewRelease
			a.release = a.release.Start(a.root, a.statuses)
			return a, a.release.Init()
		case "b":
			a.view = ViewBuild
			a.build = a.build.Start(a.root, a.statuses)
			return a, nil
		case "d":
			a.view = ViewDeploy
			a.deploy = a.deploy.Start(a.root)
			return a, nil
		case "h":
			a.view = ViewHistory
			a.history = a.history.Start(a.root)
			return a, nil
		case "v":
			a.view = ViewValidate
			a.validate = a.validate.Start(a.root)
			return a, a.validate.Init()
		case "R":
			return a, a.refreshStatus()
		}
	}

	d, cmd := a.dashboard.Update(msg)
	a.dashboard = d
	return a, cmd
}

func (a App) updateRelease(msg tea.Msg) (App, tea.Cmd) {
	r, cmd := a.release.Update(msg)
	a.release = r
	if a.release.Done {
		a.view = ViewDashboard
		return a, a.refreshStatus()
	}
	return a, cmd
}

func (a App) updateBuild(msg tea.Msg) (App, tea.Cmd) {
	b, cmd := a.build.Update(msg)
	a.build = b
	return a, cmd
}

func (a App) updateDeploy(msg tea.Msg) (App, tea.Cmd) {
	d, cmd := a.deploy.Update(msg)
	a.deploy = d
	return a, cmd
}

func (a App) updateHistory(msg tea.Msg) (App, tea.Cmd) {
	h, cmd := a.history.Update(msg)
	a.history = h
	return a, cmd
}

func (a App) updateValidate(msg tea.Msg) (App, tea.Cmd) {
	v, cmd := a.validate.Update(msg)
	a.validate = v
	return a, cmd
}
