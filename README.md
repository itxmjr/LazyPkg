# lazypkg 📦

A lazygit-style terminal UI (TUI) tool to seamlessly manage all your CLI utilities installed across various package managers including **cargo, dnf, pipx, pip, npm, and pnpm**.

No more wondering if that tool you used 3 months ago was installed via `pip3`, `cargo`, or `npm`. Find them, read their cheatsheets, and manage them all from a single beautiful interface!

![lazypkg TUI concept](https://i.imgur.com/placeholder_lazypkg.png)

## Features 🚀
- 🔍 **Universal Discovery**: Automatically scans all major package managers (`cargo, dnf, pipx, pip, npm, pnpm`) for installed tools.
- 📖 **Instant Cheatsheets**: Automatically loads [tldr](https://tldr.sh) pages to show you how to use a tool the instant you select it.
- 🗑️ **Easy Deletions**: Press `d` to cleanly uninstall a tool from its respective package manager.
- 📦 **Snapshot & Restore**: Export your entire toolchain setup across all package managers to a single `.toml` file and restore it on new machines!

## Installation 🛠️
Ensure you have Rust installed, then install from source:

```bash
cargo install --path .
```
*(GitHub Releases & Crates.io coming soon!)*

## Usage 💻
Just type `lazypkg` into your terminal to launch the interactive UI!

### CLI Commands
```bash
lazypkg                    # Launch the interactive TUI
lazypkg list               # List all installed packages to stdout
lazypkg list --manager npm # Filter packages to only NPM
lazypkg export             # Export your tools to ~/.config/lazypkg/snapshot.toml
lazypkg import             # Import and install missing tools from snapshot
```

## Key Bindings ⌨️

While in the TUI, use the following keybinds:

- <kbd>j</kbd> / <kbd>k</kbd> or <kbd>Up</kbd> / <kbd>Down</kbd> : Navigate lists
- <kbd>h</kbd> / <kbd>l</kbd> or <kbd>Left</kbd> / <kbd>Right</kbd> : Change active panel
- <kbd>Tab</kbd> : Next panel
- <kbd>Enter</kbd> : Focus on cheatsheet for the tool
- <kbd>d</kbd> : Delete (uninstall) selected tool
- <kbd>r</kbd> : Refresh tool list
- <kbd>/</kbd> : Search filter
- <kbd>e</kbd> : Export snapshot of all tools
- <kbd>?</kbd> : Open Help popup
- <kbd>q</kbd> : Quit

## Supported Package Managers
- `cargo` — Global crates installed in `~/.cargo/bin`
- `dnf` — User-installed system software via Fedora's package manager
- `pipx` — Python applications in isolated environments
- `pip` — Global user Python packages
- `npm` / `pnpm` — Node.js global dependencies

## Contributing
Pull requests are welcome! In the future (v2), we will support TOML-defined plugin structures for arbitrary package managers.
