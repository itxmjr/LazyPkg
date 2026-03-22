use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

use super::{PackageManager, Tool};

pub struct NpmManager;

impl NpmManager {
    pub fn new() -> Self {
        NpmManager
    }

    fn parse_json(json: &str) -> Result<Vec<Tool>> {
        #[derive(Deserialize)]
        struct NpmDep {
            version: Option<String>,
        }

        #[derive(Deserialize)]
        struct NpmList {
            #[serde(default)]
            dependencies: HashMap<String, NpmDep>,
        }

        let list: NpmList = serde_json::from_str(json).context("failed to parse npm JSON")?;

        let mut tools: Vec<Tool> = list
            .dependencies
            .into_iter()
            .map(|(name, dep)| Tool {
                name,
                version: dep.version,
                manager: "npm".to_string(),
            })
            .collect();

        tools.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tools)
    }
}

impl PackageManager for NpmManager {
    fn name(&self) -> &str {
        "npm"
    }

    fn icon(&self) -> &str {
        "📦"
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("npm")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        let output = std::process::Command::new("npm")
            .args(["list", "-g", "--json", "--depth=0"])
            .output()
            .context("failed to run npm")?;

        // npm list -g --json can exit non-zero when some packages have issues
        // but still produce valid JSON on stdout — try to parse regardless
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }
        Self::parse_json(&stdout)
    }

    fn uninstall(&self, tool: &Tool) -> Result<()> {
        let status = std::process::Command::new("npm")
            .args(["uninstall", "-g", &tool.name])
            .status()
            .context("failed to run npm uninstall")?;
        if !status.success() {
            let status_pkexec = std::process::Command::new("pkexec")
                .args(["npm", "uninstall", "-g", &tool.name])
                .status()
                .context("failed to run pkexec npm uninstall")?;
            if !status_pkexec.success() {
                anyhow::bail!("npm uninstall {} failed", tool.name);
            }
        }
        Ok(())
    }

    fn install(&self, name: &str) -> Result<()> {
        let status = std::process::Command::new("npm")
            .args(["install", "-g", name])
            .status()
            .context("failed to run npm install")?;
        if !status.success() {
            anyhow::bail!("npm install {} failed with status {}", name, status);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_npm_json_basic() {
        let json = r#"
{
  "dependencies": {
    "typescript": {"version": "5.4.5"},
    "yarn": {"version": "1.22.22"},
    "eslint": {"version": "9.0.0"}
  }
}
"#;
        let tools = NpmManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 3);
        // sorted
        assert_eq!(tools[0].name, "eslint");
        assert_eq!(tools[0].version, Some("9.0.0".to_string()));
        assert_eq!(tools[1].name, "typescript");
        assert_eq!(tools[2].name, "yarn");
        assert_eq!(tools[2].version, Some("1.22.22".to_string()));
    }

    #[test]
    fn test_parse_npm_json_empty_deps() {
        let json = r#"{"dependencies": {}}"#;
        let tools = NpmManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 0);
    }

    #[test]
    fn test_parse_npm_json_no_deps_key() {
        // npm list -g with nothing installed may omit the key
        let json = r#"{}"#;
        let tools = NpmManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 0);
    }

    #[test]
    fn test_parse_npm_json_missing_version() {
        let json = r#"{"dependencies": {"sometool": {}}}"#;
        let tools = NpmManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "sometool");
        assert_eq!(tools[0].version, None);
    }
}
