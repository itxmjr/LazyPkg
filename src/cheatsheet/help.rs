use super::CheatsheetProvider;
use anyhow::Result;

const HELP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

pub struct HelpProvider;

impl HelpProvider {
    pub fn new() -> Self {
        HelpProvider
    }
}

impl CheatsheetProvider for HelpProvider {
    fn fetch(&self, tool_name: &str) -> Result<Option<String>> {
        // Only run something that really is an executable on PATH (a package
        // name is not necessarily a command), never a path.
        if tool_name.contains('/') || crate::exec::which(tool_name).is_none() {
            return Ok(None);
        }
        // stdin is closed and the process is killed after HELP_TIMEOUT, so a
        // program that ignores --help (starts a GUI, a server, a REPL…) can't hang us.
        let output = match crate::exec::run_captured(
            &crate::exec::Cmd::new(tool_name, &["--help"]),
            Some(HELP_TIMEOUT),
        ) {
            Ok(o) => o,
            Err(_) => return Ok(None),
        };

        // --help output goes to stdout or stderr depending on the tool
        let raw = if output.stdout.trim().is_empty() {
            output.stderr
        } else {
            output.stdout
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
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            i += 1;
            if i >= bytes.len() {
                break;
            }
            match bytes[i] {
                b'[' => {
                    // CSI sequence - skip until final byte (0x40-0x7E)
                    i += 1;
                    while i < bytes.len() && !(0x40..=0x7E).contains(&bytes[i]) {
                        i += 1;
                    }
                    i += 1; // skip final byte
                }
                b']' => {
                    // OSC sequence - skip until BEL or ST (ESC \)
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == 0x07 {
                            i += 1; // BEL terminator
                            break;
                        } else if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            i += 2; // ST terminator
                            break;
                        }
                        i += 1;
                    }
                }
                _ => {
                    // Other ESC sequences - skip one char
                    i += 1;
                }
            }
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}
