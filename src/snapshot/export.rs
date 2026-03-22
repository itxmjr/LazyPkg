use crate::managers::all_managers;
use crate::snapshot::{ManagerSnapshot, Snapshot, SnapshotMeta};
use anyhow::Result;
use std::path::PathBuf;

pub fn default_snapshot_path() -> PathBuf {
    // ~/.config/lazypkg/snapshot.toml
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
        });
    config_dir.join("lazypkg").join("snapshot.toml")
}

pub fn export_snapshot() -> Result<PathBuf> {
    let managers = all_managers();
    let mut packages = std::collections::HashMap::new();

    for manager in &managers {
        if manager.is_available() {
            if let Ok(tools) = manager.list_installed() {
                packages.insert(
                    manager.name().to_string(),
                    ManagerSnapshot {
                        tools: tools.iter().map(|t| t.name.clone()).collect(),
                    },
                );
            }
        }
    }

    let snapshot = Snapshot {
        meta: SnapshotMeta {
            date: format_now(),
            hostname: get_hostname(),
            lazypkg_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        packages,
    };

    let path = default_snapshot_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let toml_str = toml::to_string_pretty(&snapshot)?;
    std::fs::write(&path, toml_str)?;
    Ok(path)
}

fn format_now() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

fn get_hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}
