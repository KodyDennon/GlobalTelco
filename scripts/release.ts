#!/usr/bin/env bun
/**
 * GlobalTelco Release Manager
 *
 * Interactive TUI for semantic version management across three
 * independently versioned components:
 *   - app     → Cargo workspace + web/package.json
 *   - server  → crates/gt-server/Cargo.toml
 *   - desktop → desktop/src-tauri/Cargo.toml + tauri.conf.json
 *
 * Usage:
 *   bun scripts/release.ts            # Interactive TUI
 *   bun scripts/release.ts --dry-run  # Preview without changes
 *
 * Compile to standalone binary:
 *   bun build --compile scripts/release.ts --outfile release
 */

import { execSync } from "child_process";
import { readFileSync, writeFileSync, existsSync } from "fs";
import { resolve, dirname } from "path";

// ── Locate project root ─────────────────────────────────────────────────

const SCRIPT_DIR = dirname(Bun.main);
const ROOT = resolve(SCRIPT_DIR, "..");

function rootPath(...parts: string[]): string {
  return resolve(ROOT, ...parts);
}

// ── CLI Flags ────────────────────────────────────────────────────────────

const DRY_RUN = process.argv.includes("--dry-run");

// ── ANSI Helpers ─────────────────────────────────────────────────────────

const ESC = "\x1b[";
const c = {
  reset: `${ESC}0m`,
  bold: `${ESC}1m`,
  dim: `${ESC}2m`,
  italic: `${ESC}3m`,
  underline: `${ESC}4m`,
  red: `${ESC}31m`,
  green: `${ESC}32m`,
  yellow: `${ESC}33m`,
  blue: `${ESC}34m`,
  magenta: `${ESC}35m`,
  cyan: `${ESC}36m`,
  gray: `${ESC}90m`,
  bgRed: `${ESC}41m`,
  bgGreen: `${ESC}42m`,
  bgBlue: `${ESC}44m`,
  bgMagenta: `${ESC}45m`,
  up: (n: number) => `${ESC}${n}A`,
  clearLine: `${ESC}2K`,
  cursorHide: `${ESC}?25l`,
  cursorShow: `${ESC}?25h`,
};

function print(s: string) {
  process.stdout.write(s);
}
function println(s = "") {
  process.stdout.write(s + "\n");
}

// ── Semver ───────────────────────────────────────────────────────────────

interface SemVer {
  major: number;
  minor: number;
  patch: number;
  pre: string; // e.g. "alpha.1", "beta.2", "rc.1", or ""
}

type BumpType = "major" | "minor" | "patch" | "premajor" | "preminor" | "prepatch" | "prerelease";

function parseSemver(raw: string): SemVer {
  const cleaned = raw.replace(/^v/, "").trim();
  const [main, pre = ""] = cleaned.split("-", 2);
  const parts = main.split(".").map(Number);
  return {
    major: parts[0] ?? 0,
    minor: parts[1] ?? 0,
    patch: parts[2] ?? 0,
    pre,
  };
}

function formatSemver(v: SemVer): string {
  const base = `${v.major}.${v.minor}.${v.patch}`;
  return v.pre ? `${base}-${v.pre}` : base;
}

function bumpSemver(v: SemVer, type: BumpType, preTag = "alpha"): SemVer {
  const next = { ...v };
  switch (type) {
    case "major":
      next.major++;
      next.minor = 0;
      next.patch = 0;
      next.pre = "";
      break;
    case "minor":
      next.minor++;
      next.patch = 0;
      next.pre = "";
      break;
    case "patch":
      next.patch++;
      next.pre = "";
      break;
    case "premajor":
      next.major++;
      next.minor = 0;
      next.patch = 0;
      next.pre = `${preTag}.0`;
      break;
    case "preminor":
      next.minor++;
      next.patch = 0;
      next.pre = `${preTag}.0`;
      break;
    case "prepatch":
      next.patch++;
      next.pre = `${preTag}.0`;
      break;
    case "prerelease":
      if (next.pre) {
        // Increment pre-release number: alpha.0 → alpha.1
        const match = next.pre.match(/^(.+)\.(\d+)$/);
        if (match) {
          next.pre = `${match[1]}.${Number(match[2]) + 1}`;
        } else {
          next.pre = `${next.pre}.1`;
        }
      } else {
        next.patch++;
        next.pre = `${preTag}.0`;
      }
      break;
  }
  return next;
}

