use crate::managers::all_managers;
use crate::snapshot::Snapshot;
use anyhow::Result;
use std::path::PathBuf;

pub struct ImportDiff {
    pub manager: String,
    pub missing_tools: Vec<String>,
}

pub fn read_snapshot(path: &PathBuf) -> Result<Snapshot> {
    let content = std::fs::read_to_string(path)?;
    let snapshot: Snapshot = toml::from_str(&content)?;
    Ok(snapshot)
}

pub fn compute_diff(snapshot: &Snapshot) -> Result<Vec<ImportDiff>> {
    let managers = all_managers();
    let mut diffs = Vec::new();

    for (manager_name, manager_snap) in &snapshot.packages {
        // Find the matching manager
        if let Some(manager) = managers.iter().find(|m| m.name() == manager_name) {
            if manager.is_available() {
                let installed = manager
                    .list_installed()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| t.name)
                    .collect::<std::collections::HashSet<_>>();

                let missing: Vec<String> = manager_snap
                    .tools
                    .iter()
                    .filter(|t| !installed.contains(*t))
                    .cloned()
                    .collect();

                if !missing.is_empty() {
                    diffs.push(ImportDiff {
                        manager: manager_name.clone(),
                        missing_tools: missing,
                    });
                }
            }
        }
    }

    Ok(diffs)
}

pub fn install_missing(diffs: &[ImportDiff]) -> Result<Vec<String>> {
    let managers = all_managers();
    let mut installed = Vec::new();
    let mut errors = Vec::new();

    for diff in diffs {
        if let Some(manager) = managers.iter().find(|m| m.name() == diff.manager) {
            for tool in &diff.missing_tools {
                match manager.install(tool) {
                    Ok(()) => installed.push(format!("{}/{}", diff.manager, tool)),
                    Err(e) => errors.push(format!("{}/{}: {}", diff.manager, tool, e)),
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(anyhow::anyhow!(
            "Some installs failed:\n{}",
            errors.join("\n")
        ));
    }

    Ok(installed)
}
