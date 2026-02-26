package tui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"

	"gt/config"
	"gt/core"
)

// HistoryModel displays release tag history.
type HistoryModel struct {
	root    string
	entries []historyEntry
	cursor  int
	filter  int // 0=all, 1=engine, 2=server, 3=web, 4=desktop
}

type historyEntry struct {
	Tag       string
	CompID    string
	CompName  string
}

// NewHistory creates a new history model.
func NewHistory() HistoryModel {
	return HistoryModel{}
}

func (h HistoryModel) Start(root string) HistoryModel {
	h.root = root
	h.cursor = 0
	h.filter = 0
	h.entries = h.loadEntries()
	return h
}

func (h HistoryModel) loadEntries() []historyEntry {
	var entries []historyEntry
	for _, comp := range config.Components() {
		tags := core.AllTagsForComponent(h.root, comp)
		for _, tag := range tags {
			entries = append(entries, historyEntry{
				Tag:      tag,
				CompID:   comp.ID,
				CompName: comp.Name,
			})
		}
	}
	return entries
}

func (h HistoryModel) Update(msg tea.Msg) (HistoryModel, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "up", "k":
			if h.cursor > 0 {
				h.cursor--
			}
		case "down", "j":
			filtered := h.filteredEntries()
			if h.cursor < len(filtered)-1 {
				h.cursor++
			}
		case "tab":
			h.filter = (h.filter + 1) % 5
			h.cursor = 0
		}
	}
	return h, nil
}

func (h HistoryModel) View(width, height int) string {
	var sb strings.Builder

	// Filter tabs
	filters := []string{"All", "Engine", "Server", "Web", "Desktop"}
	var tabs []string
	for i, f := range filters {
		if i == h.filter {
			tabs = append(tabs, StyleSelected.Render("["+f+"]"))
		} else {
			tabs = append(tabs, StyleUnselected.Render(" "+f+" "))
		}
	}
	sb.WriteString("  " + strings.Join(tabs, " "))
	sb.WriteString("\n\n")

	// Entries
	entries := h.filteredEntries()
	if len(entries) == 0 {
		sb.WriteString(StyleDim.Render("  No release tags found"))
	} else {
		maxShow := height - 10
		if maxShow < 5 {
			maxShow = 5
		}
		start := 0
		if h.cursor >= maxShow {
			start = h.cursor - maxShow + 1
		}
		end := start + maxShow
		if end > len(entries) {
			end = len(entries)
		}

		for i := start; i < end; i++ {
			e := entries[i]
			prefix := "  "
			if i == h.cursor {
				prefix = StyleAccent.Render("> ")
			}
			name := ComponentStyle(e.CompID, fmt.Sprintf("%-8s", e.CompName))
			tag := StyleBright.Render(e.Tag)
			sb.WriteString(fmt.Sprintf("%s%s  %s\n", prefix, name, tag))
		}

		if len(entries) > maxShow {
			sb.WriteString(StyleDim.Render(fmt.Sprintf("\n  Showing %d-%d of %d", start+1, end, len(entries))))
		}
	}

	return sb.String()
}

func (h HistoryModel) filteredEntries() []historyEntry {
	if h.filter == 0 {
		return h.entries
	}

	compIDs := []string{"", "engine", "server", "web", "desktop"}
	filterID := compIDs[h.filter]

	var filtered []historyEntry
	for _, e := range h.entries {
		if e.CompID == filterID {
			filtered = append(filtered, e)
		}
	}
	return filtered
}
