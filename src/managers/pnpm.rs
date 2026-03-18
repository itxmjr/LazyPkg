use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

use super::{PackageManager, Tool};

pub struct PnpmManager;

impl PnpmManager {
    pub fn new() -> Self {
        PnpmManager
    }

    fn parse_json(json: &str) -> Result<Vec<Tool>> {
        #[derive(Deserialize)]
        struct PnpmDep {
            version: Option<String>,
        }

        #[derive(Deserialize)]
        struct PnpmEntry {
            #[serde(default)]
            dependencies: HashMap<String, PnpmDep>,
        }

        let entries: Vec<PnpmEntry> =
            serde_json::from_str(json).context("failed to parse pnpm JSON")?;

        let deps = entries
            .into_iter()
            .next()
            .map(|e| e.dependencies)
            .unwrap_or_default();

        let mut tools: Vec<Tool> = deps
            .into_iter()
            .map(|(name, dep)| Tool {
                name,
                version: dep.version,
                manager: "pnpm".to_string(),
            })
            .collect();

        tools.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tools)
    }
}

impl PackageManager for PnpmManager {
    fn name(&self) -> &str {
        "pnpm"
    }

    fn icon(&self) -> &str {
        "📦"
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("pnpm")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        let output = std::process::Command::new("pnpm")
            .args(["list", "-g", "--json"])
            .output()
            .context("failed to run pnpm")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }
        Self::parse_json(&stdout)
    }

    fn uninstall(&self, tool: &Tool) -> Result<()> {
        let status = std::process::Command::new("pnpm")
            .args(["remove", "-g", &tool.name])
            .status()
            .context("failed to run pnpm remove")?;
        if !status.success() {
            anyhow::bail!("pnpm remove {} failed with status {}", tool.name, status);
        }
        Ok(())
    }

    fn install(&self, name: &str) -> Result<()> {
        let status = std::process::Command::new("pnpm")
            .args(["add", "-g", name])
            .status()
            .context("failed to run pnpm add")?;
        if !status.success() {
            anyhow::bail!("pnpm add {} failed with status {}", name, status);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pnpm_json_basic() {
        let json = r#"[
  {
    "name": "root",
    "version": "0.0.1",
    "dependencies": {
      "typescript": {"version": "5.4.5"},
      "prettier": {"version": "3.2.5"}
    }
  }
]"#;
        let tools = PnpmManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 2);
        // sorted
        assert_eq!(tools[0].name, "prettier");
        assert_eq!(tools[0].version, Some("3.2.5".to_string()));
        assert_eq!(tools[1].name, "typescript");
        assert_eq!(tools[1].version, Some("5.4.5".to_string()));
    }

    #[test]
    fn test_parse_pnpm_json_empty_array() {
        let json = "[]";
        let tools = PnpmManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 0);
    }

    #[test]
    fn test_parse_pnpm_json_empty_deps() {
        let json = r#"[{"name": "root", "version": "0.0.1", "dependencies": {}}]"#;
        let tools = PnpmManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 0);
    }

    #[test]
    fn test_parse_pnpm_json_missing_version() {
        let json = r#"[{"dependencies": {"sometool": {}}}]"#;
        let tools = PnpmManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "sometool");
        assert_eq!(tools[0].version, None);
    }
}
