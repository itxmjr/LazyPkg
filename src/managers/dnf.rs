use anyhow::{Context, Result};

use super::{PackageManager, Tool};

pub struct DnfManager;

impl DnfManager {
    pub fn new() -> Self {
        DnfManager
    }

    /// Parse output of `dnf list installed --quiet`.
    ///
    /// Lines look like:
    ///   bat.x86_64                    0.24.0-3.fc41    @fedora
    ///
    /// We skip header lines and lines without a `.` in the first field.
    fn parse_output(output: &str) -> Vec<Tool> {
        let mut tools = Vec::new();
        for line in output.lines() {
            let line = line.trim();
            // Skip blank lines and known header/info lines
            if line.is_empty()
                || line.starts_with("Last metadata")
                || line.starts_with("Installed Packages")
            {
                continue;
            }

            let mut cols = line.split_whitespace();
            let name_arch = match cols.next() {
                Some(s) => s,
                None => continue,
            };
            let version = cols.next().map(|v| v.to_string());

            // Only accept lines where the first token contains a `.` (name.arch)
            if !name_arch.contains('.') {
                continue;
            }

            let name = match name_arch.split('.').next() {
                Some(n) => n.to_string(),
                None => continue,
            };

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
            .args(["list", "installed", "--quiet"])
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
        let input = "\
Last metadata expiration check: 0:01:23 ago on Wed Mar 18 2026.
Installed Packages
bat.x86_64                    0.24.0-3.fc41    @fedora
ripgrep.x86_64                14.1.1-1.fc41    @fedora
";
        let tools = DnfManager::parse_output(input);
        assert_eq!(tools.len(), 2);
        // sorted
        assert_eq!(tools[0].name, "bat");
        assert_eq!(tools[0].version, Some("0.24.0-3.fc41".to_string()));
        assert_eq!(tools[1].name, "ripgrep");
        assert_eq!(tools[1].version, Some("14.1.1-1.fc41".to_string()));
    }

    #[test]
    fn test_parse_dnf_skips_header_lines() {
        let input = "\
Installed Packages
glibc.x86_64    2.40-1.fc41    @anaconda
bash.x86_64     5.2.37-1.fc41  @anaconda
";
        let tools = DnfManager::parse_output(input);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "bash");
        assert_eq!(tools[1].name, "glibc");
    }

    #[test]
    fn test_parse_dnf_empty_output() {
        let tools = DnfManager::parse_output("");
        assert_eq!(tools.len(), 0);
    }

    #[test]
    fn test_parse_dnf_no_dot_in_first_col_skipped() {
        let input = "SomeHeaderLine without arch column\nbat.x86_64  0.24.0  @fedora\n";
        let tools = DnfManager::parse_output(input);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "bat");
    }
}
