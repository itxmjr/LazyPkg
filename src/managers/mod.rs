use anyhow::Result;
use std::sync::Arc;

use crate::exec::Cmd;

pub mod cargo;
pub mod dnf;
pub mod npm;
pub mod pip;
pub mod pipx;
pub mod pnpm;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tool {
    pub name: String,
    pub version: Option<String>,
    pub manager: String,
}

/// A package manager lazypkg can inspect and drive.
///
/// Managers never run install/uninstall themselves: they *describe* the command
/// (`install_cmd` / `uninstall_cmd`) and the caller decides how to run it
/// (captured in the background, or attached to the terminal when it needs root
/// or prints progress). This keeps elevation explicit and output off the TUI.
pub trait PackageManager: Send + Sync {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    /// Cheap check (no process spawn) for whether this manager exists here.
    fn is_available(&self) -> bool;
    fn list_installed(&self) -> Result<Vec<Tool>>;
    fn install_cmd(&self, name: &str) -> Cmd;
    fn uninstall_cmd(&self, tool: &Tool) -> Cmd;
    /// System managers provide runtimes (node, python, cargo…) that language
    /// managers depend on, so they are installed first during import.
    fn is_system(&self) -> bool {
        false
    }
    /// Canonical form used to compare names (e.g. snapshot vs installed).
    fn normalize_name(&self, name: &str) -> String {
        name.to_string()
    }
}

pub fn all_managers() -> Vec<Arc<dyn PackageManager>> {
    vec![
        Arc::new(cargo::CargoManager::new()),
        Arc::new(dnf::DnfManager::new()),
        Arc::new(pipx::PipxManager::new()),
        Arc::new(pip::PipManager::new()),
        Arc::new(npm::NpmManager::new()),
        Arc::new(pnpm::PnpmManager::new()),
    ]
}

/// Check that a package name (e.g. from a snapshot file) is safe to pass to a
/// package manager. Names are always passed as a single argument, never via a
/// shell, but a leading `-` would still be read as an *option*
/// (`--index-url=https://evil`), so it is rejected.
pub fn validate_package_name(name: &str) -> std::result::Result<(), &'static str> {
    if name.is_empty() {
        return Err("empty name");
    }
    if name.starts_with('-') {
        return Err("names may not start with '-'");
    }
    if name.len() > 214 {
        return Err("name is too long");
    }
    if name.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("names may not contain spaces or control characters");
    }
    Ok(())
}

/// Run a listing command and return stdout, tolerating a non-zero exit when the
/// tool still printed output (npm/pnpm do this when a global package is broken).
pub(crate) fn list_output(cmd: &Cmd) -> Result<String> {
    let out = crate::exec::run_captured(cmd, Some(crate::exec::LIST_TIMEOUT))?;
    if !out.status.success() && out.stdout.trim().is_empty() {
        let err = out
            .stderr
            .lines()
            .map(str::trim)
            .rfind(|l| !l.is_empty())
            .unwrap_or("no output");
        anyhow::bail!("`{}` failed: {}", cmd.display(), err);
    }
    Ok(out.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_validation() {
        for ok in [
            "ripgrep",
            "@biomejs/biome",
            "python3-requests",
            "black[d]",
            "g++",
            "pkg==1.0",
        ] {
            assert!(validate_package_name(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "-rf",
            "--index-url=https://evil",
            "two words",
            "tab\tname",
            "nl\nname",
        ] {
            assert!(validate_package_name(bad).is_err(), "{bad:?}");
        }
    }
}
