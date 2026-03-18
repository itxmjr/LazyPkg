use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

use super::{PackageManager, Tool};

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
        std::process::Command::new("pipx")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        let output = std::process::Command::new("pipx")
            .args(["list", "--json"])
            .output()
            .context("failed to run pipx")?;

        if !output.status.success() && output.stdout.is_empty() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }
        Self::parse_json(&stdout)
    }

    fn uninstall(&self, tool: &Tool) -> Result<()> {
        let status = std::process::Command::new("pipx")
            .args(["uninstall", &tool.name])
            .status()
            .context("failed to run pipx uninstall")?;
        if !status.success() {
            anyhow::bail!("pipx uninstall {} failed with status {}", tool.name, status);
        }
        Ok(())
    }

    fn install(&self, name: &str) -> Result<()> {
        let status = std::process::Command::new("pipx")
            .args(["install", name])
            .status()
            .context("failed to run pipx install")?;
        if !status.success() {
            anyhow::bail!("pipx install {} failed with status {}", name, status);
        }
        Ok(())
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
