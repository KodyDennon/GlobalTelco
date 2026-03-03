package tui

import (
	"strings"

	"gt/config"
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
	ViewServer
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
	engineVer string
	branch    string
	dashboard DashboardModel
	release   ReleaseModel
	build     BuildModel
	deploy    DeployModel
	history   HistoryModel
	validate  ValidateModel
	server    ServerModel
	err       error
}

// NewApp creates the root TUI model.
func NewApp(root string) App {
	engineVer := "?.?.?"
	if comp := config.FindComponent("engine"); comp != nil {
		if v, err := core.ReadComponentVersion(root, *comp); err == nil {
			engineVer = v
		}
	}
	branch := core.CurrentBranch(root)

	return App{
		root:      root,
		view:      ViewDashboard,
		engineVer: engineVer,
		branch:    branch,
		dashboard: NewDashboard(),
		release:   NewRelease(),
		build:     NewBuild(),
		deploy:    NewDeploy(),
		history:   NewHistory(),
		validate:  NewValidate(),
		server:    NewServer(root),
	}
}

func (a App) Init() tea.Cmd {
	return tea.Batch(
		a.dashboard.Init(),
		a.refreshStatus(),
		a.dashboard.FetchServer(),
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
			if !a.isBusy() {
				a.view = ViewDashboard
				return a, a.refreshStatus()
			}
		case "esc":
			if a.view != ViewDashboard && !a.isBusy() {
				a.view = ViewDashboard
				return a, a.refreshStatus()
			}
		}

	case StatusMsg:
		a.statuses = msg.Statuses
		a.err = msg.Err
		a.dashboard = a.dashboard.SetStatuses(msg.Statuses)
		a.branch = core.CurrentBranch(a.root)
		for _, s := range msg.Statuses {
			if s.Component.ID == "engine" {
				a.engineVer = s.Version
				break
			}
		}

	case ServerStatusMsg:
		a.dashboard.serverStatus = msg.Status
		a.dashboard.serverFetched = true
		a.server.status = msg.Status
		a.server.statusFetched = true
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
	case ViewServer:
		a, cmd = a.updateServer(msg)
	}

	return a, cmd
}

func (a App) View() string {
	if a.width == 0 {
		return "Loading..."
	}

	w := a.width
	h := a.height

	// Header bar
	header := a.renderHeader(w)
	headerH := lipgloss.Height(header)

	// Footer help bar
	footer := RenderHelpBar(a.footerKeys(), w)
	footerH := lipgloss.Height(footer)

	// Content fills remaining space
	contentH := h - headerH - footerH
	if contentH < 4 {
		contentH = 4
	}

	var content string
	switch a.view {
	case ViewDashboard:
		content = a.dashboard.View(w, contentH)
	case ViewRelease:
		content = a.release.View(w, contentH)
	case ViewBuild:
		content = a.build.View(w, contentH)
	case ViewDeploy:
		content = a.deploy.View(w, contentH)
	case ViewHistory:
		content = a.history.View(w, contentH)
	case ViewValidate:
		content = a.validate.View(w, contentH)
	case ViewServer:
		content = a.server.View(w, contentH)
	}

	// Pad content to push footer to bottom
	rendered := lipgloss.Height(content)
	if rendered < contentH {
		content += strings.Repeat("\n", contentH-rendered)
	}

	return lipgloss.JoinVertical(lipgloss.Left, header, content, footer)
}

func (a App) renderHeader(width int) string {
	left := StyleTitle.Render("GT") + StyleDim.Render(" v"+a.engineVer)
	right := StyleDim.Render(a.branch) + "  " + StyleAccent.Render(a.viewName())
	return RenderHeaderBar(left, right, width)
}

func (a App) viewName() string {
	switch a.view {
	case ViewDashboard:
		return "Dashboard"
	case ViewRelease:
		return "Release"
	case ViewBuild:
		return "Build"
	case ViewDeploy:
		return "Deploy"
	case ViewHistory:
		return "History"
	case ViewValidate:
		return "Validate"
	case ViewServer:
		return "Server"
	default:
		return ""
	}
}

