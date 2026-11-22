use crate::exec::{self, Cmd};
use crate::managers::{all_managers, PackageManager};
use crate::snapshot::Snapshot;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

pub struct ImportDiff {
    pub manager: String,
    pub missing_tools: Vec<String>,
}

/// What `import` will do. Nothing is silently dropped: every manager in the
/// snapshot ends up either in `diffs` (something to install), in `skipped`
/// with a reason, or is already fully installed.
pub struct ImportPlan {
    /// System managers first: they provide runtimes language managers need.
    pub diffs: Vec<ImportDiff>,
    /// (manager, reason)
    pub skipped: Vec<(String, String)>,
}

pub struct InstallSummary {
    pub installed: Vec<String>,
    /// ("manager/tool", error)
    pub failed: Vec<(String, String)>,
}

pub fn read_snapshot(path: &Path) -> Result<Snapshot> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let snapshot: Snapshot =
        toml::from_str(&content).with_context(|| format!("invalid snapshot {}", path.display()))?;
    Ok(snapshot)
}

pub fn compute_plan(snapshot: &Snapshot) -> ImportPlan {
    let managers = all_managers();
    let mut diffs: Vec<(bool, ImportDiff)> = Vec::new();
    let mut skipped = Vec::new();

    for (manager_name, manager_snap) in &snapshot.packages {
        if manager_snap.tools.is_empty() {
            continue;
        }
        let Some(manager) = managers.iter().find(|m| m.name() == manager_name) else {
            skipped.push((
                manager_name.clone(),
                "not supported by this version of lazypkg".to_string(),
            ));
            continue;
        };
        if !manager.is_available() {
            skipped.push((
                manager_name.clone(),
                format!("{} is not installed on this machine", manager_name),
            ));
            continue;
        }
        // If we can't tell what's installed we must not guess: treating the
        // failure as "nothing installed" would reinstall every package.
        let installed = match manager.list_installed() {
            Ok(tools) => tools.into_iter().map(|t| t.name).collect::<Vec<_>>(),
            Err(e) => {
                skipped.push((
                    manager_name.clone(),
                    format!("could not list installed packages: {:#}", e),
                ));
                continue;
            }
        };

        let missing = missing_tools(manager.as_ref(), &manager_snap.tools, &installed);
        if !missing.is_empty() {
            diffs.push((
                manager.is_system(),
                ImportDiff {
                    manager: manager_name.clone(),
                    missing_tools: missing,
                },
            ));
        }
    }

    // Stable sort: system managers first, otherwise keep snapshot order.
    diffs.sort_by_key(|(is_system, _)| !*is_system);
    ImportPlan {
        diffs: diffs.into_iter().map(|(_, d)| d).collect(),
        skipped,
    }
}

/// Tools in `wanted` that aren't in `installed`, comparing normalised names and
/// ignoring blanks and duplicates in the snapshot.
fn missing_tools(
    manager: &dyn PackageManager,
    wanted: &[String],
    installed: &[String],
) -> Vec<String> {
    let installed: HashSet<String> = installed
        .iter()
        .map(|n| manager.normalize_name(n))
        .collect();
    let mut seen = HashSet::new();
    wanted
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .filter(|t| {
            let key = manager.normalize_name(t);
            !installed.contains(&key) && seen.insert(key)
        })
        .map(str::to_string)
        .collect()
}

/// Install everything in `diffs`, attached to the terminal so progress output
/// and sudo prompts work. `before_each` is called with the command about to
/// run. One failure never stops the rest.
pub fn install_missing(
    diffs: &[ImportDiff],
    mut before_each: impl FnMut(&str, &str, &Cmd),
) -> InstallSummary {
    let managers = all_managers();
    let mut summary = InstallSummary {
        installed: Vec::new(),
        failed: Vec::new(),
    };

    for diff in diffs {
        let Some(manager) = managers.iter().find(|m| m.name() == diff.manager) else {
            for tool in &diff.missing_tools {
                summary.failed.push((
                    format!("{}/{}", diff.manager, tool),
                    "unknown package manager".to_string(),
                ));
            }
            continue;
        };
        for tool in &diff.missing_tools {
            let cmd = manager.install_cmd(tool);
            before_each(&diff.manager, tool, &cmd);
            let id = format!("{}/{}", diff.manager, tool);
            match exec::run_interactive(&cmd) {
                Ok(()) => summary.installed.push(id),
                Err(e) => summary.failed.push((id, format!("{:#}", e))),
            }
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::{cargo::CargoManager, pip::PipManager};

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn missing_ignores_installed_blanks_and_duplicates() {
        let m = CargoManager::new();
        let missing = missing_tools(
            &m,
            &s(&["bat", "ripgrep", "bat", " ", "fd"]),
            &s(&["ripgrep"]),
        );
        assert_eq!(missing, s(&["bat", "fd"]));
    }

    #[test]
    fn missing_uses_python_normalisation() {
        let m = PipManager::new();
        let missing = missing_tools(
            &m,
            &s(&["PyYAML", "ruamel.yaml", "black"]),
            &s(&["pyyaml", "ruamel-yaml"]),
        );
        assert_eq!(missing, s(&["black"]));
    }

    #[test]
    fn old_snapshot_format_still_loads() {
        let dir = std::env::temp_dir().join(format!("lazypkg-old-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("snapshot.toml");
        std::fs::write(
            &path,
            "[meta]\ndate = \"1774300000\"\nhostname = \"h\"\nlazypkg_version = \"0.1.0\"\n\n[packages.cargo]\ntools = [\"bat\"]\n",
        )
        .unwrap();
        let snap = read_snapshot(&path).unwrap();
        assert_eq!(snap.packages["cargo"].tools, s(&["bat"]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_manager_is_reported_not_dropped() {
        let mut snap = Snapshot::default();
        snap.packages.insert(
            "definitely-not-a-manager".into(),
            crate::snapshot::ManagerSnapshot { tools: s(&["x"]) },
        );
        let plan = compute_plan(&snap);
        assert!(plan.diffs.is_empty());
        assert_eq!(plan.skipped.len(), 1);
        assert!(plan.skipped[0].1.contains("not supported"));
    }
}
