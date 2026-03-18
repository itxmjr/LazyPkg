use anyhow::{Context, Result};
use serde::Deserialize;

use super::{PackageManager, Tool};

pub struct PipManager;

impl PipManager {
    pub fn new() -> Self {
        PipManager
    }

    /// Return the first pip binary found (pip, then pip3).
    fn pip_binary() -> &'static str {
        // We check availability at runtime; for command construction we try pip first.
        // The is_available() method handles the fallback check.
        "pip"
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
        let pip_ok = std::process::Command::new("pip")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if pip_ok {
            return true;
        }
        std::process::Command::new("pip3")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        // Try pip first, fall back to pip3
        match Self::run_pip_list(Self::pip_binary()) {
            Ok(tools) => Ok(tools),
            Err(_) => Self::run_pip_list("pip3"),
        }
    }

    fn uninstall(&self, tool: &Tool) -> Result<()> {
        let try_uninstall = |binary: &str| -> Result<bool> {
            let status = std::process::Command::new(binary)
                .args(["uninstall", "-y", &tool.name])
                .status()
                .with_context(|| format!("failed to run {} uninstall", binary))?;
            Ok(status.success())
        };

        if try_uninstall("pip")? {
            return Ok(());
        }
        let ok = try_uninstall("pip3")?;
        if !ok {
            anyhow::bail!("pip uninstall {} failed", tool.name);
        }
        Ok(())
    }

    fn install(&self, name: &str) -> Result<()> {
        let status = std::process::Command::new("pip")
            .args(["install", "--user", name])
            .status()
            .context("failed to run pip install")?;
        if !status.success() {
            anyhow::bail!("pip install {} failed with status {}", name, status);
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