// ── Component Definition ─────────────────────────────────────────────────

interface ComponentFile {
  path: string;
  read: () => string;
  write: (version: string) => void;
}

interface Component {
  name: string;
  displayName: string;
  tagPrefix: string;
  color: string;
  files: ComponentFile[];
}

function makeCargoWorkspaceVersion(): ComponentFile {
  const path = rootPath("Cargo.toml");
  return {
    path,
    read() {
      const content = readFileSync(path, "utf-8");
      const match = content.match(/\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]+)"/);
      return match?.[1] ?? "0.0.0";
    },
    write(version) {
      let content = readFileSync(path, "utf-8");
      content = content.replace(
        /(\[workspace\.package\][\s\S]*?version\s*=\s*")[^"]+(")/,
        `$1${version}$2`
      );
      writeFileSync(path, content);
    },
  };
}

function makePackageJson(relPath: string): ComponentFile {
  const path = rootPath(relPath);
  return {
    path,
    read() {
      const pkg = JSON.parse(readFileSync(path, "utf-8"));
      return pkg.version ?? "0.0.0";
    },
    write(version) {
      const pkg = JSON.parse(readFileSync(path, "utf-8"));
      pkg.version = version;
      writeFileSync(path, JSON.stringify(pkg, null, "\t") + "\n");
    },
  };
}

function makeCargoToml(relPath: string): ComponentFile {
  const path = rootPath(relPath);
  return {
    path,
    read() {
      const content = readFileSync(path, "utf-8");
      const match = content.match(/\[package\][\s\S]*?version\s*=\s*"([^"]+)"/);
      return match?.[1] ?? "0.0.0";
    },
    write(version) {
      let content = readFileSync(path, "utf-8");
      content = content.replace(
        /(\[package\][\s\S]*?version\s*=\s*")[^"]+(")/,
        `$1${version}$2`
      );
      writeFileSync(path, content);
    },
  };
}

function makeTauriConf(relPath: string): ComponentFile {
  const path = rootPath(relPath);
  return {
    path,
    read() {
      const conf = JSON.parse(readFileSync(path, "utf-8"));
      return conf.version ?? "0.0.0";
    },
    write(version) {
      const conf = JSON.parse(readFileSync(path, "utf-8"));
      conf.version = version;
      writeFileSync(path, JSON.stringify(conf, null, "  ") + "\n");
    },
  };
}

const COMPONENTS: Component[] = [
  {
    name: "app",
    displayName: "App (Engine + Web)",
    tagPrefix: "app-v",
    color: c.cyan,
    files: [
      makeCargoWorkspaceVersion(),
      makePackageJson("web/package.json"),
    ],
  },
  {
    name: "server",
    displayName: "Server",
    tagPrefix: "server-v",
    color: c.green,
    files: [makeCargoToml("crates/gt-server/Cargo.toml")],
  },
  {
    name: "desktop",
    displayName: "Desktop (Tauri)",
    tagPrefix: "desktop-v",
    color: c.magenta,
    files: [
      makeCargoToml("desktop/src-tauri/Cargo.toml"),
      makeTauriConf("desktop/src-tauri/tauri.conf.json"),
    ],
  },
];

function readComponentVersion(comp: Component): string {
  return comp.files[0].read();
}

function writeComponentVersion(comp: Component, version: string) {
  for (const f of comp.files) {
    f.write(version);
  }
}

// ── Git ──────────────────────────────────────────────────────────────────

function exec(cmd: string, opts?: { cwd?: string; silent?: boolean }): string {
  try {
    return execSync(cmd, {
      cwd: opts?.cwd ?? ROOT,
      encoding: "utf-8",
      stdio: opts?.silent ? "pipe" : ["pipe", "pipe", "pipe"],
    }).trim();
  } catch (e: any) {
    if (opts?.silent) return "";
    throw new Error(`Command failed: ${cmd}\n${e.stderr || e.message}`);
  }
}

function gitIsDirty(): boolean {
  return exec("git status --porcelain", { silent: true }).length > 0;
}

