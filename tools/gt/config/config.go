package config

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// VersionFileType describes how a version is stored in a file.
type VersionFileType int

const (
	PlainText      VersionFileType = iota // VERSION file: just "0.1.0\n"
	CargoWorkspace                        // Cargo.toml: [workspace.package] version = "X.Y.Z"
	CargoPackage                          // Cargo.toml: [package] version = "X.Y.Z"
	PackageJSON                           // package.json: "version": "X.Y.Z"
	TauriConf                             // tauri.conf.json: top-level "version": "X.Y.Z"
	SvelteFallback                        // MainMenu.svelte: __APP_VERSION__ fallback literal
)

// VersionFile defines a single file that contains a version string.
type VersionFile struct {
	RelPath  string          // Path relative to project root
	FileType VersionFileType // How to read/write the version
}

// Component defines an independently versioned component.
type Component struct {
	Name           string        // Display name
	ID             string        // Short identifier
	TagPrefix      string        // Git tag prefix (e.g., "engine-v")
	LegacyPrefixes []string     // Old tag prefixes to also search (migration)
	Color          string        // Hex color for TUI display
	Files          []VersionFile // Version files to update
	DirtyPaths     []string      // Paths to check for changes since last tag
}

// DeployConfig holds deployment configuration.
type DeployConfig struct {
	Host        string
	SSHKey      string
	SSHUser     string
	Domain      string
	ServiceName string
	BinaryPath  string
}

// Components returns the 4 independent components.
func Components() []Component {
	return []Component{
		{
			Name:           "Engine",
			ID:             "engine",
			TagPrefix:      "engine-v",
			LegacyPrefixes: []string{"app-v"},
			Color:          "#00BFFF",
			Files: []VersionFile{
				{RelPath: "VERSION", FileType: PlainText},
				{RelPath: "Cargo.toml", FileType: CargoWorkspace},
				{RelPath: "web/src/lib/menu/MainMenu.svelte", FileType: SvelteFallback},
			},
			DirtyPaths: []string{
				"crates/gt-common/",
				"crates/gt-simulation/",
				"crates/gt-world/",
				"crates/gt-economy/",
				"crates/gt-infrastructure/",
				"crates/gt-population/",
				"crates/gt-ai/",
				"crates/gt-bridge/",
				"crates/gt-wasm/",
				"crates/gt-tauri/",
				"Cargo.toml",
				"Cargo.lock",
			},
		},
		{
			Name:      "Server",
			ID:        "server",
			TagPrefix: "server-v",
			Color:     "#00FF7F",
			Files: []VersionFile{
				{RelPath: "crates/gt-server/Cargo.toml", FileType: CargoPackage},
			},
			DirtyPaths: []string{
				"crates/gt-server/",
			},
		},
		{
			Name:      "Web",
			ID:        "web",
			TagPrefix: "web-v",
			Color:     "#FFD700",
			Files: []VersionFile{
				{RelPath: "web/package.json", FileType: PackageJSON},
			},
			DirtyPaths: []string{
				"web/",
			},
		},
		{
			Name:      "Desktop",
			ID:        "desktop",
			TagPrefix: "desktop-v",
			Color:     "#FF69B4",
			Files: []VersionFile{
				{RelPath: "desktop/src-tauri/Cargo.toml", FileType: CargoPackage},
				{RelPath: "desktop/src-tauri/tauri.conf.json", FileType: TauriConf},
			},
			DirtyPaths: []string{
				"desktop/",
			},
		},
		{
			Name:      "Admin",
			ID:        "admin",
			TagPrefix: "admin-v",
			Color:     "#FF4500",
			Files: []VersionFile{
				{RelPath: "admin/package.json", FileType: PackageJSON},
			},
			DirtyPaths: []string{
				"admin/",
			},
		},
	}
}

// FindComponent returns a component by ID, or nil if not found.
func FindComponent(id string) *Component {
	for _, c := range Components() {
		if c.ID == id {
			return &c
		}
	}
	return nil
}

// ComponentIDs returns a list of valid component IDs.
func ComponentIDs() []string {
	comps := Components()
	ids := make([]string, len(comps))
	for i, c := range comps {
		ids[i] = c.ID
	}
	return ids
}

// DefaultDeployConfig returns the deploy configuration.
// Host and SSH key are read from environment variables (ORACLE_IP, SSH_KEY)
// to avoid hardcoding infrastructure details in source code.
func DefaultDeployConfig() DeployConfig {
	home, _ := os.UserHomeDir()
	host := envOrDotenv("ORACLE_IP")
	sshKey := envOrDotenv("SSH_KEY")
	if sshKey == "" {
		sshKey = filepath.Join(home, ".ssh", "oracle_globaltelco")
	}
	sshUser := envOrDotenv("SSH_USER")
	if sshUser == "" {
		sshUser = "ubuntu"
	}
	return DeployConfig{
		Host:        host,
		SSHKey:      sshKey,
		SSHUser:     sshUser,
		Domain:      "server.globaltelco.online",
		ServiceName: "globaltelco",
		BinaryPath:  "/opt/globaltelco/gt-server",
	}
}

// envFromDotenv reads a key from the project root .env file.
// Returns empty string if the file doesn't exist or key isn't found.
func envFromDotenv(key string) string {
	root, err := FindProjectRoot()
	if err != nil {
		return ""
	}
	f, err := os.Open(filepath.Join(root, ".env"))
	if err != nil {
		return ""
	}
	defer f.Close()
	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		k, v, ok := strings.Cut(line, "=")
		if ok && strings.TrimSpace(k) == key {
			return strings.TrimSpace(v)
		}
	}
	return ""
}

// envOrDotenv reads from OS environment first, then falls back to .env file.
func envOrDotenv(key string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return envFromDotenv(key)
}

// FindProjectRoot walks up from cwd looking for the VERSION file.
func FindProjectRoot() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", err
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "VERSION")); err == nil {
			return dir, nil
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return "", fmt.Errorf("could not find project root (no VERSION file found)")
		}
		dir = parent
	}
}
