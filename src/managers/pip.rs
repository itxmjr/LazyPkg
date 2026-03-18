use anyhow::{Context, Result};
use serde::Deserialize;

use super::{PackageManager, Tool};

pub struct PipManager;

impl PipManager {
    pub fn new() -> Self {
        PipManager
    }

    /// Find which pip binary is available (pip or pip3).
    fn find_pip_binary() -> Option<&'static str> {
        for binary in ["pip", "pip3"] {
            if std::process::Command::new(binary)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(binary);
            }
        }
        None
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
        let output = std::process::Command::new(binary)
            .args(["list", "--format=json", "--user"])
            .output()
            .with_context(|| format!("failed to run {}", binary))?;

        if !output.status.success() && output.stdout.is_empty() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }
        Self::parse_json(&stdout)
    }
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

    fn uninstall(&self, tool: &Tool) -> Result<()> {
        let binary = Self::find_pip_binary().unwrap_or("pip");
        let status = std::process::Command::new(binary)
            .args(["uninstall", "-y", &tool.name])
            .status()
            .with_context(|| format!("failed to run {} uninstall", binary))?;
        if !status.success() {
            anyhow::bail!("pip uninstall {} failed", tool.name);
        }
        Ok(())
    }

    fn install(&self, name: &str) -> Result<()> {
        let binary = Self::find_pip_binary().unwrap_or("pip");
        let status = std::process::Command::new(binary)
            .args(["install", "--user", name])
            .status()
            .with_context(|| format!("failed to run {} install", binary))?;
        if !status.success() {
            anyhow::bail!("{} install {} failed with status {}", binary, name, status);
        }
        Ok(())
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
