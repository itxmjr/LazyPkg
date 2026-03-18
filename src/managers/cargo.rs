use anyhow::{Context, Result};
use std::path::PathBuf;

use super::{PackageManager, Tool};

pub struct CargoManager;

impl CargoManager {
    pub fn new() -> Self {
        CargoManager
    }

    fn crates_toml_path() -> Option<PathBuf> {
        // First try CARGO_HOME
        if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
            return Some(std::path::PathBuf::from(cargo_home).join(".crates.toml"));
        }
        // Fall back to HOME
        std::env::var("HOME").ok()
            .map(|h| std::path::PathBuf::from(h).join(".cargo").join(".crates.toml"))
    }

    /// Parse a crates.toml key of the form `"name version (registry...)"` into (name, version).
    fn parse_crate_key(key: &str) -> Option<(String, Option<String>)> {
        let mut parts = key.splitn(3, ' ');
        let name = parts.next()?.to_string();
        let version = parts.next().map(|v| v.to_string());
        Some((name, version))
    }
}

impl PackageManager for CargoManager {
    fn name(&self) -> &str {
        "cargo"
    }

    fn icon(&self) -> &str {
        "🦀"
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("cargo")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        let path = Self::crates_toml_path()
            .context("Could not determine HOME directory")?;

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(anyhow::Error::from(e).context(format!("Failed to read {}", path.display()))),
        };

        let doc: toml::Value = contents
            .parse::<toml::Value>()
            .context("Failed to parse .crates.toml")?;

        let v1 = match doc.get("v1").and_then(|v| v.as_table()) {
            Some(t) => t,
            None => return Ok(vec![]),
        };

        let mut tools = Vec::new();
        for key in v1.keys() {
            if let Some((name, version)) = Self::parse_crate_key(key) {
                tools.push(Tool {
                    name,
                    version,
                    manager: "cargo".to_string(),
                });
            }
        }

        tools.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tools)
    }

    fn uninstall(&self, tool: &Tool) -> Result<()> {
        let status = std::process::Command::new("cargo")
            .args(["uninstall", &tool.name])
            .status()
            .context("Failed to run cargo uninstall")?;
        if !status.success() {
            anyhow::bail!("cargo uninstall {} failed with status {}", tool.name, status);
        }
        Ok(())
    }

    fn install(&self, name: &str) -> Result<()> {
        let status = std::process::Command::new("cargo")
            .args(["install", name])
            .status()
            .context("Failed to run cargo install")?;
        if !status.success() {
            anyhow::bail!("cargo install {} failed with status {}", name, status);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_crate_key_with_registry() {
        let key = "tokei 12.1.2 (registry+https://github.com/rust-lang/crates.io-index)";
        let (name, version) = CargoManager::parse_crate_key(key).unwrap();
        assert_eq!(name, "tokei");
        assert_eq!(version, Some("12.1.2".to_string()));
    }

    #[test]
    fn test_parse_crate_key_ripgrep() {
        let key = "ripgrep 14.1.1 (registry+https://github.com/rust-lang/crates.io-index)";
        let (name, version) = CargoManager::parse_crate_key(key).unwrap();
        assert_eq!(name, "ripgrep");
        assert_eq!(version, Some("14.1.1".to_string()));
    }

    #[test]
    fn test_parse_crate_key_no_registry() {
        // Minimal key with just name and version
        let key = "mytool 1.0.0";
        let (name, version) = CargoManager::parse_crate_key(key).unwrap();
        assert_eq!(name, "mytool");
        assert_eq!(version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_crate_key_name_only() {
        let key = "sometool";
        let (name, version) = CargoManager::parse_crate_key(key).unwrap();
        assert_eq!(name, "sometool");
        assert_eq!(version, None);
    }

    #[test]
    fn test_list_installed_parses_toml() {
        let toml_content = r#"
[v1]
"tokei 12.1.2 (registry+https://github.com/rust-lang/crates.io-index)" = []
"ripgrep 14.1.1 (registry+https://github.com/rust-lang/crates.io-index)" = ["dep1"]
"#;
        let doc: toml::Value = toml_content.parse().unwrap();
        let v1 = doc.get("v1").and_then(|v| v.as_table()).unwrap();

        let mut tools: Vec<(String, Option<String>)> = v1
            .keys()
            .filter_map(|k| CargoManager::parse_crate_key(k))
            .collect();
        tools.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].0, "ripgrep");
        assert_eq!(tools[0].1, Some("14.1.1".to_string()));
        assert_eq!(tools[1].0, "tokei");
        assert_eq!(tools[1].1, Some("12.1.2".to_string()));
    }
}