function gitCurrentBranch(): string {
  return exec("git rev-parse --abbrev-ref HEAD", { silent: true });
}

function gitTagsForPrefix(prefix: string): string[] {
  const output = exec(`git tag -l "${prefix}*" --sort=-v:refname`, { silent: true });
  return output ? output.split("\n").filter(Boolean) : [];
}

function gitAllTags(): string[] {
  const output = exec("git tag -l --sort=-v:refname", { silent: true });
  return output ? output.split("\n").filter(Boolean) : [];
}

function gitLatestTagForPrefix(prefix: string): string | null {
  const tags = gitTagsForPrefix(prefix);
  return tags[0] ?? null;
}

function gitLogBetween(from: string | null, to = "HEAD"): string[] {
  const range = from ? `${from}..${to}` : to;
  const output = exec(`git log ${range} --pretty=format:"%s" --no-merges`, { silent: true });
  return output ? output.split("\n").filter(Boolean) : [];
}

function gitCreateTag(tag: string, message: string) {
  if (DRY_RUN) {
    println(`  ${c.yellow}[dry-run]${c.reset} Would create tag: ${c.bold}${tag}${c.reset}`);
    return;
  }
  exec(`git tag -a "${tag}" -m "${message}"`);
}

function gitDeleteTag(tag: string) {
  if (DRY_RUN) {
    println(`  ${c.yellow}[dry-run]${c.reset} Would delete tag: ${c.bold}${tag}${c.reset}`);
    return;
  }
  exec(`git tag -d "${tag}"`, { silent: true });
}

function gitDeleteRemoteTag(tag: string) {
  if (DRY_RUN) {
    println(`  ${c.yellow}[dry-run]${c.reset} Would delete remote tag: ${c.bold}${tag}${c.reset}`);
    return;
  }
  exec(`git push origin :refs/tags/${tag}`, { silent: true });
}

function gitPushWithTags() {
  if (DRY_RUN) {
    println(`  ${c.yellow}[dry-run]${c.reset} Would push commits and tags`);
    return;
  }
  exec("git push && git push --tags");
}

function gitCommit(message: string, files: string[]) {
  if (DRY_RUN) {
    println(`  ${c.yellow}[dry-run]${c.reset} Would commit: ${message}`);
    return;
  }
  for (const f of files) {
    exec(`git add "${f}"`);
  }
  // Use heredoc-style commit to handle special chars
  execSync(`git commit -m "${message.replace(/"/g, '\\"')}"`, {
    cwd: ROOT,
    stdio: "pipe",
  });
}

// ── Changelog ────────────────────────────────────────────────────────────

interface ChangelogSection {
  title: string;
  prefix: string;
  entries: string[];
}

function categorizeCommits(commits: string[]): ChangelogSection[] {
  const sections: ChangelogSection[] = [
    { title: "Features", prefix: "feat", entries: [] },
    { title: "Bug Fixes", prefix: "fix", entries: [] },
    { title: "Performance", prefix: "perf", entries: [] },
    { title: "Refactoring", prefix: "refactor", entries: [] },
    { title: "Documentation", prefix: "docs", entries: [] },
    { title: "Other Changes", prefix: "", entries: [] },
  ];

  for (const msg of commits) {
    let placed = false;
    for (const section of sections) {
      if (section.prefix && msg.toLowerCase().startsWith(`${section.prefix}:`)) {
        section.entries.push(msg);
        placed = true;
        break;
      }
      if (section.prefix && msg.toLowerCase().startsWith(`${section.prefix}(`)) {
        section.entries.push(msg);
        placed = true;
        break;
      }
    }
    if (!placed) {
      sections[sections.length - 1].entries.push(msg);
    }
  }

  return sections.filter((s) => s.entries.length > 0);
}

function generateChangelogEntry(
  tag: string,
  date: string,
  commits: string[]
): string {
  const sections = categorizeCommits(commits);
  let md = `## [${tag}] - ${date}\n\n`;
  for (const section of sections) {
    md += `### ${section.title}\n\n`;
    for (const entry of section.entries) {
      md += `- ${entry}\n`;
    }
    md += "\n";
  }
  return md;
}

