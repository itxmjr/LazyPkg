use anyhow::Result;

pub mod tldr;
pub mod help;

pub trait CheatsheetProvider {
    fn fetch(&self, tool_name: &str) -> Result<Option<String>>;
}

/// Load cheatsheet for a tool: tries tldr first, falls back to --help
pub fn load_cheatsheet(tool_name: &str) -> Option<String> {
    let tldr = tldr::TldrProvider::new();
    match tldr.fetch(tool_name) {
        Ok(Some(content)) => return Some(content),
        Ok(None) => {} // No tldr page, try --help
        Err(e) => {
            // Try --help as fallback but also note the tldr error
            let help = help::HelpProvider::new();
            if let Ok(Some(content)) = help.fetch(tool_name) {
                return Some(content);
            }
            return Some(format!("[tldr error: {}]\n\nNo --help output found either.", e));
        }
    }

    let help = help::HelpProvider::new();
    match help.fetch(tool_name) {
        Ok(Some(content)) => Some(content),
        Ok(None) => None,
        Err(e) => Some(format!("Error loading cheatsheet: {}", e)),
    }
}
