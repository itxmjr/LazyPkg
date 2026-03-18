use super::CheatsheetProvider;
use anyhow::Result;

pub struct HelpProvider;

impl HelpProvider {
    pub fn new() -> Self {
        HelpProvider
    }
}

impl CheatsheetProvider for HelpProvider {
    fn fetch(&self, tool_name: &str) -> Result<Option<String>> {
        let output = match std::process::Command::new(tool_name)
            .arg("--help")
            .output()
        {
            Ok(o) => o,
            Err(_) => return Ok(None),
        };

        // Combine stdout and stderr (--help output varies by tool)
        let raw = if output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stderr).into_owned()
        } else {
            String::from_utf8_lossy(&output.stdout).into_owned()
        };

        if raw.trim().is_empty() {
            return Ok(None);
        }

        let cleaned = strip_ansi(&raw);
        let lines: Vec<&str> = cleaned.lines().take(30).collect();
        Ok(Some(lines.join("\n")))
    }
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Check for ESC [ sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we hit a letter (the command terminator)
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                // Other ESC sequences - skip the next char
                chars.next();
            }
        } else {
            result.push(c);
        }
    }

    result
}
