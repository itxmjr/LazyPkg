# Records assets/demo.gif for the README.
#
#   cargo build --release
#   vhs assets/demo.tape
#
# Needs: vhs and ttyd (pacman -S vhs ttyd / brew install vhs), plus ffmpeg.
# It only browses, searches and does a dry run: nothing is installed or removed.
# The searches use tools installed on the recording machine (npm: vercel, pnpm);
# change them if you re-record elsewhere.

Output assets/demo.gif

Set Shell "bash"
Set FontFamily "Noto Sans Mono"
Set FontSize 16
Set Width 1280
Set Height 720
Set Padding 12
Set TypingSpeed 70ms
# Slightly slower than real time so each step is easy to follow.
Set PlaybackSpeed 0.8

# kitty's default colours
Set Theme { "name": "kitty", "background": "#000000", "foreground": "#dddddd", "cursor": "#cccccc", "selection": "#fffacd", "black": "#000000", "red": "#cc0403", "green": "#19cb00", "yellow": "#cecb00", "blue": "#0d73cc", "purple": "#cb1ed1", "cyan": "#0dcdcd", "white": "#dddddd", "brightBlack": "#767676", "brightRed": "#f2201f", "brightGreen": "#23fd00", "brightYellow": "#fffd00", "brightBlue": "#1a8fff", "brightPurple": "#fd28ff", "brightCyan": "#14ffff", "brightWhite": "#ffffff" }

# Use the freshly built binary, a plain prompt, and a "snapshot from another
# machine" for the import step at the end.
Hide
Type "export PATH=$PWD/target/release:$PATH PS1='$ '"
Enter
Type `printf '[meta]\nhostname = "old-laptop"\n\n[packages.npm]\ntools = ["pnpm", "typescript", "vercel"]\n\n[packages.pipx]\ntools = ["httpie", "online-judge-tools"]\n' > /tmp/new-laptop.toml && clear`
Enter
Show

# ── 1. Every package manager in one place ─────────────────────────────────
Type "lazypkg"
Sleep 400ms
Enter
Sleep 3s

Down
Sleep 700ms
Down
Sleep 700ms
Up
Sleep 700ms
Up
Sleep 1s

# ── 2. "/" in the managers panel searches EVERY manager ───────────────────
Type "/"
Sleep 1.2s
Type "verc"
Sleep 1.5s
Enter
Sleep 400ms
# Open the cheatsheet of the result
Enter
Sleep 3.5s

# Esc goes back to the managers panel
Escape
Sleep 1s

# ── 3. "/" in the tools panel searches only that manager (npm) ────────────
Down 3
Sleep 1s
Right
Sleep 1.2s
Type "/"
Sleep 600ms
Type "pn"
Sleep 2.5s
Escape
Sleep 800ms

# ── 4. Help, then quit ────────────────────────────────────────────────────
Type "?"
Sleep 3s
Escape
Sleep 600ms
Type "q"
Sleep 800ms

# ── 5. Moving to a new computer ───────────────────────────────────────────
Type "lazypkg list --manager npm"
Sleep 400ms
Enter
Sleep 2.5s

Type "lazypkg import /tmp/new-laptop.toml --dry-run"
Sleep 400ms
Enter
Sleep 4.5s
