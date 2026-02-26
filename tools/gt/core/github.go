package core

import (
	"fmt"
	"os/exec"
)

// CreateGitHubRelease creates a GitHub release using the gh CLI.
func CreateGitHubRelease(dir, tag, body string) error {
	args := []string{
		"release", "create", tag,
		"--title", tag,
		"--draft",
	}
	if body != "" {
		args = append(args, "--notes", body)
	} else {
		args = append(args, "--generate-notes")
	}

	cmd := exec.Command("gh", args...)
	cmd.Dir = dir
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("gh release create failed: %s: %w", string(out), err)
	}
	return nil
}

// GHInstalled checks if the GitHub CLI is available.
func GHInstalled() bool {
	_, err := exec.LookPath("gh")
	return err == nil
}