function updateChangelogFile(entry: string) {
  const path = rootPath("CHANGELOG.md");
  if (DRY_RUN) {
    println(`  ${c.yellow}[dry-run]${c.reset} Would update CHANGELOG.md`);
    return;
  }
  let content: string;
  if (existsSync(path)) {
    content = readFileSync(path, "utf-8");
    // Insert after the header
    const headerEnd = content.indexOf("\n## ");
    if (headerEnd >= 0) {
      content = content.slice(0, headerEnd) + "\n" + entry + content.slice(headerEnd);
    } else {
      content += "\n" + entry;
    }
  } else {
    content = `# Changelog\n\nAll notable changes to GlobalTelco.\n\n${entry}`;
  }
  writeFileSync(path, content);
}

// ── TUI Primitives ───────────────────────────────────────────────────────

async function select(prompt: string, options: string[]): Promise<number> {
  const isTTY = process.stdin.isTTY;

  // Fallback for non-interactive (piped) environments
  if (!isTTY) {
    println(`${c.bold}${c.blue}?${c.reset} ${c.bold}${prompt}${c.reset}`);
    for (let i = 0; i < options.length; i++) {
      println(`  ${c.cyan}${i + 1})${c.reset} ${options[i]}`);
    }
    return new Promise<number>((resolve) => {
      const rl = require("readline").createInterface({ input: process.stdin, output: process.stdout });
      rl.question(`${c.dim}  Enter number: ${c.reset}`, (ans: string) => {
        rl.close();
        const n = parseInt(ans, 10) - 1;
        resolve(n >= 0 && n < options.length ? n : 0);
      });
    });
  }

  return new Promise<number>((resolve) => {
    let cursor = 0;
    const total = options.length;

    function render(initial = false) {
      if (!initial) {
        print(c.up(total));
      }
      for (let i = 0; i < total; i++) {
        print(c.clearLine);
        if (i === cursor) {
          println(`  ${c.cyan}>${c.reset} ${c.bold}${options[i]}${c.reset}`);
        } else {
          println(`    ${c.dim}${options[i]}${c.reset}`);
        }
      }
    }

    println(`${c.bold}${c.blue}?${c.reset} ${c.bold}${prompt}${c.reset}`);
    print(c.cursorHide);
    render(true);

    process.stdin.setRawMode(true);
    process.stdin.resume();
    process.stdin.setEncoding("utf-8");

    const handler = (key: string) => {
      if (key === "\x1b[A" || key === "k") {
        cursor = (cursor - 1 + total) % total;
        render();
      } else if (key === "\x1b[B" || key === "j") {
        cursor = (cursor + 1) % total;
        render();
      } else if (key === "\r" || key === "\n") {
        process.stdin.setRawMode(false);
        process.stdin.pause();
        process.stdin.removeListener("data", handler);
        print(c.cursorShow);
        // Show final selection
        print(c.up(total));
        for (let i = 0; i < total; i++) {
          print(c.clearLine);
          if (i === cursor) {
            println(`  ${c.green}>${c.reset} ${c.bold}${options[i]}${c.reset}`);
          } else {
            println();
          }
        }
        print(c.up(total));
        for (let i = 0; i < total; i++) {
          if (i !== cursor) print(c.clearLine);
          println();
        }
        resolve(cursor);
      } else if (key === "\x03" || key === "q") {
        print(c.cursorShow);
        process.stdin.setRawMode(false);
        process.exit(0);
      }
    };

    process.stdin.on("data", handler);
  });
}

async function confirm(prompt: string, defaultYes = true): Promise<boolean> {
  return new Promise<boolean>((resolve) => {
    const hint = defaultYes ? "Y/n" : "y/N";
    print(`${c.bold}${c.blue}?${c.reset} ${c.bold}${prompt}${c.reset} ${c.dim}(${hint})${c.reset} `);

    process.stdin.setRawMode(true);
    process.stdin.resume();
    process.stdin.setEncoding("utf-8");

    const handler = (key: string) => {
      process.stdin.setRawMode(false);
      process.stdin.pause();
      process.stdin.removeListener("data", handler);

      if (key === "\x03") process.exit(0);
      if (key === "\r" || key === "\n") {
        println(defaultYes ? "Yes" : "No");
        resolve(defaultYes);
      } else if (key.toLowerCase() === "y") {
        println("Yes");
        resolve(true);
      } else {
        println("No");
        resolve(false);
      }
    };

    process.stdin.on("data", handler);
  });
}

