<div align="center">
  <img src="src-tauri/icons/128x128.png" width="96" height="96" alt="Forge Desktop logo" />

  # Forge Desktop
  *Fast native Git desktop client built with Rust and Svelte 5*

  [![Latest Release](https://img.shields.io/github/v/release/machfudin/forge-desktop?style=flat-square&color=2563eb)](https://github.com/machfudin/forge-desktop/releases/latest)
  [![Rust](https://img.shields.io/badge/Rust-2024%20edition-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
  [![Tauri](https://img.shields.io/badge/Tauri-v2-24c8db?style=flat-square&logo=tauri)](https://tauri.app/)
  [![Svelte](https://img.shields.io/badge/Svelte-v5-ff3e00?style=flat-square&logo=svelte)](https://svelte.dev/)
  [![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-gray?style=flat-square)](https://github.com/machfudin/forge-desktop/releases/latest)
  [![License: GPL v3](https://img.shields.io/badge/license-GPLv3-blue?style=flat-square)](LICENSE)

  [Download](#download) • [Features](#features) • [Installation](#installation) • [CLI Shortcut](#cli-shortcut) • [Author](#author)

</div>

Forge Desktop is a compiled Git client engineered for fast startup, low memory usage, and full local privacy. It connects directly with GitHub, GitLab, Bitbucket, and Azure DevOps while executing Git commands locally through system binaries.

> [!NOTE]
> Forge Desktop runs as a standalone desktop application with zero background telemetry. All repository credentials stay protected inside your operating system keyring.

## Download

Get precompiled releases for your operating system from the [Latest GitHub Release](https://github.com/machfudin/forge-desktop/releases/latest):

| Operating System | Recommended Download | Alternative Formats |
| :--- | :--- | :--- |
| **Windows** (x64) | `forge-desktop-portable.exe` (run directly, no install required) | `forge-desktop-setup.exe`, `.msi` |
| **Linux** (x64) | `forge-desktop.AppImage` (universal executable) | `.deb` (Debian/Ubuntu), `.rpm` (Fedora/RHEL), `.tar.gz` |
| **macOS** (Apple Silicon / Intel) | `forge-desktop.dmg` (drag to Applications) | `forge-desktop.app.tar.gz` |

## Features

### Multi-Provider Git Hosting
- **Supported Hosts**: GitHub, GitLab, Bitbucket, and Azure DevOps (both cloud and self-hosted Enterprise instances).
- **Direct Publishing**: Publish local Git repositories directly to your remote account with public or private visibility.
- **Pull Requests and Issues**: Inspect active pull requests, merge requests, and issues without opening a browser.
- **CI Pipeline Indicators**: Visual status dots next to commit SHAs show continuous integration status (green for passed, red for failed, yellow for pending).

### Dynamic History Table
- **Customizable Columns**: Reorder columns (`Graph`, `Commit`, `Changes`, `Author`, `Date`, `SHA`) using pointer drag or keyboard controls (`ArrowLeft` / `ArrowRight`).
- **Visibility Control**: Toggle individual columns and choose between Comfortable, Compact, or Dense row spacing.
- **Branch Graph**: Visual Git branch graph with color-coded lanes and clear head indicators.
- **Interactive Rebase**: Visual modal to reorder commits, drop revisions, and squash intermediate changes.

### Granular Staging and Visual Diffs
- **Partial Hunk Staging**: Stage specific lines or hunks directly inside unified or split diff views.
- **Image Diff Inspection**: Compare graphics using three specialized visual inspection modes:
  - **2-Up**: Side-by-side comparison of old and new images.
  - **Swipe Bar**: Interactive divider slider revealing changes dynamically.
  - **Onion Skin**: Adjustable transparency overlay from 0% to 100%.
- **Rendered Markdown Diff**: Preview rendered Markdown diffs with headings, lists, and formatted code blocks.
- **Whitespace Toggle**: Ignore whitespace changes with `--ignore-all-space` to review substantive edits quickly.

### Productivity Accelerators
- **Conventional Commits**: One-click prefixes (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`).
- **Emoji Autocomplete**: Type `:` in commit messages to search and insert Git commit emojis.
- **History Mutations**: Safe one-click commit undo (`git reset --soft HEAD~1`) without losing working tree changes.
- **Conflict Management**: Visual conflict indicators with direct actions to abort or open external editors.
- **In-App Updates**: Built-in update detector with one-click download and installation.

## Installation

### Windows
1. Download `forge-desktop-portable.exe` or `forge-desktop-setup.exe`.
2. Double-click the file to start.
3. If Windows SmartScreen displays an unrecognized application prompt on first launch, select **More info** and click **Run anyway**.

### Linux
1. Download `forge-desktop.AppImage`.
2. Mark the file as executable and launch:
   ```bash
   chmod +x forge-desktop.AppImage
   ./forge-desktop.AppImage
   ```
3. To install via Debian package on Ubuntu/Debian:
   ```bash
   sudo dpkg -i forge-desktop_*_amd64.deb
   ```

### macOS
1. Download `forge-desktop.dmg`.
2. Open the disk image and drag **Forge Desktop** into your **Applications** folder.
3. If macOS Gatekeeper blocks opening on first launch, right-click the application icon, choose **Open**, and confirm the dialog.

## CLI Shortcut

Launch Forge Desktop directly into any local repository folder from your terminal:

```bash
# Open repository in current directory
forge .

# Open repository at specific path
forge ~/projects/my-app
```

> [!TIP]
> To enable the `forge` command from anywhere:
> - **Linux & macOS**: Create a symbolic link to the executable:
>   ```bash
>   sudo ln -sf /path/to/forge /usr/local/bin/forge
>   ```
> - **Windows**: Add the directory containing `forge-desktop.exe` to your user `PATH` environment variable.

## Security & Privacy

- **Local Execution**: All Git operations run locally through your installed Git CLI.
- **Encrypted Credentials**: Tokens and credentials are saved in your operating system credential vault (Windows Credential Manager, macOS Keychain, Linux Secret Service).
- **No Data Harvesting**: Forge Desktop does not transmit analytics, logs, or repository contents to any tracking endpoint.

<details>
<summary><strong>Building from Source</strong></summary>

### Prerequisites
- Node.js 20 or higher
- Rust 1.80 or higher
- Git 2.30 or higher

### Build Steps
```bash
# Clone the repository
git clone https://github.com/machfudin/forge-desktop.git
cd forge-desktop

# Install frontend dependencies
npm install

# Run application in development mode
npm run tauri dev

# Run unit tests
cargo test --manifest-path src-tauri/Cargo.toml

# Compile release package
npm run tauri build
```
</details>

## Author

**Machfudin** (Software Engineer)

- GitHub: [@machfudin](https://github.com/machfudin)
- Project: [github.com/machfudin/forge-desktop](https://github.com/machfudin/forge-desktop)
