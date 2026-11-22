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

fn tealdeer_cache_paths(tool_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG_DATA_HOME (current tealdeer)
    if let Some(data_home) = crate::paths::data_home() {
        let base = data_home.join("tealdeer").join("tldr-pages").join("pages");
        paths.push(base.join("common").join(format!("{}.md", tool_name)));
        paths.push(base.join("linux").join(format!("{}.md", tool_name)));
    }

    // XDG_CACHE_HOME (older tealdeer versions)
    if let Some(cache_home) = crate::paths::cache_home() {
        paths.push(
            cache_home
                .join("tealdeer")
                .join(format!("{}.md", tool_name)),
        );
    }

    paths
}

/// Where lazypkg caches a downloaded tldr page. `None` if the name isn't a
/// safe page name or no cache directory can be determined.
pub fn lazypkg_cache_path(tool_name: &str) -> Option<PathBuf> {
    let name = super::page_name(tool_name)?;
    Some(
        crate::paths::cache_dir()?
            .join("cheatsheets")
            .join(format!("{}.md", name)),
    )
}

fn check_local_cache(tool_name: &str) -> Option<String> {
    // Check our own cache first
    if let Some(lazy_path) = lazypkg_cache_path(tool_name) {
        if let Ok(content) = std::fs::read_to_string(&lazy_path) {
            return Some(content);
        }
    }

    for path in tealdeer_cache_paths(tool_name) {
        match std::fs::read_to_string(&path) {
            Ok(content) => return Some(content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => continue,
        }
    }
    None
}

fn save_to_cache(tool_name: &str, content: &str) {
    let Some(path) = lazypkg_cache_path(tool_name) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, content);
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
        let text = response.text()?;
        save_to_cache(tool_name, &text);
        return Ok(Some(text));
    }

    // Try linux page
    let url = format!(
        "https://raw.githubusercontent.com/tldr-pages/tldr/main/pages/linux/{}.md",
        tool_name
    );
    let response = client.get(&url).send()?;
    if response.status().is_success() {
        let text = response.text()?;
        save_to_cache(tool_name, &text);
        return Ok(Some(text));
    }

    Ok(None)
}
