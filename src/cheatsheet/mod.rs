use anyhow::Result;

pub mod tldr;
pub mod help;

pub trait CheatsheetProvider {
    fn fetch(&self, tool_name: &str) -> Result<Option<String>>;
}

/// Load cheatsheet for a tool: tries tldr first, falls back to --help
pub fn load_cheatsheet(tool_name: &str) -> Option<String> {
    // Try tldr first
    let tldr = tldr::TldrProvider::new();
    if let Ok(Some(content)) = tldr.fetch(tool_name) {
        return Some(content);
    }

    // Fallback to --help
    let help = help::HelpProvider::new();
    help.fetch(tool_name).ok().flatten()
}
