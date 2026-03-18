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
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            i += 1;
            if i >= bytes.len() { break; }
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
