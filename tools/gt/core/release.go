package core

import (
	"fmt"
	"strings"

	"gt/config"
)

// ReleaseStep represents a step in the release pipeline.
type ReleaseStep int

const (
	StepValidate ReleaseStep = iota
	StepBumpVersion
	StepChangelog
	StepCommit
	StepTag
	StepPush
	StepGitHubRelease
	StepDone
)

func (s ReleaseStep) String() string {
	switch s {
	case StepValidate:
		return "Validate"
	case StepBumpVersion:
		return "Bump version"
	case StepChangelog:
		return "Generate changelog"
	case StepCommit:
		return "Git commit"
	case StepTag:
		return "Git tag"
	case StepPush:
		return "Push to remote"
	case StepGitHubRelease:
		return "GitHub release"
	case StepDone:
		return "Done"
	default:
		return "Unknown"
	}
}

// ReleaseOpts configures a release execution.
type ReleaseOpts struct {
	Root       string
	Component  config.Component
	BumpType   BumpType
	DryRun     bool
	Push       bool
	GitHubRelease bool
	OnStep     func(step ReleaseStep, msg string)
	OnError    func(step ReleaseStep, err error)
}

// ReleaseResult contains the result of a release execution.
type ReleaseResult struct {
	OldVersion  string
	NewVersion  string
	Tag         string
	Changelog   string
	FilesChanged []string
	Pushed      bool
}

// ExecuteRelease runs the full release pipeline with step callbacks.
func ExecuteRelease(opts ReleaseOpts) (*ReleaseResult, error) {
	notify := func(step ReleaseStep, msg string) {
		if opts.OnStep != nil {
			opts.OnStep(step, msg)
		}
	}
	notifyErr := func(step ReleaseStep, err error) {
		if opts.OnError != nil {
			opts.OnError(step, err)
		}
	}

	result := &ReleaseResult{}

	// Step 1: Validate
	notify(StepValidate, "Checking version consistency...")

	oldVersion, mismatches, err := ValidateComponentVersions(opts.Root, opts.Component)
	if err != nil {
		notifyErr(StepValidate, err)
		return nil, fmt.Errorf("validation failed: %w", err)
	}
	if len(mismatches) > 0 {
		err := fmt.Errorf("version mismatch: %s", mismatches[0])
		notifyErr(StepValidate, err)
		return nil, err
	}
	result.OldVersion = oldVersion

	sv, err := ParseSemVer(oldVersion)
	if err != nil {
		notifyErr(StepValidate, err)
		return nil, fmt.Errorf("invalid current version %q: %w", oldVersion, err)
	}

	newSV := sv.Bump(opts.BumpType)
	result.NewVersion = newSV.String()
	result.Tag = opts.Component.TagPrefix + result.NewVersion

	if TagExists(opts.Root, result.Tag) {
		err := fmt.Errorf("tag %s already exists", result.Tag)
		notifyErr(StepValidate, err)
		return nil, err
	}

	notify(StepValidate, fmt.Sprintf("Version: %s -> %s", oldVersion, result.NewVersion))

	if opts.DryRun {
		notify(StepDone, "Dry run complete (no changes made)")
		return result, nil
	}

	// Step 2: Bump version files
	notify(StepBumpVersion, fmt.Sprintf("Updating version files to %s...", result.NewVersion))

	if err := WriteComponentVersion(opts.Root, opts.Component, result.NewVersion); err != nil {
		notifyErr(StepBumpVersion, err)
		return nil, fmt.Errorf("failed to write version: %w", err)
	}

	for _, vf := range opts.Component.Files {
		result.FilesChanged = append(result.FilesChanged, vf.RelPath)
	}

	// Step 3: Generate changelog
	notify(StepChangelog, "Generating changelog...")

	latestTag := LatestTag(opts.Root, opts.Component)
	var commits []CategorizedCommit
	if latestTag != "" {
		logLines := LogSinceTag(opts.Root, latestTag)
		for _, line := range logLines {
			if parts := splitOnce(line, " "); parts != nil {
				commits = append(commits, CategorizeCommit(parts[0], parts[1]))
			}
		}
	}

	changelog := GenerateChangelog(result.NewVersion, commits)
	result.Changelog = changelog

	if changelog != "" {
		if err := UpdateChangelogFile(opts.Root, opts.Component.ID, changelog); err != nil {
			notifyErr(StepChangelog, err)
			return nil, fmt.Errorf("failed to update changelog: %w", err)
		}
		changelogFile := "CHANGELOG.md"
		if opts.Component.ID != "engine" {
			changelogFile = fmt.Sprintf("CHANGELOG-%s.md", opts.Component.ID)
		}
		result.FilesChanged = append(result.FilesChanged, changelogFile)
	}

	// Step 4: Git commit
	notify(StepCommit, "Creating commit...")

	if err := StageFiles(opts.Root, result.FilesChanged); err != nil {
		notifyErr(StepCommit, err)
		return nil, fmt.Errorf("failed to stage files: %w", err)
	}

	commitMsg := fmt.Sprintf("release(%s): v%s", opts.Component.ID, result.NewVersion)
	if err := Commit(opts.Root, commitMsg); err != nil {
		notifyErr(StepCommit, err)
		return nil, fmt.Errorf("failed to commit: %w", err)
	}

	// Step 5: Git tag
	notify(StepTag, fmt.Sprintf("Creating tag %s...", result.Tag))

	tagMsg := fmt.Sprintf("%s v%s", opts.Component.Name, result.NewVersion)
	if err := CreateTag(opts.Root, result.Tag, tagMsg); err != nil {
		notifyErr(StepTag, err)
		return nil, fmt.Errorf("failed to create tag: %w", err)
	}

	// Step 6: Push (optional)
	if opts.Push {
		notify(StepPush, "Pushing to remote...")
		if err := Push(opts.Root, false); err != nil {
			notifyErr(StepPush, err)
			return nil, fmt.Errorf("failed to push: %w", err)
		}
		if err := PushTag(opts.Root, result.Tag); err != nil {
			notifyErr(StepPush, err)
			return nil, fmt.Errorf("failed to push tag: %w", err)
		}
		result.Pushed = true
	}

	// Step 7: GitHub release (optional)
	if opts.GitHubRelease && opts.Push {
		notify(StepGitHubRelease, "Creating GitHub release...")
		if err := CreateGitHubRelease(opts.Root, result.Tag, result.Changelog); err != nil {
			notifyErr(StepGitHubRelease, err)
			// Non-fatal: just report the error
		}
	}

	notify(StepDone, fmt.Sprintf("Released %s v%s", opts.Component.Name, result.NewVersion))
	return result, nil
}

func splitOnce(s, sep string) []string {
	parts := strings.SplitN(s, sep, 2)
	if len(parts) != 2 {
		return nil
	}
	return parts
}
