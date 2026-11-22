use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

use super::{list_output, PackageManager, Tool};
use crate::exec::Cmd;

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
        crate::exec::which("pnpm").is_some()
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        // pnpm may print valid JSON even on a non-zero exit; list_output tolerates that.
        let stdout = list_output(&Cmd::new("pnpm", &["list", "-g", "--json"]))?;
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }
        Self::parse_json(&stdout)
    }

    fn uninstall_cmd(&self, tool: &Tool) -> Cmd {
        Cmd::new("pnpm", &["remove", "-g", tool.name.as_str()])
    }

    fn install_cmd(&self, name: &str) -> Cmd {
        Cmd::new("pnpm", &["add", "-g", name])
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
