# Changelog

All notable changes to lazypkg are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-11-26

### Added
- **pacman, apt, Homebrew and gem support.** lazypkg now covers ten package managers.
- **Package managers defined in TOML.** Add or override a manager with a short file in `~/.config/lazypkg/managers/`, no Rust or rebuild needed. Invalid files produce a warning that names the file and the problem.
- `lazypkg import --dry-run` shows what would be installed; `--yes` skips the confirmation for setup scripts.
- `lazypkg list --manager <name>`; an unknown name is an error.
- Exit codes: 0 ok, 1 error, 2 partial failure.
- Loading spinners, empty-state messages and a status line that stays visible for 4 seconds.

### Changed
- Uninstall always asks first and shows the exact command, including whether it needs root.
- Commands that need `sudo`/`doas`/`pkexec` pause the TUI so the password prompt works, then bring you back.
- Export writes atomically, keeps the previous file as `snapshot.toml.bak`, and uses sorted keys for clean git diffs.
- Import installs system managers first, compares Python names case-insensitively (PEP 503), and explains every skipped package.
- `pip install`/`uninstall` failures caused by PEP 668 suggest pipx or uv.
- TLS via rustls instead of OpenSSL; tokio removed. Smaller binary, no system TLS library needed.

### Fixed
- **Security:** package names from a snapshot that start with `-` (e.g. `--index-url=https://…`), or contain spaces or control characters, are rejected before anything runs.
- npm uninstall no longer silently retries with `pkexec`.
- Import no longer reinstalls everything when a manager fails to list its packages.
- Export no longer drops managers that failed to list, or overwrites an unreadable snapshot.
- `dnf install` during import now asks for root instead of always failing.
- Pressing `e` no longer freezes the TUI; export runs in the background.
- Uninstall output no longer draws over the TUI.
- While a confirmation or help popup is open, other keys no longer act on the list behind it.
- The tools list no longer gets stuck on "Loading…" when switching managers quickly.
- The `--help` cheatsheet fallback only runs binaries found on `PATH`, with a 2-second timeout.
- Cheatsheets for scoped npm packages (`@scope/pkg`).

## [0.1.0] - 2026-03-23

First release: a TUI listing cargo, dnf, pip, pipx, npm and pnpm packages, with tldr cheatsheets, uninstall, and snapshot export/import.

[0.2.0]: https://github.com/itxmjr/LazyPkg/compare/5560ab1...v0.2.0
[0.1.0]: https://crates.io/crates/lazypkg/0.1.0
