use crate::managers::all_managers;
use crate::snapshot::{ManagerSnapshot, Snapshot, SnapshotMeta};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `~/.config/lazypkg/snapshot.toml`
pub fn default_snapshot_path() -> Result<PathBuf> {
    crate::paths::config_dir()
        .map(|d| d.join("snapshot.toml"))
        .context(
            "cannot locate the config directory: set HOME or XDG_CONFIG_HOME to an absolute path",
        )
}

#[derive(Debug)]
pub struct ExportReport {
    pub path: PathBuf,
    /// (manager, number of packages written)
    pub exported: Vec<(String, usize)>,
    /// Things the user should know about (kept old entries, failed listings…)
    pub warnings: Vec<String>,
    /// Where the previous snapshot was backed up, if there was one.
    pub backup: Option<PathBuf>,
}

/// Result of asking one manager for its packages.
#[derive(Debug)]
pub(crate) enum Listing {
    Listed(Vec<String>),
    Failed(String),
    Unavailable,
}

pub fn export_snapshot() -> Result<ExportReport> {
    export_to(&default_snapshot_path()?)
}

pub fn export_to(path: &Path) -> Result<ExportReport> {
    // Never overwrite a snapshot we can't read: it may be hand-edited with a typo,
    // and silently replacing it would lose data.
    let previous = if path.exists() {
        Some(super::import::read_snapshot(path).with_context(|| {
            format!(
                "the existing snapshot {} could not be read; fix or move it before exporting",
                path.display()
            )
        })?)
    } else {
        None
    };

    let listings = list_all_managers();
    let (packages, exported, warnings) = merge(previous.as_ref(), listings);

    let snapshot = Snapshot {
        meta: SnapshotMeta {
            date: format_rfc3339_utc(std::time::SystemTime::now()),
            hostname: get_hostname(),
            lazypkg_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        packages,
    };

    let toml_str = toml::to_string_pretty(&snapshot)?;
    let backup = write_atomic_with_backup(path, toml_str.as_bytes())?;

    Ok(ExportReport {
        path: path.to_path_buf(),
        exported,
        warnings,
        backup,
    })
}

/// List every known manager in parallel (dnf alone can take seconds).
fn list_all_managers() -> Vec<(String, Listing)> {
    let managers = all_managers();
    std::thread::scope(|s| {
        let handles: Vec<_> = managers
            .iter()
            .map(|m| {
                s.spawn(move || {
                    let listing = if !m.is_available() {
                        Listing::Unavailable
                    } else {
                        match m.list_installed() {
                            Ok(tools) => {
                                Listing::Listed(tools.into_iter().map(|t| t.name).collect())
                            }
                            Err(e) => Listing::Failed(format!("{:#}", e)),
                        }
                    };
                    (m.name().to_string(), listing)
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    })
}

/// (packages to write, (manager, count) exported, warnings)
pub(crate) type Merged = (
    BTreeMap<String, ManagerSnapshot>,
    Vec<(String, usize)>,
    Vec<String>,
);

/// Combine fresh listings with the previous snapshot without ever losing data:
/// a manager that failed to list, or isn't installed on this machine, keeps its
/// previous entries (the snapshot may be shared between machines).
pub(crate) fn merge(previous: Option<&Snapshot>, listings: Vec<(String, Listing)>) -> Merged {
    let mut packages: BTreeMap<String, ManagerSnapshot> =
        previous.map(|p| p.packages.clone()).unwrap_or_default();
    let mut exported = Vec::new();
    let mut warnings = Vec::new();

    for (name, listing) in listings {
        let prev_count = packages.get(&name).map(|m| m.tools.len()).unwrap_or(0);
        match listing {
            Listing::Listed(mut tools) => {
                tools.sort();
                tools.dedup();
                exported.push((name.clone(), tools.len()));
                if tools.is_empty() {
                    packages.remove(&name);
                } else {
                    packages.insert(name, ManagerSnapshot { tools });
                }
            }
            Listing::Failed(err) if prev_count > 0 => warnings.push(format!(
                "{}: listing failed, kept {} entries from the previous snapshot ({})",
                name, prev_count, err
            )),
            Listing::Failed(err) => {
                warnings.push(format!("{}: listing failed, skipped ({})", name, err))
            }
            Listing::Unavailable if prev_count > 0 => warnings.push(format!(
                "{}: not installed here, kept {} entries from the previous snapshot",
                name, prev_count
            )),
            Listing::Unavailable => {}
        }
    }

    (packages, exported, warnings)
}

/// Write `contents` to `path` via a temp file + rename, so a crash or full disk
/// never leaves a half-written snapshot. The old file is copied to `*.bak` first.
fn write_atomic_with_backup(path: &Path, contents: &[u8]) -> Result<Option<PathBuf>> {
    use std::io::Write;

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;

    let backup = if path.exists() {
        let bak = with_suffix(path, ".bak");
        std::fs::copy(path, &bak).with_context(|| {
            format!("failed to back up {} to {}", path.display(), bak.display())
        })?;
        Some(bak)
    } else {
        None
    };

    let tmp = with_suffix(path, ".tmp");
    {
        let mut f = std::fs::File::create(&tmp)
            .with_context(|| format!("failed to create {}", tmp.display()))?;
        f.write_all(contents)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(backup)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Format a time as `YYYY-MM-DDTHH:MM:SSZ` without pulling in a date crate.
pub(crate) fn format_rfc3339_utc(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(unix)]
fn get_hostname() -> String {
    let mut buf = [0u8; 256];
    // SAFETY: buf is valid for buf.len() bytes; gethostname NUL-terminates on success.
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if rc == 0 {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        let name = String::from_utf8_lossy(&buf[..end]).trim().to_string();
        if !name.is_empty() {
            return name;
        }
    }
    "unknown".to_string()
}

#[cfg(not(unix))]
fn get_hostname() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn snap(entries: &[(&str, &[&str])]) -> Snapshot {
        let mut s = Snapshot::default();
        for (m, tools) in entries {
            s.packages.insert(
                m.to_string(),
                ManagerSnapshot {
                    tools: tools.iter().map(|t| t.to_string()).collect(),
                },
            );
        }
        s
    }

    #[test]
    fn rfc3339_formatting() {
        assert_eq!(format_rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        // 2024-02-29T12:34:56Z (leap day)
        let t = UNIX_EPOCH + Duration::from_secs(1_709_210_096);
        assert_eq!(format_rfc3339_utc(t), "2024-02-29T12:34:56Z");
    }

    #[test]
    fn merge_replaces_listed_and_sorts() {
        let prev = snap(&[("cargo", &["old"])]);
        let (pk, exported, warnings) = merge(
            Some(&prev),
            vec![(
                "cargo".into(),
                Listing::Listed(vec!["ripgrep".into(), "bat".into(), "bat".into()]),
            )],
        );
        assert_eq!(pk["cargo"].tools, vec!["bat", "ripgrep"]);
        assert_eq!(exported, vec![("cargo".to_string(), 2)]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn merge_keeps_previous_on_failure() {
        let prev = snap(&[("npm", &["typescript"])]);
        let (pk, _, warnings) = merge(
            Some(&prev),
            vec![("npm".into(), Listing::Failed("boom".into()))],
        );
        assert_eq!(pk["npm"].tools, vec!["typescript"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("kept 1"));
    }

    #[test]
    fn merge_keeps_entries_from_other_machines() {
        let prev = snap(&[("dnf", &["htop"]), ("brew", &["wget"])]);
        let (pk, _, _) = merge(Some(&prev), vec![("dnf".into(), Listing::Unavailable)]);
        assert_eq!(pk["dnf"].tools, vec!["htop"]);
        // a manager this version doesn't know about is preserved too
        assert_eq!(pk["brew"].tools, vec!["wget"]);
    }

    #[test]
    fn merge_drops_manager_with_no_packages() {
        let prev = snap(&[("pnpm", &["x"])]);
        let (pk, _, _) = merge(Some(&prev), vec![("pnpm".into(), Listing::Listed(vec![]))]);
        assert!(!pk.contains_key("pnpm"));
    }

    #[test]
    fn atomic_write_creates_backup() {
        let dir = std::env::temp_dir().join(format!("lazypkg-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("snapshot.toml");

        assert!(write_atomic_with_backup(&path, b"one").unwrap().is_none());
        let bak = write_atomic_with_backup(&path, b"two").unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), "one");
        assert!(!with_suffix(&path, ".tmp").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_refuses_to_overwrite_unreadable_snapshot() {
        let dir = std::env::temp_dir().join(format!("lazypkg-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("snapshot.toml");
        std::fs::write(&path, "this is [not valid toml").unwrap();

        let err = export_to(&path).unwrap_err();
        assert!(format!("{:#}", err).contains("could not be read"));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "this is [not valid toml"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
