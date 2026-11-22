use anyhow::Result;

pub mod help;
pub mod tldr;

pub trait CheatsheetProvider {
    fn fetch(&self, tool_name: &str) -> Result<Option<String>>;
}

/// Turn a package name into a name that is safe to use as a tldr page name,
/// a cache file name and a command name.
///
/// - npm scoped packages (`@scope/pkg`) map to `pkg`.
/// - Anything containing path separators, `..`, or unusual characters is
///   rejected so it can never escape the cache directory or form a bad URL.
pub fn page_name(tool_name: &str) -> Option<String> {
    let name = match tool_name.strip_prefix('@') {
        Some(scoped) => scoped.split_once('/')?.1,
        None => tool_name,
    };
    let valid = !name.is_empty()
        && name.len() <= 100
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'));
    valid.then(|| name.to_string())
}

/// Load cheatsheet for a tool: tries tldr first, falls back to --help
pub fn load_cheatsheet(tool_name: &str) -> Option<String> {
    let name = page_name(tool_name)?;

    let tldr = tldr::TldrProvider::new();
    let tldr_error = match tldr.fetch(&name) {
        Ok(Some(content)) => return Some(content),
        Ok(None) => None, // No tldr page, try --help
        Err(e) => Some(e),
    };

    let help = help::HelpProvider::new();
    match (help.fetch(&name), tldr_error) {
        (Ok(Some(content)), _) => Some(content),
        (_, Some(e)) => Some(format!(
            "[tldr error: {}]\n\nNo --help output found either.",
            e
        )),
        (Ok(None), None) => None,
        (Err(e), None) => Some(format!("Error loading cheatsheet: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::page_name;

    #[test]
    fn page_names() {
        assert_eq!(page_name("ripgrep").as_deref(), Some("ripgrep"));
        assert_eq!(page_name("@biomejs/biome").as_deref(), Some("biome"));
        assert_eq!(page_name("g++").as_deref(), Some("g++"));
        assert_eq!(page_name("../../etc/passwd"), None);
        assert_eq!(page_name("a/b"), None);
        assert_eq!(page_name("@scope"), None);
        assert_eq!(page_name(".hidden"), None);
        assert_eq!(page_name(""), None);
        assert_eq!(page_name("name with space"), None);
    }
}