async function input(prompt: string, defaultValue = ""): Promise<string> {
  return new Promise<string>((resolve) => {
    const rl = require("readline").createInterface({
      input: process.stdin,
      output: process.stdout,
    });
    const hint = defaultValue ? ` ${c.dim}(${defaultValue})${c.reset}` : "";
    rl.question(`${c.bold}${c.blue}?${c.reset} ${c.bold}${prompt}${c.reset}${hint} `, (answer: string) => {
      rl.close();
      resolve(answer.trim() || defaultValue);
    });
  });
}

// ── Display ──────────────────────────────────────────────────────────────

function printHeader() {
  println();
  println(`${c.bold}${c.cyan}  ╔═══════════════════════════════════════════╗${c.reset}`);
  println(`${c.bold}${c.cyan}  ║${c.reset}${c.bold}     GlobalTelco Release Manager          ${c.cyan}║${c.reset}`);
  println(`${c.bold}${c.cyan}  ╚═══════════════════════════════════════════╝${c.reset}`);
  if (DRY_RUN) {
    println(`${c.yellow}  ⚠  DRY RUN MODE — no changes will be made${c.reset}`);
  }
  println();
}

function printVersions() {
  println(`${c.bold}  Current Versions:${c.reset}`);
  for (const comp of COMPONENTS) {
    const ver = readComponentVersion(comp);
    const latestTag = gitLatestTagForPrefix(comp.tagPrefix);
    const tagInfo = latestTag ? `${c.dim}(latest tag: ${latestTag})${c.reset}` : `${c.dim}(no tags)${c.reset}`;
    println(`    ${comp.color}${comp.displayName}${c.reset}  ${c.bold}${ver}${c.reset}  ${tagInfo}`);
  }
  println();
}

function printDivider() {
  println(`${c.dim}  ${"─".repeat(43)}${c.reset}`);
}

// ── Flows ────────────────────────────────────────────────────────────────

async function releaseFlow() {
  println();
  println(`${c.bold}  Release a Component${c.reset}`);
  printDivider();

  // 1. Pick component
  const compIdx = await select("Which component?", COMPONENTS.map((c) => c.displayName));
  const comp = COMPONENTS[compIdx];
  const currentVer = readComponentVersion(comp);

  println();
  println(`  ${comp.color}${comp.displayName}${c.reset} is currently at ${c.bold}v${currentVer}${c.reset}`);
  println();

  // 2. Pick bump type
  const parsed = parseSemver(currentVer);
  const bumpTypes: BumpType[] = ["patch", "minor", "major"];
  const bumpLabels = bumpTypes.map((t) => {
    const next = formatSemver(bumpSemver(parsed, t));
    return `${t.padEnd(8)} → ${c.bold}v${next}${c.reset}`;
  });
  const bumpIdx = await select("Release type?", bumpLabels);
  const bumpType = bumpTypes[bumpIdx];
  const nextVer = bumpSemver(parsed, bumpType);
  const nextVerStr = formatSemver(nextVer);
  const tag = `${comp.tagPrefix}${nextVerStr}`;

  println();
  println(`  ${c.bold}${comp.color}${comp.displayName}${c.reset}: ${c.dim}v${currentVer}${c.reset} → ${c.bold}${c.green}v${nextVerStr}${c.reset}`);
  println(`  ${c.bold}Tag:${c.reset} ${tag}`);

  // 3. Generate changelog preview
  const latestTag = gitLatestTagForPrefix(comp.tagPrefix);
  const commits = gitLogBetween(latestTag);

  if (commits.length > 0) {
    println();
    println(`  ${c.bold}Changes since ${latestTag ?? "beginning"}:${c.reset}`);
    for (const msg of commits.slice(0, 15)) {
      println(`    ${c.dim}•${c.reset} ${msg}`);
    }
    if (commits.length > 15) {
      println(`    ${c.dim}... and ${commits.length - 15} more${c.reset}`);
    }
  }

  println();

  // 4. Confirm
  const ok = await confirm("Proceed with release?");
  if (!ok) {
    println(`  ${c.yellow}Aborted.${c.reset}`);
    return;
  }

  // 5. Execute
  println();
  const date = new Date().toISOString().split("T")[0];

  // Update version files
  print(`  ${c.dim}Bumping version...${c.reset}`);
  writeComponentVersion(comp, nextVerStr);
  println(` ${c.green}done${c.reset}`);

  // Generate changelog
  if (commits.length > 0) {
    print(`  ${c.dim}Updating CHANGELOG.md...${c.reset}`);
    const entry = generateChangelogEntry(tag, date, commits);
    updateChangelogFile(entry);
    println(` ${c.green}done${c.reset}`);
  }

  // Commit
  print(`  ${c.dim}Committing...${c.reset}`);
  const allFiles = comp.files.map((f) => f.path);
  const changelogPath = rootPath("CHANGELOG.md");
  if (existsSync(changelogPath)) allFiles.push(changelogPath);
  gitCommit(`release: ${comp.name} v${nextVerStr}`, allFiles);
  println(` ${c.green}done${c.reset}`);

  // Tag
  print(`  ${c.dim}Creating tag ${tag}...${c.reset}`);
  gitCreateTag(tag, `Release ${comp.displayName} v${nextVerStr}`);
  println(` ${c.green}done${c.reset}`);

  // Push
  const push = await confirm("Push to remote?");
  if (push) {
    print(`  ${c.dim}Pushing...${c.reset}`);
    gitPushWithTags();
    println(` ${c.green}done${c.reset}`);
  }

  println();
  println(`  ${c.green}${c.bold}Release ${tag} complete!${c.reset}`);
  println();
}

