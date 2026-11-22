use anyhow::{Context, Result};
use serde::Deserialize;

use super::{list_output, PackageManager, Tool};
use crate::exec::Cmd;

pub struct PipManager;

impl PipManager {
    pub fn new() -> Self {
        PipManager
    }

    /// Find which pip binary is available (pip or pip3), without spawning it.
    fn find_pip_binary() -> Option<&'static str> {
        ["pip", "pip3"]
            .into_iter()
            .find(|binary| crate::exec::which(binary).is_some())
    }

    fn parse_json(json: &str) -> Result<Vec<Tool>> {
        #[derive(Deserialize)]
        struct PipPackage {
            name: String,
            version: String,
        }

        let packages: Vec<PipPackage> =
            serde_json::from_str(json).context("failed to parse pip JSON")?;

        let mut tools: Vec<Tool> = packages
            .into_iter()
            .map(|p| Tool {
                name: p.name,
                version: Some(p.version),
                manager: "pip".to_string(),
            })
            .collect();

        tools.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tools)
    }

    fn run_pip_list(binary: &str) -> Result<Vec<Tool>> {
        let stdout = list_output(&Cmd::new(binary, &["list", "--format=json", "--user"]))?;
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }
        Self::parse_json(&stdout)
    }
}

/// PEP 503 name normalisation: case-insensitive, and runs of `-`, `_`, `.`
/// are equivalent (`PyYAML` == `pyyaml`, `ruamel.yaml` == `ruamel-yaml`).
pub fn normalize_python_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_sep = false;
    for c in name.chars() {
        if matches!(c, '-' | '_' | '.') {
            if !last_sep {
                out.push('-');
            }
            last_sep = true;
        } else {
            out.extend(c.to_lowercase());
            last_sep = false;
        }
    }
    out
}

impl PackageManager for PipManager {
    fn name(&self) -> &str {
        "pip"
    }

    fn icon(&self) -> &str {
        "🐍"
    }

    fn is_available(&self) -> bool {
        Self::find_pip_binary().is_some()
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        let binary = Self::find_pip_binary().unwrap_or("pip");
        Self::run_pip_list(binary)
    }

    fn uninstall_cmd(&self, tool: &Tool) -> Cmd {
        let binary = Self::find_pip_binary().unwrap_or("pip");
        Cmd::new(binary, &["uninstall", "-y", tool.name.as_str()])
    }

    fn install_cmd(&self, name: &str) -> Cmd {
        // Note: on PEP 668 distros (Arch, Debian 12+, Fedora 38+) this fails with
        // "externally-managed-environment"; exec adds a hint pointing to pipx/uv.
        let binary = Self::find_pip_binary().unwrap_or("pip");
        Cmd::new(binary, &["install", "--user", name])
    }

    fn normalize_name(&self, name: &str) -> String {
        normalize_python_name(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pip_json_basic() {
        let json = r#"[
  {"name": "requests", "version": "2.31.0"},
  {"name": "black", "version": "24.4.0"},
  {"name": "ansible", "version": "9.0.0"}
]"#;
        let tools = PipManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 3);
        // sorted
        assert_eq!(tools[0].name, "ansible");
        assert_eq!(tools[0].version, Some("9.0.0".to_string()));
        assert_eq!(tools[1].name, "black");
        assert_eq!(tools[2].name, "requests");
        assert_eq!(tools[2].version, Some("2.31.0".to_string()));
    }

    #[test]
    fn test_normalize_python_name() {
        assert_eq!(normalize_python_name("PyYAML"), "pyyaml");
        assert_eq!(normalize_python_name("ruamel.yaml"), "ruamel-yaml");
        assert_eq!(normalize_python_name("Foo__Bar-.baz"), "foo-bar-baz");
    }

    #[test]
    fn test_parse_pip_json_empty() {
        let json = "[]";
        let tools = PipManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 0);
    }

    #[test]
    fn test_parse_pip_json_single() {
        let json = r#"[{"name": "mycli", "version": "0.1.0"}]"#;
        let tools = PipManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "mycli");
        assert_eq!(tools[0].manager, "pip");
    }
}
