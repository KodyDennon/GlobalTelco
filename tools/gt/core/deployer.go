package core

import (
	"fmt"
	"os/exec"
	"strings"
	"time"

	"gt/config"
)

// DeployStep represents a step in the deploy pipeline.
type DeployStep int

const (
	DeployStepCrossCompile DeployStep = iota
	DeployStepUpload
	DeployStepInstall
	DeployStepHealthCheck
	DeployStepDone
)

func (s DeployStep) String() string {
	switch s {
	case DeployStepCrossCompile:
		return "Cross-compile"
	case DeployStepUpload:
		return "Upload binary"
	case DeployStepInstall:
		return "Install & restart"
	case DeployStepHealthCheck:
		return "Health check"
	case DeployStepDone:
		return "Done"
	default:
		return "Unknown"
	}
}

// DeployOpts configures a deployment.
type DeployOpts struct {
	Root      string
	Config    config.DeployConfig
	SkipBuild bool
	OnStep    func(step DeployStep, msg string)
	OnOutput  func(line string)
	OnError   func(step DeployStep, err error)
}

// ExecuteDeploy runs the deployment pipeline.
func ExecuteDeploy(opts DeployOpts) error {
	notify := func(step DeployStep, msg string) {
		if opts.OnStep != nil {
			opts.OnStep(step, msg)
		}
	}
	output := func(line string) {
		if opts.OnOutput != nil {
			opts.OnOutput(line)
		}
	}

	cfg := opts.Config

	// Step 1: Cross-compile
	if !opts.SkipBuild {
		notify(DeployStepCrossCompile, "Cross-compiling gt-server for x86_64 Linux (static musl)...")

		cmd := exec.Command("cargo", "zigbuild",
			"--target", "x86_64-unknown-linux-musl",
			"--release",
			"--bin", "gt-server",
			"--features", "postgres",
		)
		cmd.Dir = opts.Root
		out, err := cmd.CombinedOutput()
		output(string(out))
		if err != nil {
			return fmt.Errorf("cross-compile failed: %w", err)
		}
	} else {
		notify(DeployStepCrossCompile, "Skipping build (--skip-build)")
	}

	binaryPath := opts.Root + "/target/x86_64-unknown-linux-musl/release/gt-server"
	remote := fmt.Sprintf("%s@%s", cfg.SSHUser, cfg.Host)
	sshArgs := []string{"-i", cfg.SSHKey, "-o", "StrictHostKeyChecking=no", "-o", "ConnectTimeout=10"}

	// Step 2: Upload (using exec.Command with explicit args, no shell)
	notify(DeployStepUpload, fmt.Sprintf("Uploading binary to %s...", cfg.Host))

	scpArgs := append(sshArgs, binaryPath, remote+":/tmp/gt-server")
	out, err := exec.Command("scp", scpArgs...).CombinedOutput()
	output(string(out))
	if err != nil {
		return fmt.Errorf("upload failed: %w", err)
	}

	// Step 3: Install & restart (using exec.Command with explicit args)
	notify(DeployStepInstall, "Installing and restarting service...")

	installScript := fmt.Sprintf(
		"sudo systemctl stop %s 2>/dev/null || true && "+
			"sudo mv /tmp/gt-server %s && "+
			"sudo chmod +x %s && "+
			"sudo chown globaltelco:globaltelco %s && "+
			"sudo systemctl restart %s && "+
			"echo 'Service restarted'",
		cfg.ServiceName, cfg.BinaryPath, cfg.BinaryPath, cfg.BinaryPath, cfg.ServiceName,
	)

	sshExecArgs := append(sshArgs, remote, installScript)
	out, err = exec.Command("ssh", sshExecArgs...).CombinedOutput()
	output(string(out))
	if err != nil {
		return fmt.Errorf("install failed: %w", err)
	}

	// Step 4: Health check
	notify(DeployStepHealthCheck, "Running health check...")

	time.Sleep(3 * time.Second)

	healthURL := fmt.Sprintf("https://%s/health", cfg.Domain)
	healthCmd := exec.Command("curl", "-sf", "--max-time", "10", healthURL)
	out, err = healthCmd.CombinedOutput()
	if err != nil {
		output(fmt.Sprintf("HTTPS health check failed, trying HTTP..."))
		healthURL = fmt.Sprintf("http://%s/health", cfg.Domain)
		healthCmd = exec.Command("curl", "-sf", "--max-time", "10", healthURL)
		out, err = healthCmd.CombinedOutput()
		if err != nil {
			return fmt.Errorf("health check failed: %w", err)
		}
	}

	response := strings.TrimSpace(string(out))
	output(fmt.Sprintf("Health check: %s", response))

	notify(DeployStepDone, fmt.Sprintf("Deploy complete! Server running at %s", cfg.Domain))
	return nil
}
