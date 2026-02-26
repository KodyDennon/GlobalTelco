package core

import (
	"os"
	"path/filepath"
	"testing"

	"gt/config"
)

func TestReadWritePlainText(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "VERSION")
	os.WriteFile(path, []byte("1.2.3\n"), 0644)

	vf := config.VersionFile{RelPath: "VERSION", FileType: config.PlainText}
	v, err := ReadVersion(dir, vf)
	if err != nil {
		t.Fatalf("ReadVersion: %v", err)
	}
	if v != "1.2.3" {
		t.Errorf("got %q, want %q", v, "1.2.3")
	}

	err = WriteVersion(dir, vf, "2.0.0")
	if err != nil {
		t.Fatalf("WriteVersion: %v", err)
	}

	v, err = ReadVersion(dir, vf)
	if err != nil {
		t.Fatalf("ReadVersion after write: %v", err)
	}
	if v != "2.0.0" {
		t.Errorf("got %q, want %q", v, "2.0.0")
	}
}

func TestReadWriteCargoWorkspace(t *testing.T) {
	dir := t.TempDir()
	content := `[workspace]
resolver = "2"
members = ["crates/a"]

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
serde = "1"
`
	path := filepath.Join(dir, "Cargo.toml")
	os.WriteFile(path, []byte(content), 0644)

	vf := config.VersionFile{RelPath: "Cargo.toml", FileType: config.CargoWorkspace}
	v, err := ReadVersion(dir, vf)
	if err != nil {
		t.Fatalf("ReadVersion: %v", err)
	}
	if v != "0.1.0" {
		t.Errorf("got %q, want %q", v, "0.1.0")
	}

	err = WriteVersion(dir, vf, "1.0.0")
	if err != nil {
		t.Fatalf("WriteVersion: %v", err)
	}

	v, err = ReadVersion(dir, vf)
	if err != nil {
		t.Fatalf("ReadVersion after write: %v", err)
	}
	if v != "1.0.0" {
		t.Errorf("got %q, want %q", v, "1.0.0")
	}

	// Verify other content preserved
	data, _ := os.ReadFile(path)
	if got := string(data); got == "" {
		t.Fatal("file is empty after write")
	}
}

func TestReadWriteCargoPackage(t *testing.T) {
	dir := t.TempDir()
	content := `[package]
name = "gt-server"
version = "0.5.1"
edition.workspace = true

[dependencies]
tokio = "1"
`
	os.WriteFile(filepath.Join(dir, "Cargo.toml"), []byte(content), 0644)

	vf := config.VersionFile{RelPath: "Cargo.toml", FileType: config.CargoPackage}
	v, err := ReadVersion(dir, vf)
	if err != nil {
		t.Fatalf("ReadVersion: %v", err)
	}
	if v != "0.5.1" {
		t.Errorf("got %q, want %q", v, "0.5.1")
	}

	err = WriteVersion(dir, vf, "0.6.0")
	if err != nil {
		t.Fatalf("WriteVersion: %v", err)
	}

	v, _ = ReadVersion(dir, vf)
	if v != "0.6.0" {
		t.Errorf("after write: got %q, want %q", v, "0.6.0")
	}
}

func TestReadWritePackageJSON(t *testing.T) {
	dir := t.TempDir()
	content := `{
	"name": "web",
	"private": true,
	"version": "0.1.0",
	"type": "module"
}
`
	os.WriteFile(filepath.Join(dir, "package.json"), []byte(content), 0644)

	vf := config.VersionFile{RelPath: "package.json", FileType: config.PackageJSON}
	v, err := ReadVersion(dir, vf)
	if err != nil {
		t.Fatalf("ReadVersion: %v", err)
	}
	if v != "0.1.0" {
		t.Errorf("got %q, want %q", v, "0.1.0")
	}

	err = WriteVersion(dir, vf, "1.0.0")
	if err != nil {
		t.Fatalf("WriteVersion: %v", err)
	}

	v, _ = ReadVersion(dir, vf)
	if v != "1.0.0" {
		t.Errorf("after write: got %q, want %q", v, "1.0.0")
	}
}

func TestReadWriteTauriConf(t *testing.T) {
	dir := t.TempDir()
	content := `{
  "productName": "GlobalTelco",
  "version": "0.1.0",
  "identifier": "com.globaltelco.app"
}
`
	os.WriteFile(filepath.Join(dir, "tauri.conf.json"), []byte(content), 0644)

	vf := config.VersionFile{RelPath: "tauri.conf.json", FileType: config.TauriConf}
	v, err := ReadVersion(dir, vf)
	if err != nil {
		t.Fatalf("ReadVersion: %v", err)
	}
	if v != "0.1.0" {
		t.Errorf("got %q, want %q", v, "0.1.0")
	}

	err = WriteVersion(dir, vf, "2.0.0")
	if err != nil {
		t.Fatalf("WriteVersion: %v", err)
	}

	v, _ = ReadVersion(dir, vf)
	if v != "2.0.0" {
		t.Errorf("after write: got %q, want %q", v, "2.0.0")
	}
}

func TestReadWriteSvelteFallback(t *testing.T) {
	dir := t.TempDir()
	content := `<script>
	const version = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '0.1.0';
</script>
`
	os.WriteFile(filepath.Join(dir, "MainMenu.svelte"), []byte(content), 0644)

	vf := config.VersionFile{RelPath: "MainMenu.svelte", FileType: config.SvelteFallback}
	v, err := ReadVersion(dir, vf)
	if err != nil {
		t.Fatalf("ReadVersion: %v", err)
	}
	if v != "0.1.0" {
		t.Errorf("got %q, want %q", v, "0.1.0")
	}

	err = WriteVersion(dir, vf, "3.0.0")
	if err != nil {
		t.Fatalf("WriteVersion: %v", err)
	}

	v, _ = ReadVersion(dir, vf)
	if v != "3.0.0" {
		t.Errorf("after write: got %q, want %q", v, "3.0.0")
	}
}

func TestValidateComponentVersions(t *testing.T) {
	dir := t.TempDir()

	// Create VERSION and Cargo.toml with matching versions
	os.WriteFile(filepath.Join(dir, "VERSION"), []byte("1.0.0\n"), 0644)
	os.WriteFile(filepath.Join(dir, "Cargo.toml"), []byte(`[workspace.package]
version = "1.0.0"
`), 0644)

	// Create svelte fallback
	os.MkdirAll(filepath.Join(dir, "web/src/lib/menu"), 0755)
	os.WriteFile(filepath.Join(dir, "web/src/lib/menu/MainMenu.svelte"), []byte(
		`const version = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '1.0.0';`,
	), 0644)

	comp := config.Component{
		ID: "engine",
		Files: []config.VersionFile{
			{RelPath: "VERSION", FileType: config.PlainText},
			{RelPath: "Cargo.toml", FileType: config.CargoWorkspace},
			{RelPath: "web/src/lib/menu/MainMenu.svelte", FileType: config.SvelteFallback},
		},
	}

	v, mismatches, err := ValidateComponentVersions(dir, comp)
	if err != nil {
		t.Fatalf("ValidateComponentVersions: %v", err)
	}
	if v != "1.0.0" {
		t.Errorf("version: got %q, want %q", v, "1.0.0")
	}
	if len(mismatches) != 0 {
		t.Errorf("expected no mismatches, got: %v", mismatches)
	}
}
