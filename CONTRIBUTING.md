# Contributing to lazypkg

Thanks for helping! lazypkg is a terminal UI that shows every tool you installed with any package manager, and moves them all to a new computer with one command. Every package manager someone adds makes it useful to a whole new group of people. That's the main way the project grows, and it's very approachable for first-time contributors.

**New to open source?** Look for issues labelled [`good first issue`](https://github.com/itxmjr/LazyPkg/labels/good%20first%20issue). Comment "I'd like to take this" and it's yours. Questions are welcome in [Discussions](https://github.com/itxmjr/LazyPkg/discussions).

## Ways to contribute

| You have… | Do this |
|---|---|
| 5 minutes | ⭐ Star the repo, or report a bug with the output of `lazypkg list` |
| 30 minutes | [Add a package manager](#add-a-package-manager-in-30-minutes), or add a test fixture from your machine |
| An afternoon | Pick up an issue from the [roadmap](#roadmap) |
| Writing skills | Improve the README or docs, or write about how you use lazypkg |

## Development setup

You need a recent stable Rust toolchain (install via [rustup](https://rustup.rs)). On machines with little RAM, limit build parallelism: `export CARGO_BUILD_JOBS=2`.

```bash
git clone https://github.com/itxmjr/LazyPkg
cd LazyPkg
cargo run            # launch the TUI
cargo run -- list    # CLI: list everything installed
cargo test           # run the test suite
```

Before opening a pull request, run the same checks as CI:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## How the code is organised

```
src/
├── main.rs          CLI commands (list / export / import) and the TUI event loop
├── app.rs           TUI state machine: modes, selection, key handling (unit-tested)
├── exec.rs          The ONLY place that runs external commands (timeouts, sudo, captured output)
├── paths.rs         XDG config/cache directories
├── managers/        One file per package manager — this is where most contributions go
│   ├── defs/        Package managers described in TOML (easiest to add)
│   └── declarative.rs  Loads and runs the TOML definitions
│   └── mod.rs       The PackageManager trait
├── snapshot/        export (write snapshot.toml) and import (diff + install missing)
├── cheatsheet/      tldr pages with a `--help` fallback
└── ui/              ratatui rendering (no logic; reads App state only)
```

Ground rules that keep lazypkg safe:

1. **Never call `std::process::Command` directly.** Use `exec::run_captured`, `exec::run_checked` or `exec::run_interactive`. They close stdin, apply timeouts, and keep command output from drawing over the TUI.
2. **Managers describe commands; they don't run them.** `install_cmd` and `uninstall_cmd` return an `exec::Cmd`. The caller shows it to the user, asks for confirmation and runs it.
3. **Root is explicit.** If a command needs root, mark it with `.with_root(true)`. Never retry a failed command with `sudo` or `pkexec`.
4. **Never lose user data.** If a manager fails to list its packages, report the error. Don't treat it as "nothing installed".

## Add a package manager in 30 minutes

There are two ways. Use **A** whenever you can.

### A. Describe it in TOML (no Rust needed)

If the manager has a command that lists installed packages as plain text, you only need a small TOML file. Look at the built-in ones in [`src/managers/defs/`](src/managers/defs/): `pacman.toml`, `apt.toml`, `brew.toml`, `gem.toml`.

**1. Try it on your machine first**, without touching the code, by saving a file to `~/.config/lazypkg/managers/<name>.toml`:

```toml
name = "flatpak"          # lowercase letters, digits, '-'
icon = "📦"
binary = "flatpak"        # must be on PATH for the manager to appear
system = false            # true for OS package managers (installed first on import)

[list]
cmd = ["flatpak", "list", "--app", "--columns=application,version"]
format = "columns"        # lines | columns | parens  (see below)
skip_lines_containing = []  # optional: drop header/warning lines

[install]
cmd = ["flatpak", "install", "-y", "flathub", "{name}"]
root = false              # true → lazypkg asks for sudo/doas/pkexec, visibly

[uninstall]
cmd = ["flatpak", "uninstall", "{name}"]
```

Then run `lazypkg list --manager flatpak`. If your file has a mistake, lazypkg prints a warning naming the file and the problem. Unknown keys are errors, so typos don't go unnoticed.

| `format` | Output looks like | Example |
|---|---|---|
| `lines` | one name per line | `apt-mark showmanual`, `brew leaves` |
| `columns` | `name version` | `pacman -Qen` |
| `parens` | `name (1.2.3, 1.1.0)` | `gem list` |

`{name}` is replaced by the package name as **one argument**. Nothing goes through a shell, and names starting with `-` are rejected.

**2. Ship it:** copy the file to `src/managers/defs/<name>.toml` and add it to the `BUILTIN` list at the top of `src/managers/declarative.rs`.

**3. Add a test** with a few lines of real output. Put them in `tests/fixtures/<name>/` and add a parse test next to the others in `declarative.rs` (see `pacman_columns`). Include the awkward lines you've seen: headers, warnings, empty output. Use made-up package names, not a dump of your own machine.

That's it. `cargo test` checks that every built-in definition is valid.

### B. Write it in Rust

Use Rust when the output is JSON or the package list comes from a file (npm, pnpm, pipx and cargo work this way). `src/managers/pipx.rs` is a short, complete template:

1. Create `src/managers/<name>.rs` with a struct implementing `PackageManager` (`name`, `icon`, `is_available`, `list_installed`, `install_cmd`, `uninstall_cmd`).
2. Keep parsing in a pure function (`fn parse_json(&str) -> Result<Vec<Tool>>`) and unit-test it with real output.
3. Register it in `all_managers()` in `src/managers/mod.rs`.
4. Optional overrides: `is_system()` for OS managers, and `normalize_name()` if names are case-insensitive (see `pip::normalize_python_name`).

Then check it end to end: `cargo run -- list --manager <name>`, and try uninstalling something harmless in the TUI.

Already built in: cargo, npm, pnpm, pip, pipx, dnf, pacman, apt, brew, gem.\nWanted (each is a good first issue): **paru/yay (AUR), flatpak, snap, uv tool, go install, bun, yarn, cargo-binstall, brew casks, mise, nix profile, zypper, xbps, apk, winget, scoop**.

## Pull requests

- Keep each PR focused on one change. Small PRs get merged quickly.
- Use [Conventional Commits](https://www.conventionalcommits.org/) messages: `feat: add pacman manager`, `fix: …`, `docs: …`.
- Add or update tests for any behaviour you change.
- Update `README.md` if you add a manager, flag or key binding.
- CI must pass (format, clippy, tests).

Maintainers aim to reply to every issue and PR within **48 hours**. If you've waited longer, a friendly ping is welcome.

## Reporting bugs

Use the bug report template. The most useful things to include:
- your OS or distro, and `lazypkg --version`
- the exact command you ran and the full output
- for wrong or missing packages: the raw output of the manager's own list command (for example `npm list -g --json --depth=0`)

Security problems: please follow [SECURITY.md](SECURITY.md) instead of opening a public issue.

## Roadmap

The big picture, in order:
1. **Stability:** safe uninstall/import, no TUI corruption, honest error messages.
2. **Core refactor:** ✅ package managers defined in TOML; next, a test-friendly command runner.
3. **More managers + cross-machine sync:** a versioned `lazypkg.toml` you keep in your dotfiles, with name translation between distros (`fd` on Arch ↔ `fd-find` on Debian), plus `lazypkg diff`, `apply --dry-run` and `doctor`.
4. **Packaging:** prebuilt binaries, AUR, Homebrew, Copr.

## Code of conduct

Be kind and assume good intent. This project follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
