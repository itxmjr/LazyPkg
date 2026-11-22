## What does this change?

<!-- One or two sentences. Link the issue: "Closes #123" -->

## How did you test it?

<!-- e.g. "cargo test", "ran `lazypkg list --manager pacman` on Arch", screenshots of the TUI -->

## Checklist

- [ ] `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass
- [ ] Tests added or updated (parsers use real command output)
- [ ] README updated if I added a manager, flag or key binding
- [ ] No direct `std::process::Command`, external commands go through `exec`