async function bumpFlow() {
  println();
  println(`${c.bold}  Bump Version (no tag/push)${c.reset}`);
  printDivider();

  const compIdx = await select("Which component?", COMPONENTS.map((c) => c.displayName));
  const comp = COMPONENTS[compIdx];
  const currentVer = readComponentVersion(comp);
  const parsed = parseSemver(currentVer);

  println();
  println(`  ${comp.color}${comp.displayName}${c.reset} is currently at ${c.bold}v${currentVer}${c.reset}`);
  println();

  const bumpTypes: BumpType[] = ["patch", "minor", "major"];
  const bumpLabels = bumpTypes.map((t) => {
    const next = formatSemver(bumpSemver(parsed, t));
    return `${t.padEnd(8)} → v${next}`;
  });
  const bumpIdx = await select("Bump type?", bumpLabels);
  const nextVer = formatSemver(bumpSemver(parsed, bumpTypes[bumpIdx]));

  println();
  const ok = await confirm(`Bump ${comp.displayName} to v${nextVer}?`);
  if (!ok) {
    println(`  ${c.yellow}Aborted.${c.reset}`);
    return;
  }

  writeComponentVersion(comp, nextVer);
  println(`  ${c.green}Version bumped to v${nextVer}${c.reset}`);
  println(`  ${c.dim}Files updated — commit when ready.${c.reset}`);
  println();
}

