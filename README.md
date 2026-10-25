<div align="center">
  <img src="src-tauri/icons/128x128.png" alt="Forge Desktop Logo" width="96" height="96" />
  <h1>Forge Desktop</h1>
  <p><strong>A high-performance native Git desktop client built with Rust, Tauri 2, and Svelte 5.</strong></p>
  <p>100% feature parity with GitHub Desktop, designed with minimal resource overhead, instant startup, and complete local privacy.</p>
</div>

---

## Overview

Forge Desktop is a cross-platform desktop Git client engineered to replace heavy web-wrapper Git applications with a lean, compiled native architecture. By combining a Rust core for deterministic Git execution and a reactive Svelte 5 frontend using custom CSS design tokens, Forge Desktop provides instant responsiveness, low memory footprint, and full compatibility with Git workflows.

> [!NOTE]
> Forge Desktop achieves full feature parity with GitHub Desktop (v3.4.9), supporting complex workflows including partial hunk staging, visual interactive rebase, multi-mode image diffing, and GitHub Actions CI status checks.

---

## Features

### Repository Management
- **Repository Operations**: Open existing local repositories, clone via HTTPS or SSH, and initialize new repositories with customizable `.gitignore` templates and licenses.
- **GitHub Publishing**: Publish local repositories directly to GitHub with public or private visibility settings.
- **Repository Tracking**: Automatic tracking of recent repositories, path relocation for moved folders, and custom repository aliases.

### Changes and Staging
- **Real-Time Working Tree**: Instant detection of modified, added, deleted, renamed, and untracked files.
- **Granular Staging**: File-level staging checkboxes, batch stage/unstage actions, and **partial hunk staging** directly within diff views.
- **Safety and Discard**: Selective discard of individual file changes or full working tree discard with configurable confirmation dialogs.

### Diff Viewers
- **Unified and Split Modes**: Switch between unified and side-by-side split diff layouts with line-level addition and deletion indicators.
- **Syntax Highlighting**: Built-in syntax highlighting across major programming languages (Rust, TypeScript, JavaScript, Python, CSS, JSON, TOML).
- **Image Diff Inspection**: Compare image changes using three specialized modes:
  - **2-Up**: Side-by-side comparison of baseline and modified graphics.
  - **Swipe Bar**: Interactive horizontal slider divider revealing changes dynamically.
  - **Onion Skin**: Adjustable opacity overlay (0-100%) to spot subtle graphic revisions.
- **Rendered Markdown Diff**: Native rendering of Markdown diffs with styled headings, task lists, code blocks, and change markers.
- **Whitespace Control**: Toggle whitespace changes (`--ignore-all-space`) to focus on substantive code modifications.

### Commit Workflows
- **Rich Commit Authoring**: Commit summaries, multi-line descriptions, and co-author attribution via `Co-authored-by:` trailers.
- **Productivity Accelerators**: One-click conventional commit prefixes (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`), inline emoji autocompletion (`:`), and GitHub issue autocompletion (`#`).
- **History Mutations**: Undo recent commits (`git reset --soft HEAD~1`) and amend the latest commit safely.

### Commit History and Advanced Git Operations
- **History Timeline**: Searchable commit history displaying SHAs, authors, dates, and verification status.
- **Visual Interactive Rebase**: Interactive rebase modal to reorder commits using vertical controls or drop unwanted commits before applying changes.
- **Branch Divergence**: Live ahead and behind commit indicators comparing the current branch against upstream or target references.
- **Advanced Operations**: Cherry-pick commits, revert commits (`git revert`), squash multiple commits into one, and manage Git tags (create and delete).

### Branches, Remotes, and Collaboration
- **Branch Management**: Filter, create, rename, delete, and merge branches with conflict detection.
- **Remote Synchronization**: Fetch, pull, push with `--force-with-lease`, sync forks directly against upstream remotes, and automated periodic background fetch.
- **Merge Conflict Recovery**: Visual indicators for merge conflicts, one-click merge abort, and direct shortcuts to resolve conflicts in external editors.
- **GitHub Integration**: GitHub OAuth device flow authentication, pull requests and issues browser, branch checkout from pull requests, and real-time GitHub Actions CI status indicators.

---

## Architecture

Forge Desktop is architected with a strict separation between low-level system operations and reactive presentation.

```
+-------------------------------------------------------------+
|                      Svelte 5 Frontend                      |
|  - Custom CSS Design Tokens (Dark / Light / System)         |
|  - Reactive State Management (Runes: $state, $derived)      |
|  - Diff Parsers (Unified, Split, Image, Markdown)           |
+-------------------------------------------------------------+
                              |
                     Tauri 2 IPC Bridge
                              |
+-------------------------------------------------------------+
|                      Rust Core Engine                       |
|  - Safe Git CLI Subprocess Execution                        |
|  - Native Base64 Image Processing                           |
|  - GitHub REST API Client (Device Flow, Releases, Checks)   |
|  - System Diagnostics with Credential Redaction             |
+-------------------------------------------------------------+
```

---

## Getting Started

### Prerequisites
- **Node.js**: Version 18.0.0 or higher
- **npm** or **pnpm**
- **Rust and Cargo**: Version 1.80.0 or higher
- **Git**: Version 2.30.0 or higher installed in system PATH

### Installation
Clone the repository and install dependencies:

```bash
git clone https://github.com/machfudin/github-desktop.git
cd github-desktop
npm install
```

### Development
Launch the development server with live reload and Tauri desktop runtime:

```bash
npm run tauri dev
```

Run frontend type checking and linter:

```bash
npm run build
```

Run backend unit and integration tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## CLI Launcher

Forge Desktop includes a standalone CLI launcher located at `scripts/forge`. You can launch repositories directly from your terminal:

```bash
# Open current working directory
./scripts/forge .

# Open a specific repository path
./scripts/forge /path/to/my-project
```

> [!TIP]
> Symlink `scripts/forge` into your `$PATH` (e.g. `/usr/local/bin/forge`) to launch Forge Desktop globally from any terminal directory.

---

## Packaging and Release Signing

Forge Desktop supports multi-OS release builds with native signing infrastructure:

- **Linux**: `.deb`, `.rpm`, and `.AppImage` packages.
- **Windows**: `.msi` and `.exe` installers.
- **macOS**: `.dmg` and `.app` bundles.

Artifacts can be built locally using:

```bash
npm run tauri build
```

Release signing is handled via GPG for Linux artifacts (`scripts/sign-linux-artifacts.sh`) and Tauri updater key verification (`tauri.conf.json`).
