//! XDG base directories, resolved safely.
//!
//! Per the XDG spec, empty or relative values of `XDG_*_HOME` are ignored.
//! If `HOME` is also unusable we return `None` rather than silently falling
//! back to a path relative to the current working directory.

use std::path::PathBuf;

fn xdg_dir(var: &str, home_fallback: &[&str]) -> Option<PathBuf> {
    if let Some(v) = std::env::var_os(var) {
        let p = PathBuf::from(v);
        if p.is_absolute() {
            return Some(p);
        }
    }
    let home = PathBuf::from(std::env::var_os("HOME")?);
    if !home.is_absolute() {
        return None;
    }
    Some(home_fallback.iter().fold(home, |acc, part| acc.join(part)))
}

/// `~/.config/lazypkg`
pub fn config_dir() -> Option<PathBuf> {
    xdg_dir("XDG_CONFIG_HOME", &[".config"]).map(|p| p.join("lazypkg"))
}

/// `~/.cache/lazypkg`
pub fn cache_dir() -> Option<PathBuf> {
    xdg_dir("XDG_CACHE_HOME", &[".cache"]).map(|p| p.join("lazypkg"))
}

/// `~/.local/share` (not lazypkg-specific; used to find tealdeer's cache)
pub fn data_home() -> Option<PathBuf> {
    xdg_dir("XDG_DATA_HOME", &[".local", "share"])
}

/// `~/.cache` (not lazypkg-specific)
pub fn cache_home() -> Option<PathBuf> {
    xdg_dir("XDG_CACHE_HOME", &[".cache"])
}
