# Security Policy

lazypkg runs package-manager commands on your machine, sometimes as root, so security reports are taken seriously.

## Supported versions

Only the latest release on crates.io receives fixes.

## Reporting a vulnerability

Please **do not open a public issue**. Instead use GitHub's private reporting:
**[Report a vulnerability](https://github.com/itxmjr/LazyPkg/security/advisories/new)**.

Include what you found, how to reproduce it, and the impact. You'll get a reply within 72 hours, and a fix or mitigation plan within 14 days for confirmed issues. Credit is given in the release notes unless you prefer otherwise.

## Design rules we hold ourselves to

- Commands needing root are always shown to you before they run, and are never run as a silent retry.
- Package names are passed as separate arguments (never through a shell), and names used for file paths or URLs are validated.
- External commands run with stdin closed and with timeouts, so a misbehaving program can't hang lazypkg.
