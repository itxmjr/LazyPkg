# 📦 lazypkg

![Rust](https://img.shields.io/badge/Rust-stable-orange.svg?style=for-the-badge&logo=rust&logoColor=white)
![Ratatui](https://img.shields.io/badge/Ratatui-0.28-blueviolet.svg?style=for-the-badge)
[![Crates.io](https://img.shields.io/crates/v/lazypkg.svg?style=for-the-badge&logo=rust)](https://crates.io/crates/lazypkg)
[![CI](https://img.shields.io/github/actions/workflow/status/itxmjr/LazyPkg/ci.yml?branch=main&style=for-the-badge&label=CI)](https://github.com/itxmjr/LazyPkg/actions/workflows/ci.yml)
![License](https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge)

**lazypkg** is a lazygit-style terminal UI that shows every command-line tool you installed globally, across **cargo, npm, pnpm, pip, pipx and dnf**, in one place. Use it to find out which package manager installed a tool, read its cheatsheet, uninstall it, or export your whole toolchain to one TOML file and reinstall it on a new machine. Written in **Rust** with **Ratatui**, shipped as a single binary.

[Report Bug](https://github.com/itxmjr/LazyPkg/issues/new?template=bug_report.yml) · [Request a Package Manager](https://github.com/itxmjr/LazyPkg/issues/new?template=new_manager.yml) · [Contribute](CONTRIBUTING.md)

## ✨ Features

- **One list for every package manager**: see global cargo crates, npm/pnpm packages, pip user packages, pipx apps and user-installed dnf packages side by side.
- **Instant cheatsheets**: the selected tool's [tldr](https://tldr.sh) page loads next to the list, falling back to its `--help` output.
- **Safe uninstall**: press `d`, and lazypkg shows the exact command it will run, and whether it needs root, before doing anything.
- **Password prompts that work**: commands that need `sudo`/`doas`/`pkexec` pause the TUI so you can type your password, then bring you back.
- **Snapshot & restore**: `lazypkg export` writes all your tools to `~/.config/lazypkg/snapshot.toml`; `lazypkg import` installs whatever is missing on another machine.
- **Never loses your snapshot**: writes are atomic, the previous file is kept as `snapshot.toml.bak`, and a manager that fails to list keeps its old entries.
- **Script-friendly CLI**: `list`, `export`, `import --dry-run`, `import --yes`, with meaningful exit codes (0 ok, 1 error, 2 partial).

## 🛠️ Tech Stack

- [Rust](https://www.rust-lang.org/): single binary, no runtime needed
- [Ratatui](https://ratatui.rs) + [crossterm](https://github.com/crossterm-rs/crossterm): terminal UI
- [clap](https://github.com/clap-rs/clap): command-line interface
- [serde](https://serde.rs) + [toml](https://github.com/toml-rs/toml): snapshot file format
- [reqwest](https://github.com/seanmonstar/reqwest) (rustls): fetching tldr pages

## 📦 Supported package managers

| Manager | What lazypkg lists | Needs root to change? |
|---|---|---|
| `cargo` | crates installed with `cargo install` | no |
| `npm` | global packages (`npm list -g`) | only if the global prefix isn't writable by you |
| `pnpm` | global packages (`pnpm list -g`) | no |
| `pipx` | apps installed with pipx | no |
| `pip` | user-site packages (`pip list --user`) | no |
| `dnf` | packages you installed explicitly | yes |

**Coming next:** pacman + AUR (paru/yay), apt, Homebrew, flatpak, `uv tool`, `go install`, and cross-distro name translation for import. Want one sooner? [Adding a manager takes about 30 minutes](CONTRIBUTING.md#add-a-package-manager-in-30-minutes).

## 📦 Installation

**Prerequisites:** a Rust toolchain ([rustup](https://rustup.rs)). Linux is fully supported; macOS support is in progress.

1. Install from crates.io:

```bash
cargo install lazypkg
```

2. Or install the latest development version:

```bash
cargo install --git https://github.com/itxmjr/LazyPkg
```

3. Or build from source:

```bash
git clone https://github.com/itxmjr/LazyPkg
cd LazyPkg
cargo install --path .
```

## 🎮 Usage

1. Launch the TUI:

```bash
lazypkg
```

2. Move with `j`/`k`, switch panels with `h`/`l` or `Tab`, press `?` for all keys.

| Key | Action |
|---|---|
| `j` `k` / `↑` `↓` | Move in the list |
| `h` `l` / `←` `→` / `Tab` | Switch panel |
| `Enter` | Open the cheatsheet |
| `d` | Uninstall the selected tool (asks first) |
| `/` | Search (`Enter` keeps the filter, `Esc` clears it) |
| `r` / `R` | Refresh tools / re-download the cheatsheet |
| `e` | Export snapshot |
| `?` | Help |
| `q` / `Ctrl+C` | Quit |

3. Use the CLI for scripts and new machines:

```bash
lazypkg list                     # every installed package, as manager: name@version
lazypkg list --manager npm       # only one manager
lazypkg export                   # write ~/.config/lazypkg/snapshot.toml
lazypkg export -o tools.toml     # write somewhere else (e.g. your dotfiles repo)
lazypkg import --dry-run         # show what's missing on this machine
lazypkg import                   # install it (asks for confirmation)
lazypkg import tools.toml --yes  # non-interactive, for setup scripts
```

### Moving to a new computer

```bash
# old machine
lazypkg export -o ~/dotfiles/lazypkg.toml

# new machine (after installing lazypkg)
lazypkg import ~/dotfiles/lazypkg.toml --dry-run
lazypkg import ~/dotfiles/lazypkg.toml
```

The snapshot is plain TOML with sorted keys, so it's easy to read and gives clean diffs in git:

```toml
[meta]
date = "2026-11-22T10:00:00Z"
hostname = "laptop"
lazypkg_version = "0.1.0"

[packages.cargo]
tools = ["bat", "ripgrep"]

[packages.pipx]
tools = ["black", "httpie"]
```

## ❓ FAQ

**How do I see all my globally installed packages at once?**
Run `lazypkg` (TUI) or `lazypkg list` (plain text). It asks each installed package manager and shows everything together.

**How do I find out which package manager installed a tool?**
Open `lazypkg`, press `/` and type the name. Each tool is listed under the manager that owns it.

**How do I back up my CLI tools before reinstalling my OS?**
`lazypkg export -o somewhere-safe.toml`, then `lazypkg import somewhere-safe.toml` after the reinstall.

**Is this the same as the Go projects called lazypkg?**
No. This is the Rust project published on crates.io as [`lazypkg`](https://crates.io/crates/lazypkg). Its focus is snapshot-and-restore of your toolchain across machines.

**Does it ever run things as root without asking?**
No. Every uninstall is confirmed first, shows the exact command, and tells you if it needs administrator rights. There are no silent `sudo` retries.

## 📂 Project Structure

```text
LazyPkg/
├── src/
│   ├── main.rs          # CLI commands (list/export/import) and TUI event loop
│   ├── app.rs           # TUI state machine: modes, selection, key handling
│   ├── exec.rs          # runs every external command (timeouts, sudo, captured output)
│   ├── paths.rs         # XDG config/cache directories
│   ├── managers/        # one file per package manager (cargo, dnf, npm, pnpm, pip, pipx)
│   ├── snapshot/        # export to / import from snapshot.toml
│   ├── cheatsheet/      # tldr pages with --help fallback
│   └── ui/              # Ratatui rendering
├── .github/             # CI workflow, issue and PR templates
├── CONTRIBUTING.md      # how to add a package manager
├── llms.txt             # project summary for AI assistants
└── Cargo.toml
```

## 🤝 Contributing

Contributions are welcome, especially new package managers. Read the [contributing guide](CONTRIBUTING.md) and pick a [`good first issue`](https://github.com/itxmjr/LazyPkg/labels/good%20first%20issue).

1. Fork the repository.
2. Create your feature branch (`git checkout -b feature/AmazingFeature`).
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4. Push to the branch (`git push origin feature/AmazingFeature`).
5. Open a Pull Request.

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🌐 Connect

<div align="center">

<a href="https://mjawad.tech"><img src="https://img.shields.io/badge/M%20Jawad-Portfolio-39ff14?style=for-the-badge&logo=googlechrome&logoColor=white&labelColor=000000" alt="Portfolio" /></a>
<a href="https://linkedin.com/in/itxmjr"><img src="https://img.shields.io/badge/LinkedIn-0A66C2?style=for-the-badge&logo=linkedin&logoColor=white" alt="LinkedIn" /></a>
<a href="https://github.com/itxmjr"><img src="https://img.shields.io/badge/GitHub-181717?style=for-the-badge&logo=github&logoColor=white" alt="GitHub" /></a>
<a href="https://youtube.com/@aibymjr"><img src="https://img.shields.io/badge/YouTube-FF0000?style=for-the-badge&logo=youtube&logoColor=white" alt="YouTube" /></a>
<a href="https://www.fiverr.com/aibymjr"><img src="https://img.shields.io/badge/Fiverr-1DBF73?style=for-the-badge&logo=fiverr&logoColor=white" alt="Fiverr" /></a>
<a href="https://www.upwork.com/freelancers/~011c75cbed1d1a6c4b"><img src="https://img.shields.io/badge/Upwork-14A800?style=for-the-badge&logo=upwork&logoColor=white" alt="Upwork" /></a>

<sub>Built with ❤️ by M Jawad ur Rehman.</sub>

</div>