async function prereleaseFlow() {
  println();
  println(`${c.bold}  Pre-release Version${c.reset}`);
  printDivider();

  const compIdx = await select("Which component?", COMPONENTS.map((c) => c.displayName));
  const comp = COMPONENTS[compIdx];
  const currentVer = readComponentVersion(comp);
  const parsed = parseSemver(currentVer);

  println();
  println(`  ${comp.color}${comp.displayName}${c.reset} is currently at ${c.bold}v${currentVer}${c.reset}`);
  println();

  // Pre-release tag
  const preTagIdx = await select("Pre-release type?", ["alpha", "beta", "rc"]);
  const preTag = ["alpha", "beta", "rc"][preTagIdx];

  // Bump type for pre-release
  let bumpTypes: BumpType[];
  let bumpLabels: string[];

  if (parsed.pre) {
    // Already a pre-release — offer increment or promote
    const nextPre = formatSemver(bumpSemver(parsed, "prerelease", preTag));
    const promoted = formatSemver({ ...parsed, pre: "" });
    bumpTypes = ["prerelease", "patch"];
    bumpLabels = [
      `Increment pre-release → v${nextPre}`,
      `Promote to stable    → v${promoted}`,
    ];
  } else {
    bumpTypes = ["prepatch", "preminor", "premajor"];
    bumpLabels = bumpTypes.map((t) => {
      const next = formatSemver(bumpSemver(parsed, t, preTag));
      return `${t.padEnd(10)} → v${next}`;
    });
  }

  const bumpIdx = await select("Version bump?", bumpLabels);
  const nextVer = formatSemver(bumpSemver(parsed, bumpTypes[bumpIdx], preTag));
  const tag = `${comp.tagPrefix}${nextVer}`;

  println();
  println(`  ${c.bold}${comp.color}${comp.displayName}${c.reset}: v${currentVer} → ${c.bold}v${nextVer}${c.reset}`);
  println(`  ${c.bold}Tag:${c.reset} ${tag}`);
  println();

  const ok = await confirm("Proceed?");
  if (!ok) {
    println(`  ${c.yellow}Aborted.${c.reset}`);
    return;
  }

  writeComponentVersion(comp, nextVer);
  gitCommit(`release: ${comp.name} v${nextVer} (pre-release)`, comp.files.map((f) => f.path));
  gitCreateTag(tag, `Pre-release ${comp.displayName} v${nextVer}`);

  const push = await confirm("Push to remote?");
  if (push) {
    gitPushWithTags();
  }

  println(`  ${c.green}${c.bold}Pre-release ${tag} created!${c.reset}`);
  println();
}

async function listReleases() {
  println();
  println(`${c.bold}  All Releases${c.reset}`);
  printDivider();

  for (const comp of COMPONENTS) {
    const tags = gitTagsForPrefix(comp.tagPrefix);
    println();
    println(`  ${comp.color}${c.bold}${comp.displayName}${c.reset} ${c.dim}(prefix: ${comp.tagPrefix})${c.reset}`);
    if (tags.length === 0) {
      println(`    ${c.dim}No releases yet${c.reset}`);
    } else {
      for (const tag of tags.slice(0, 10)) {
        // Get tag date
        const date = exec(`git log -1 --format="%ai" ${tag} 2>/dev/null || echo "unknown"`, { silent: true });
        const shortDate = date.split(" ")[0] ?? "";
        const isLatest = tag === tags[0];
        const marker = isLatest ? `${c.green} (latest)${c.reset}` : "";
        println(`    ${c.bold}${tag}${c.reset}  ${c.dim}${shortDate}${c.reset}${marker}`);
      }
      if (tags.length > 10) {
        println(`    ${c.dim}... and ${tags.length - 10} more${c.reset}`);
      }
    }
  }

  // Also show untagged version tags (v*)
  const allTags = gitAllTags().filter((t) => {
    return !COMPONENTS.some((c) => t.startsWith(c.tagPrefix));
  });
  if (allTags.length > 0) {
    println();
    println(`  ${c.dim}${c.bold}Other Tags${c.reset}`);
    for (const tag of allTags.slice(0, 5)) {
      println(`    ${tag}`);
    }
  }

  println();
}

