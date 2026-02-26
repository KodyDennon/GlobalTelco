package core

import (
	"bufio"
	"fmt"
	"io"
	"os/exec"
	"strings"
)

// BuildStep describes a single build command.
type BuildStep struct {
	Name    string
	Command string
	Args    []string
	Dir     string // working directory (relative to project root)
}

// BuildOutput is sent for each line of build output.
type BuildOutput struct {
	Line  string
	Step  string
	Error bool
}

// GetBuildSteps returns the build steps for a component.
func GetBuildSteps(root, componentID string) []BuildStep {
	switch componentID {
	case "engine":
		return []BuildStep{
			{Name: "Cargo check", Command: "cargo", Args: []string{"check"}, Dir: root},
			{Name: "Cargo test", Command: "cargo", Args: []string{"test"}, Dir: root},
			{Name: "WASM build", Command: "wasm-pack", Args: []string{
				"build", "crates/gt-wasm", "--target", "web", "--out-dir", "../../web/src/lib/wasm/pkg",
			}, Dir: root},
		}
	case "server":
		return []BuildStep{
			{Name: "Cargo check", Command: "cargo", Args: []string{"check", "-p", "gt-server"}, Dir: root},
			{Name: "Cargo test", Command: "cargo", Args: []string{"test", "-p", "gt-server"}, Dir: root},
		}
	case "web":
		return []BuildStep{
			{Name: "Bun install", Command: "bun", Args: []string{"install"}, Dir: root + "/web"},
			{Name: "Svelte check", Command: "bun", Args: []string{"run", "check"}, Dir: root + "/web"},
			{Name: "Bun build", Command: "bun", Args: []string{"run", "build"}, Dir: root + "/web"},
		}
	case "desktop":
		return []BuildStep{
			{Name: "Cargo check (desktop)", Command: "cargo", Args: []string{"check"}, Dir: root + "/desktop/src-tauri"},
		}
	default:
		return nil
	}
}

// ExecuteBuild runs build steps for a component, streaming output to a channel.
func ExecuteBuild(root, componentID string, output chan<- BuildOutput) error {
	defer close(output)

	steps := GetBuildSteps(root, componentID)
	if steps == nil {
		return fmt.Errorf("unknown component: %s", componentID)
	}

	for _, step := range steps {
		output <- BuildOutput{Line: fmt.Sprintf(">>> %s", step.Name), Step: step.Name}

		if err := runStreamingCommand(step, output); err != nil {
			output <- BuildOutput{
				Line:  fmt.Sprintf("FAILED: %s: %v", step.Name, err),
				Step:  step.Name,
				Error: true,
			}
			return fmt.Errorf("%s failed: %w", step.Name, err)
		}

		output <- BuildOutput{Line: fmt.Sprintf("    %s complete", step.Name), Step: step.Name}
	}

	return nil
}

func runStreamingCommand(step BuildStep, output chan<- BuildOutput) error {
	cmd := exec.Command(step.Command, step.Args...)
	cmd.Dir = step.Dir

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return err
	}

	if err := cmd.Start(); err != nil {
		return err
	}

	// Stream stdout and stderr
	go streamLines(stdout, step.Name, false, output)
	streamLines(stderr, step.Name, true, output)

	return cmd.Wait()
}

func streamLines(r io.Reader, stepName string, isError bool, output chan<- BuildOutput) {
	scanner := bufio.NewScanner(r)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	for scanner.Scan() {
		output <- BuildOutput{
			Line:  scanner.Text(),
			Step:  stepName,
			Error: isError,
		}
	}
}

// ExecuteBuildSync runs build steps synchronously, printing to a callback.
func ExecuteBuildSync(root, componentID string, onLine func(string)) error {
	steps := GetBuildSteps(root, componentID)
	if steps == nil {
		return fmt.Errorf("unknown component: %s", componentID)
	}

	for _, step := range steps {
		if onLine != nil {
			onLine(fmt.Sprintf(">>> %s", step.Name))
		}

		cmd := exec.Command(step.Command, step.Args...)
		cmd.Dir = step.Dir

		out, err := cmd.CombinedOutput()
		if onLine != nil && len(out) > 0 {
			lines := splitLines(string(out))
			for _, line := range lines {
				onLine("    " + line)
			}
		}

		if err != nil {
			return fmt.Errorf("%s failed: %w", step.Name, err)
		}

		if onLine != nil {
			onLine(fmt.Sprintf("    %s complete", step.Name))
		}
	}

	return nil
}

func splitLines(s string) []string {
	if s == "" {
		return nil
	}
	return strings.Split(strings.TrimRight(s, "\n"), "\n")
}