func (a App) footerKeys() []KeyBind {
	switch a.view {
	case ViewDashboard:
		return []KeyBind{
			{Key: "enter", Desc: "open"},
			{Key: "r", Desc: "release"},
			{Key: "b", Desc: "build"},
			{Key: "d", Desc: "deploy"},
			{Key: "s", Desc: "server"},
			{Key: "h", Desc: "history"},
			{Key: "v", Desc: "validate"},
			{Key: "R", Desc: "refresh"},
			{Key: "q", Desc: "quit"},
		}
	case ViewRelease:
		switch a.release.step {
		case ReleaseExecuting:
			return []KeyBind{{Key: "...", Desc: "releasing"}}
		case ReleasePushConfirm:
			return []KeyBind{
				{Key: "y", Desc: "push"},
				{Key: "n", Desc: "skip push"},
			}
		case ReleaseDone:
			return []KeyBind{{Key: "enter", Desc: "done"}}
		case ReleasePreview:
			return []KeyBind{
				{Key: "enter", Desc: "confirm"},
				{Key: "n", Desc: "go back"},
				{Key: "esc", Desc: "cancel"},
			}
		default:
			return []KeyBind{
				{Key: "j/k", Desc: "navigate"},
				{Key: "enter", Desc: "select"},
				{Key: "esc", Desc: "back"},
			}
		}
	case ViewBuild:
		if a.build.building {
			return []KeyBind{{Key: "...", Desc: "building"}}
		}
		if a.build.done {
			return []KeyBind{{Key: "esc", Desc: "back"}}
		}
		return []KeyBind{
			{Key: "j/k", Desc: "navigate"},
			{Key: "space", Desc: "toggle"},
			{Key: "enter", Desc: "build"},
			{Key: "esc", Desc: "back"},
		}
	case ViewDeploy:
		if a.deploy.deploying {
			return []KeyBind{{Key: "...", Desc: "deploying"}}
		}
		if a.deploy.done {
			return []KeyBind{{Key: "esc", Desc: "back"}}
		}
		return []KeyBind{
			{Key: "j/k", Desc: "navigate"},
			{Key: "space", Desc: "skip build"},
			{Key: "enter", Desc: "deploy"},
			{Key: "esc", Desc: "back"},
		}
	case ViewHistory:
		return []KeyBind{
			{Key: "j/k", Desc: "navigate"},
			{Key: "tab", Desc: "filter"},
			{Key: "esc", Desc: "back"},
		}
	case ViewValidate:
		return []KeyBind{
			{Key: "esc", Desc: "back"},
		}
	case ViewServer:
		if a.server.confirmRestart {
			return []KeyBind{
				{Key: "y", Desc: "confirm"},
				{Key: "n", Desc: "cancel"},
			}
		}
		if a.server.restarting {
			return []KeyBind{{Key: "...", Desc: "restarting"}}
		}
		return []KeyBind{
			{Key: "tab", Desc: "switch tab"},
			{Key: "l", Desc: "fetch logs"},
			{Key: "d", Desc: "download logs"},
			{Key: "r", Desc: "restart"},
			{Key: "R", Desc: "refresh"},
			{Key: "esc", Desc: "back"},
		}
	default:
		return []KeyBind{{Key: "esc", Desc: "back"}}
	}
}

func (a App) refreshStatus() tea.Cmd {
	root := a.root
	return func() tea.Msg {
		statuses, err := core.DetectDirtyComponents(root)
		return StatusMsg{Statuses: statuses, Err: err}
	}
}

func (a App) isBusy() bool {
	return a.build.building || a.deploy.deploying || a.release.step == ReleaseExecuting || a.server.restarting
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
		case "s":
			a.view = ViewServer
			a.server = a.server.Start()
			return a, a.server.Init()
		case "h":
			a.view = ViewHistory
			a.history = a.history.Start(a.root)
			return a, nil
		case "v":
			a.view = ViewValidate
			a.validate = a.validate.Start(a.root)
			return a, a.validate.Init()
		case "enter":
			idx := a.dashboard.cursor
			if idx < len(a.statuses) {
				a.view = ViewRelease
				a.release = a.release.Start(a.root, a.statuses)
				a.release.cursor = idx
				return a, a.release.Init()
			} else {
				a.view = ViewServer
				a.server = a.server.Start()
				return a, a.server.Init()
			}
		case "R":
			return a, tea.Batch(a.refreshStatus(), a.dashboard.FetchServer())
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

func (a App) updateServer(msg tea.Msg) (App, tea.Cmd) {
	s, cmd := a.server.Update(msg)
	a.server = s
	return a, cmd
}
