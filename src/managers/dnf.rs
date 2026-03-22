use anyhow::{Context, Result};

use super::{PackageManager, Tool};

pub struct DnfManager;

impl DnfManager {
    pub fn new() -> Self {
        DnfManager
    }

    /// Parse output of `dnf repoquery --userinstalled --qf "%{name} %{version}\n"`.
    fn parse_output(output: &str) -> Vec<Tool> {
        let mut tools = Vec::new();
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("Updating and loading repositories:") || line.starts_with("Repositories loaded.") {
                continue;
            }

            let mut cols = line.split_whitespace();
            let name = match cols.next() {
                Some(s) => s.to_string(),
                None => continue,
            };
            let version = cols.next().map(|v| v.to_string());

            tools.push(Tool {
                name,
                version,
                manager: "dnf".to_string(),
            });
        }
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
    }
}

impl PackageManager for DnfManager {
    fn name(&self) -> &str {
        "dnf"
    }

    fn icon(&self) -> &str {
        "📦"
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("dnf")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        let output = std::process::Command::new("dnf")
            .args(["repoquery", "--userinstalled", "--qf", "%{name} %{version}\\n"])
            .output()
            .context("failed to run dnf")?;

        if !output.status.success() && output.stdout.is_empty() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_output(&stdout))
    }

    fn uninstall(&self, tool: &Tool) -> Result<()> {
        let status = std::process::Command::new("dnf")
            .args(["remove", "-y", &tool.name])
            .status()
            .context("failed to run dnf remove")?;
        if !status.success() {
            anyhow::bail!("dnf remove {} failed with status {}", tool.name, status);
        }
        Ok(())
    }

    fn install(&self, name: &str) -> Result<()> {
        let status = std::process::Command::new("dnf")
            .args(["install", "-y", name])
            .status()
            .context("failed to run dnf install")?;
        if !status.success() {
            anyhow::bail!("dnf install {} failed with status {}", name, status);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dnf_output_basic() {
        let input = "bat 0.24.0-3.fc41\nripgrep 14.1.1-1.fc41\n";
        let tools = DnfManager::parse_output(input);
        assert_eq!(tools.len(), 2);
        // sorted
        assert_eq!(tools[0].name, "bat");
        assert_eq!(tools[0].version, Some("0.24.0-3.fc41".to_string()));
        assert_eq!(tools[1].name, "ripgrep");
        assert_eq!(tools[1].version, Some("14.1.1-1.fc41".to_string()));
    }

    #[test]
    fn test_parse_dnf_empty_output() {
        let tools = DnfManager::parse_output("");
        assert_eq!(tools.len(), 0);
    }
}
