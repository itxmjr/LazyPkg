use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

use super::{list_output, PackageManager, Tool};
use crate::exec::Cmd;

pub struct PipxManager;

impl PipxManager {
    pub fn new() -> Self {
        PipxManager
    }

    fn parse_json(json: &str) -> Result<Vec<Tool>> {
        #[derive(Deserialize)]
        struct MainPackage {
            package_version: Option<String>,
        }

        #[derive(Deserialize)]
        struct Metadata {
            main_package: Option<MainPackage>,
        }

        #[derive(Deserialize)]
        struct Venv {
            metadata: Option<Metadata>,
        }

        #[derive(Deserialize)]
        struct PipxList {
            venvs: HashMap<String, Venv>,
        }

        let list: PipxList = serde_json::from_str(json).context("failed to parse pipx JSON")?;

        let mut tools: Vec<Tool> = list
            .venvs
            .into_iter()
            .map(|(name, venv)| {
                let version = venv
                    .metadata
                    .and_then(|m| m.main_package)
                    .and_then(|p| p.package_version);
                Tool {
                    name,
                    version,
                    manager: "pipx".to_string(),
                }
            })
            .collect();

        tools.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tools)
    }
}

impl PackageManager for PipxManager {
    fn name(&self) -> &str {
        "pipx"
    }

    fn icon(&self) -> &str {
        "🐍"
    }

    fn is_available(&self) -> bool {
        crate::exec::which("pipx").is_some()
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        let stdout = list_output(&Cmd::new("pipx", &["list", "--json"]))?;
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }
        Self::parse_json(&stdout)
    }

    fn uninstall_cmd(&self, tool: &Tool) -> Cmd {
        Cmd::new("pipx", &["uninstall", tool.name.as_str()])
    }

    fn install_cmd(&self, name: &str) -> Cmd {
        Cmd::new("pipx", &["install", name])
    }

    fn normalize_name(&self, name: &str) -> String {
        super::pip::normalize_python_name(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pipx_json_basic() {
        let json = r#"
{
  "venvs": {
    "black": {
      "metadata": {
        "main_package": {
          "package": "black",
          "package_version": "24.4.0"
        }
      }
    },
    "httpie": {
      "metadata": {
        "main_package": {
          "package": "httpie",
          "package_version": "3.2.2"
        }
      }
    }
  }
}
"#;
        let tools = PipxManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 2);
        // sorted
        assert_eq!(tools[0].name, "black");
        assert_eq!(tools[0].version, Some("24.4.0".to_string()));
        assert_eq!(tools[1].name, "httpie");
        assert_eq!(tools[1].version, Some("3.2.2".to_string()));
    }

    #[test]
    fn test_parse_pipx_json_empty_venvs() {
        let json = r#"{"venvs": {}}"#;
        let tools = PipxManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 0);
    }

    #[test]
    fn test_parse_pipx_json_missing_version() {
        let json = r#"
{
  "venvs": {
    "sometool": {
      "metadata": {
        "main_package": {
          "package": "sometool"
        }
      }
    }
  }
}
"#;
        let tools = PipxManager::parse_json(json).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "sometool");
        assert_eq!(tools[0].version, None);
    }
}
