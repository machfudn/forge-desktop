<div align="center">
  <img src="src-tauri/icons/128x128.png" width="96" height="96" alt="Forge Desktop logo" />

  # Forge Desktop
  **Fast, native, telemetry-free Git client built with Rust and Svelte 5**

  [![Latest Release](https://img.shields.io/github/v/release/machfudn/forge-desktop?style=flat-square&color=2563eb)](https://github.com/machfudn/forge-desktop/releases/latest)
  [![Rust](https://img.shields.io/badge/Rust-2024%20edition-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
  [![Tauri](https://img.shields.io/badge/Tauri-v2-24c8db?style=flat-square&logo=tauri)](https://tauri.app/)
  [![Svelte](https://img.shields.io/badge/Svelte-v5-ff3e00?style=flat-square&logo=svelte)](https://svelte.dev/)
  [![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-gray?style=flat-square)](https://github.com/machfudn/forge-desktop/releases/latest)
  [![License: GPL v3](https://img.shields.io/badge/license-GPLv3-blue?style=flat-square)](LICENSE)

  [Download](#download) • [Features](#features) • [Installation](#installation) • [CLI Shortcut](#cli-shortcut) • [Security & Privacy](#security--privacy) • [Building](#building-from-source)

</div>

---

Forge Desktop is a compiled Git desktop client engineered for near-instant startup, low memory usage, and full local privacy. It connects directly with GitHub, GitLab, Bitbucket, and Azure DevOps while executing Git commands locally through system binaries.

> [!NOTE]
> Forge Desktop operates strictly on your local machine with zero background telemetry or tracking. All authentication tokens and credentials are encrypted inside your operating system's native keyring vault.

---

## Download

Precompiled releases are available for Windows, Linux, and macOS on the [Latest GitHub Release](https://github.com/machfudn/forge-desktop/releases/latest):

| Operating System | Recommended Download | Alternative Formats |
| :--- | :--- | :--- |
| **Windows** (x64) | `forge-desktop-portable.exe` *(no installation required)* | `forge-desktop-setup.exe`, `.msi` |
| **Linux** (x64) | `forge-desktop.AppImage` *(universal binary)* | `.deb` (Debian/Ubuntu), `.rpm` (Fedora/RHEL), `.tar.gz` |
| **macOS** (Apple Silicon / Intel) | `forge-desktop.dmg` *(drag to Applications)* | `forge-desktop.app.tar.gz` |

---

## Features

### Multi-Provider Git Hosting
- **Unified Account Integration**: Connect GitHub, GitLab, Bitbucket, and Azure DevOps (both cloud and self-hosted enterprise servers) in one client.
- **Repository Publishing**: Publish local Git repositories directly to your remote provider with configurable public or private visibility.
- **Integrated Pull Requests & Issues**: Review active pull requests, inspect discussions, and track issues without switching to a web browser.
- **CI/CD Pipeline Status**: Visual indicators next to commit SHAs display real-time build and test statuses from integrated pipelines.

### Dynamic History & Branch Graph
- **Customizable Columns**: Toggle visibility and reorder columns (`Graph`, `Commit`, `Changes`, `Author`, `Date`, `SHA`) via drag-and-drop or keyboard shortcuts (`ArrowLeft` / `ArrowRight`).
- **Flexible Row Density**: Choose between Comfortable, Compact, or Dense views to match your workflow density preferences.
- **Color-Coded Branch Graph**: Clear visual topological lanes identifying merged branches, head pointers, and upstream tracking status.
- **Visual Interactive Rebase**: Reorder, squash, edit, or drop commits visually with safety checks before modifying Git history.

### Granular Staging & Rich Diffs
- **Hunk & Line-Level Staging**: Select and stage specific lines or code blocks directly within unified or side-by-side diff views.
- **Specialized Image Diff Modes**:
  - **2-Up**: Side-by-side visual comparison of before and after images.
  - **Swipe Bar**: Interactive divider revealing pixel-level changes dynamically.
  - **Onion Skin**: Adjustable transparency overlay (0% to 100%) to spot subtle graphic modifications.
- **Rendered Markdown Diff**: Preview formatted Markdown documents with typography, lists, and rendered syntax highlighting.
- **Whitespace Controls**: Quickly toggle whitespace changes (`--ignore-all-space`) to focus on meaningful code logic.

### Productivity Accelerators
- **Conventional Commits**: One-click standardized commit prefixes (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`).
- **Emoji Autocomplete**: Type `:` within the commit editor to search and insert Git commit emojis.
- **Safe Commit Undo**: Single-click commit rollback (`git reset --soft HEAD~1`) without losing uncommitted changes.
- **Conflict Resolution**: Integrated conflict markers with immediate options to resolve in-app or launch external merge editors.
- **Seamless In-App Updates**: Built-in update checker providing one-click downloads and upgrades.

---

## Installation

### Windows
1. Download `forge-desktop-portable.exe` or `forge-desktop-setup.exe`.
2. Launch the downloaded executable.
3. If Windows SmartScreen displays a warning on first launch, select **More info** and click **Run anyway**.

### Linux
1. Download `forge-desktop.AppImage`.
2. Make the file executable and run:
   ```bash
   chmod +x forge-desktop.AppImage
   ./forge-desktop.AppImage
   ```
3. Alternatively, install the Debian package on Ubuntu/Debian:
   ```bash
   sudo dpkg -i forge-desktop_*_amd64.deb
   ```

### macOS
1. Download `forge-desktop.dmg`.
2. Open the disk image and drag **Forge Desktop** into your **Applications** folder.
3. If macOS Gatekeeper prevents opening on first launch, right-click the application icon, select **Open**, and confirm the prompt.

---

## CLI Shortcut

Launch Forge Desktop directly into any repository directory from your terminal:

```bash
# Open current repository
forge .

# Open specific repository path
forge ~/projects/my-app
```

> [!TIP]
> To enable the `forge` command globally:
> - **Linux & macOS**: Create a symlink to the binary:
>   ```bash
>   sudo ln -sf /path/to/forge /usr/local/bin/forge
>   ```
> - **Windows**: Add the directory containing `forge-desktop.exe` to your user `PATH` environment variable.

---

## Security & Privacy

- **Local-Only Execution**: All operations run locally through your installed Git CLI.
- **OS Keyring Security**: Passwords and personal access tokens are stored in the platform vault (Windows Credential Manager, macOS Keychain, Linux Secret Service).
- **No Data Harvesting**: Forge Desktop does not collect analytics, telemetry, or repository contents.

---

## Building from Source

<details>
<summary><strong>Build Instructions & Prerequisites</strong></summary>

### Prerequisites
- Node.js 20 or higher
- Rust 1.80 or higher
- Git 2.30 or higher

### Build Steps
```bash
# Clone the repository
git clone https://github.com/machfudn/forge-desktop.git
cd forge-desktop

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Run unit tests
cargo test --manifest-path src-tauri/Cargo.toml

# Compile release package
npm run tauri build
```

</details>

---

## Author

**Machfudin**
- GitHub: [@machfudn](https://github.com/machfudn)
- Project Repository: [github.com/machfudn/forge-desktop](https://github.com/machfudn/forge-desktop)
