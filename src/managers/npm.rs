use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use super::{list_output, PackageManager, Tool};
use crate::exec::{self, Cmd};

pub struct NpmManager;

impl NpmManager {
    pub fn new() -> Self {
        NpmManager
    }

    /// Global installs need root only when the global node_modules directory is
    /// not writable by the current user (e.g. a distro-packaged node with prefix
    /// /usr). With nvm, fnm, volta or a user prefix, no elevation is needed.
    fn global_needs_root() -> bool {
        let Ok(out) = exec::run_captured(
            &Cmd::new("npm", &["root", "-g"]),
            Some(Duration::from_secs(15)),
        ) else {
            return false;
        };
        let root = PathBuf::from(out.stdout.trim());
        if !out.status.success() || !root.is_absolute() {
            return false;
        }
        // If node_modules doesn't exist yet, npm will create it inside its parent.
        let probe = if root.exists() {
            root
        } else {
            match root.parent() {
                Some(p) => p.to_path_buf(),
                None => return false,
            }
        };
        !exec::is_writable(&probe)
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
        crate::exec::which("npm").is_some()
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        // npm list -g --json can exit non-zero when some packages have issues
        // but still produce valid JSON on stdout — list_output tolerates that.
        let stdout = list_output(&Cmd::new("npm", &["list", "-g", "--json", "--depth=0"]))?;
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }
        Self::parse_json(&stdout)
    }

    fn uninstall_cmd(&self, tool: &Tool) -> Cmd {
        Cmd::new("npm", &["uninstall", "-g", tool.name.as_str()])
            .with_root(Self::global_needs_root())
    }

    fn install_cmd(&self, name: &str) -> Cmd {
        Cmd::new("npm", &["install", "-g", name]).with_root(Self::global_needs_root())
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