async function validateFlow() {
  println();
  println(`${c.bold}  Validation${c.reset}`);
  printDivider();
  println();

  let issues = 0;

  // Check git status
  const dirty = gitIsDirty();
  if (dirty) {
    println(`  ${c.red}✗${c.reset} Working tree has uncommitted changes`);
    issues++;
  } else {
    println(`  ${c.green}✓${c.reset} Working tree is clean`);
  }

  // Check branch
  const branch = gitCurrentBranch();
  println(`  ${c.blue}i${c.reset} Current branch: ${c.bold}${branch}${c.reset}`);

  // Check component version consistency
  for (const comp of COMPONENTS) {
    const versions = comp.files.map((f) => ({ path: f.path, ver: f.read() }));
    const allSame = versions.every((v) => v.ver === versions[0].ver);
    if (allSame) {
      println(`  ${c.green}✓${c.reset} ${comp.displayName}: all files at ${c.bold}v${versions[0].ver}${c.reset}`);
    } else {
      println(`  ${c.red}✗${c.reset} ${comp.displayName}: version mismatch!`);
      for (const v of versions) {
        const rel = v.path.replace(ROOT + "/", "");
        println(`      ${rel}: ${c.bold}${v.ver}${c.reset}`);
      }
      issues++;
    }

    // Check if current version has a tag
    const tag = `${comp.tagPrefix}${versions[0].ver}`;
    const tagExists = gitTagsForPrefix(comp.tagPrefix).includes(tag);
    if (tagExists) {
      println(`  ${c.green}✓${c.reset} Tag ${c.bold}${tag}${c.reset} exists`);
    } else {
      println(`  ${c.yellow}!${c.reset} Tag ${c.bold}${tag}${c.reset} does not exist ${c.dim}(unreleased version)${c.reset}`);
    }
  }

  // Check remote sync
  try {
    const local = exec("git rev-parse HEAD", { silent: true });
    const remote = exec("git rev-parse @{u}", { silent: true });
    if (local === remote) {
      println(`  ${c.green}✓${c.reset} Local matches remote`);
    } else {
      println(`  ${c.yellow}!${c.reset} Local and remote are out of sync`);
    }
  } catch {
    println(`  ${c.dim}-${c.reset} No upstream tracking branch`);
  }

  println();
  if (issues === 0) {
    println(`  ${c.green}${c.bold}All checks passed!${c.reset}`);
  } else {
    println(`  ${c.yellow}${c.bold}${issues} issue(s) found${c.reset}`);
  }
  println();
}

async function rollbackFlow() {
  println();
  println(`${c.bold}  Rollback (Delete Tag)${c.reset}`);
  printDivider();

  const compIdx = await select("Which component?", COMPONENTS.map((c) => c.displayName));
  const comp = COMPONENTS[compIdx];
  const tags = gitTagsForPrefix(comp.tagPrefix);

  if (tags.length === 0) {
    println(`  ${c.dim}No tags to rollback.${c.reset}`);
    println();
    return;
  }

  println();
  const tagIdx = await select("Delete which tag?", tags.slice(0, 10));
  const tag = tags[tagIdx];

  println();
  println(`  ${c.red}${c.bold}WARNING:${c.reset} This will delete tag ${c.bold}${tag}${c.reset}`);
  const ok = await confirm("Are you sure?", false);
  if (!ok) {
    println(`  ${c.yellow}Aborted.${c.reset}`);
    return;
  }

  print(`  ${c.dim}Deleting local tag...${c.reset}`);
  gitDeleteTag(tag);
  println(` ${c.green}done${c.reset}`);

  const deleteRemote = await confirm("Also delete from remote?", false);
  if (deleteRemote) {
    print(`  ${c.dim}Deleting remote tag...${c.reset}`);
    gitDeleteRemoteTag(tag);
    println(` ${c.green}done${c.reset}`);
  }

  println(`  ${c.green}Tag ${tag} removed.${c.reset}`);
  println();
}

// ── Main ─────────────────────────────────────────────────────────────────

async function main() {
  printHeader();
  printVersions();

  while (true) {
    printDivider();
    println();
    const choice = await select("What would you like to do?", [
      `${c.green}Release${c.reset}         Bump + changelog + tag + push`,
      `${c.cyan}Bump Version${c.reset}    Update version files only`,
      `${c.magenta}Pre-release${c.reset}     Create alpha/beta/rc version`,
      `${c.blue}List Releases${c.reset}   Show all tags by component`,
      `${c.yellow}Validate${c.reset}        Check version consistency`,
      `${c.red}Rollback${c.reset}        Delete a release tag`,
      `${c.dim}Exit${c.reset}`,
    ]);

    switch (choice) {
      case 0:
        await releaseFlow();
        break;
      case 1:
        await bumpFlow();
        break;
      case 2:
        await prereleaseFlow();
        break;
      case 3:
        await listReleases();
        break;
      case 4:
        await validateFlow();
        break;
      case 5:
        await rollbackFlow();
        break;
      case 6:
        println(`\n  ${c.dim}Bye!${c.reset}\n`);
        process.exit(0);
    }

    // Refresh versions display after any action
    println();
    printVersions();
  }
}

main().catch((e) => {
  console.error(`${c.red}Error:${c.reset}`, e.message);
  process.exit(1);
});
