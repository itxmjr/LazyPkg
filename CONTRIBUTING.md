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

Let's say you want to add `gem`. Use `src/managers/pipx.rs` as a template; it's short and complete.

**1. Create `src/managers/gem.rs`:**

```rust
use anyhow::Result;
use super::{list_output, PackageManager, Tool};
use crate::exec::Cmd;

pub struct GemManager;

impl GemManager {
    pub fn new() -> Self { GemManager }

    /// Parse `gem list --local` output: "name (1.2.3, 1.1.0)"
    fn parse(output: &str) -> Vec<Tool> {
        let mut tools: Vec<Tool> = output
            .lines()
            .filter_map(|line| {
                let (name, rest) = line.split_once(" (")?;
                let version = rest.trim_end_matches(')').split(',').next().map(|v| v.trim().to_string());
                Some(Tool { name: name.trim().to_string(), version, manager: "gem".into() })
            })
            .collect();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
    }
}

impl PackageManager for GemManager {
    fn name(&self) -> &str { "gem" }
    fn icon(&self) -> &str { "💎" }
    fn is_available(&self) -> bool { crate::exec::which("gem").is_some() }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        let out = list_output(&Cmd::new("gem", &["list", "--local"]))?;
        Ok(Self::parse(&out))
    }

    fn install_cmd(&self, name: &str) -> Cmd {
        Cmd::new("gem", &["install", "--user-install", name])
    }

    fn uninstall_cmd(&self, tool: &Tool) -> Cmd {
        Cmd::new("gem", &["uninstall", "-x", tool.name.as_str()])
    }
}
```

**2. Register it** in `src/managers/mod.rs`: add `pub mod gem;` and `Arc::new(gem::GemManager::new())` to `all_managers()`.

**3. Add tests using real output.** Run the list command on your machine and paste (part of) its output into a test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gem_list() {
        let out = "bundler (2.5.6, 2.4.22)\nrake (13.1.0)\n";
        let tools = GemManager::parse(out);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "bundler");
        assert_eq!(tools[0].version.as_deref(), Some("2.5.6"));
    }
}
```

Include awkward cases you've seen: empty output, warning lines, unusual version strings. Parsing bugs are the most common source of bug reports.

**4. Optional overrides:**
- `is_system()` → `true` for OS package managers (pacman, apt, dnf, brew…), so they're installed first during import.
- `normalize_name()` if the ecosystem treats names case-insensitively (see `pip::normalize_python_name`).

**5. Check it end to end:** `cargo run -- list --manager gem`, then open the TUI and try uninstalling something harmless.

Wanted managers (each is a good first issue): **pacman, paru/yay (AUR), apt, Homebrew, flatpak, snap, uv tool, go install, gem, bun, yarn, cargo-binstall, mise, nix profile, zypper, xbps, apk**.

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
2. **Core refactor:** a test-friendly command runner, and package managers defined in TOML so new ones need no Rust.
3. **More managers + cross-machine sync:** a versioned `lazypkg.toml` you keep in your dotfiles, with name translation between distros (`fd` on Arch ↔ `fd-find` on Debian), plus `lazypkg diff`, `apply --dry-run` and `doctor`.
4. **Packaging:** prebuilt binaries, AUR, Homebrew, Copr.

## Code of conduct

Be kind and assume good intent. This project follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
