package core

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"gt/config"
)

// ReadVersion reads the version from a single version file.
func ReadVersion(root string, vf config.VersionFile) (string, error) {
	path := filepath.Join(root, vf.RelPath)
	data, err := os.ReadFile(path)
	if err != nil {
		return "", fmt.Errorf("reading %s: %w", vf.RelPath, err)
	}
	content := string(data)

	switch vf.FileType {
	case config.PlainText:
		return strings.TrimSpace(content), nil

	case config.CargoWorkspace:
		return readCargoVersion(content, "workspace.package")

	case config.CargoPackage:
		return readCargoVersion(content, "package")

	case config.PackageJSON:
		return readJSONVersion(content)

	case config.TauriConf:
		return readJSONVersion(content)

	case config.SvelteFallback:
		return readSvelteFallback(content)

	default:
		return "", fmt.Errorf("unknown file type for %s", vf.RelPath)
	}
}

// WriteVersion writes a version to a single version file, preserving formatting.
func WriteVersion(root string, vf config.VersionFile, version string) error {
	path := filepath.Join(root, vf.RelPath)
	data, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("reading %s: %w", vf.RelPath, err)
	}
	content := string(data)
	var updated string

	switch vf.FileType {
	case config.PlainText:
		updated = version + "\n"

	case config.CargoWorkspace:
		updated, err = writeCargoVersion(content, "workspace.package", version)

	case config.CargoPackage:
		updated, err = writeCargoVersion(content, "package", version)

	case config.PackageJSON:
		updated, err = writeJSONVersion(content, version)

	case config.TauriConf:
		updated, err = writeJSONVersion(content, version)

	case config.SvelteFallback:
		updated, err = writeSvelteFallback(content, version)

	default:
		return fmt.Errorf("unknown file type for %s", vf.RelPath)
	}

	if err != nil {
		return fmt.Errorf("updating %s: %w", vf.RelPath, err)
	}

	return os.WriteFile(path, []byte(updated), 0644)
}

// ReadComponentVersion reads the primary version for a component (first file).
func ReadComponentVersion(root string, comp config.Component) (string, error) {
	if len(comp.Files) == 0 {
		return "", fmt.Errorf("component %s has no version files", comp.ID)
	}
	return ReadVersion(root, comp.Files[0])
}

// WriteComponentVersion writes a version to all version files for a component.
func WriteComponentVersion(root string, comp config.Component, version string) error {
	for _, vf := range comp.Files {
		if err := WriteVersion(root, vf, version); err != nil {
			return err
		}
	}
	return nil
}

// ValidateComponentVersions checks that all version files for a component agree.
func ValidateComponentVersions(root string, comp config.Component) (string, []string, error) {
	var versions []string
	var mismatches []string

	for _, vf := range comp.Files {
		v, err := ReadVersion(root, vf)
		if err != nil {
			return "", nil, err
		}
		versions = append(versions, v)
	}

	if len(versions) == 0 {
		return "", nil, fmt.Errorf("component %s has no version files", comp.ID)
	}

	primary := versions[0]
	for i := 1; i < len(versions); i++ {
		if versions[i] != primary {
			mismatches = append(mismatches, fmt.Sprintf(
				"%s has %s (expected %s)",
				comp.Files[i].RelPath, versions[i], primary,
			))
		}
	}

	return primary, mismatches, nil
}

// --- Internal parsers ---

func readCargoVersion(content, section string) (string, error) {
	re := regexp.MustCompile(`(?m)^\[` + regexp.QuoteMeta(section) + `\]\s*\n(?:.*\n)*?version\s*=\s*"([^"]+)"`)
	matches := re.FindStringSubmatch(content)
	if matches == nil {
		// Try simpler: find section header, then scan for version
		return readCargoVersionScan(content, section)
	}
	return matches[1], nil
}

func readCargoVersionScan(content, section string) (string, error) {
	lines := strings.Split(content, "\n")
	header := "[" + section + "]"
	inSection := false
	sectionRe := regexp.MustCompile(`^\[`)
	versionRe := regexp.MustCompile(`^version\s*=\s*"([^"]+)"`)

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == header {
			inSection = true
			continue
		}
		if inSection && sectionRe.MatchString(trimmed) {
			break // entered a new section
		}
		if inSection {
			if m := versionRe.FindStringSubmatch(trimmed); m != nil {
				return m[1], nil
			}
		}
	}
	return "", fmt.Errorf("version not found in [%s] section", section)
}

func writeCargoVersion(content, section, version string) (string, error) {
	lines := strings.Split(content, "\n")
	header := "[" + section + "]"
	inSection := false
	sectionRe := regexp.MustCompile(`^\[`)
	versionRe := regexp.MustCompile(`^(\s*version\s*=\s*)"[^"]+"`)
	found := false

	for i, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == header {
			inSection = true
			continue
		}
		if inSection && sectionRe.MatchString(trimmed) {
			break
		}
		if inSection {
			if versionRe.MatchString(line) {
				lines[i] = versionRe.ReplaceAllString(line, `${1}"`+version+`"`)
				found = true
				break
			}
		}
	}

	if !found {
		return "", fmt.Errorf("version not found in [%s] section", section)
	}
	return strings.Join(lines, "\n"), nil
}

func readJSONVersion(content string) (string, error) {
	var obj map[string]json.RawMessage
	if err := json.Unmarshal([]byte(content), &obj); err != nil {
		return "", fmt.Errorf("parsing JSON: %w", err)
	}
	raw, ok := obj["version"]
	if !ok {
		return "", fmt.Errorf("no version field in JSON")
	}
	var version string
	if err := json.Unmarshal(raw, &version); err != nil {
		return "", fmt.Errorf("parsing version field: %w", err)
	}
	return version, nil
}

func writeJSONVersion(content, version string) (string, error) {
	// Replace only the first "version" field to avoid touching nested objects
	re := regexp.MustCompile(`("version"\s*:\s*)"[^"]+"`)
	loc := re.FindStringIndex(content)
	if loc == nil {
		return "", fmt.Errorf("no version field found in JSON")
	}
	match := content[loc[0]:loc[1]]
	replacement := re.ReplaceAllString(match, `${1}"`+version+`"`)
	return content[:loc[0]] + replacement + content[loc[1]:], nil
}

func readSvelteFallback(content string) (string, error) {
	re := regexp.MustCompile(`__APP_VERSION__\s*:\s*'([^']+)'`)
	matches := re.FindStringSubmatch(content)
	if matches == nil {
		return "", fmt.Errorf("no __APP_VERSION__ fallback found")
	}
	return matches[1], nil
}

func writeSvelteFallback(content, version string) (string, error) {
	re := regexp.MustCompile(`(__APP_VERSION__\s*:\s*)'[^']+'`)
	if !re.MatchString(content) {
		return "", fmt.Errorf("no __APP_VERSION__ fallback found")
	}
	return re.ReplaceAllString(content, `${1}'`+version+`'`), nil
}
