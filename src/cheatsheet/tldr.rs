use super::CheatsheetProvider;
use anyhow::Result;
use std::path::PathBuf;

pub struct TldrProvider;

impl TldrProvider {
    pub fn new() -> Self {
        TldrProvider
    }
}

impl CheatsheetProvider for TldrProvider {
    fn fetch(&self, tool_name: &str) -> Result<Option<String>> {
        // Check tealdeer local cache first (avoids network call)
        if let Some(content) = check_local_cache(tool_name) {
            return Ok(Some(content));
        }

        // Fall back to fetching from GitHub
        fetch_from_github(tool_name)
    }
}

fn check_local_cache(tool_name: &str) -> Option<String> {
    let home = std::env::var("HOME").ok()?;

    let candidates = vec![
        // Current tealdeer cache location
        format!(
            "{}/.local/share/tealdeer/tldr-pages/pages/common/{}.md",
            home, tool_name
        ),
        format!(
            "{}/.local/share/tealdeer/tldr-pages/pages/linux/{}.md",
            home, tool_name
        ),
        // Older tealdeer cache location
        format!("{}/.cache/tealdeer/{}.md", home, tool_name),
    ];

    for path_str in candidates {
        let path = PathBuf::from(&path_str);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return Some(content);
            }
        }
    }

    None
}

fn fetch_from_github(tool_name: &str) -> Result<Option<String>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    // Try common page first
    let url = format!(
        "https://raw.githubusercontent.com/tldr-pages/tldr/main/pages/common/{}.md",
        tool_name
    );
    let response = client.get(&url).send()?;
    if response.status().is_success() {
        return Ok(Some(response.text()?));
    }

    // Try linux page
    let url = format!(
        "https://raw.githubusercontent.com/tldr-pages/tldr/main/pages/linux/{}.md",
        tool_name
    );
    let response = client.get(&url).send()?;
    if response.status().is_success() {
        return Ok(Some(response.text()?));
    }

    Ok(None)
}
