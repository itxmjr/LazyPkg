use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod export;
pub mod import;

/// On-disk snapshot (`~/.config/lazypkg/snapshot.toml`).
///
/// `BTreeMap` keeps manager keys in a stable order so the file produces clean
/// diffs when it is committed to a dotfiles repository. Every section has a
/// default so hand-written or older files still load.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub meta: SnapshotMeta,
    #[serde(default)]
    pub packages: BTreeMap<String, ManagerSnapshot>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SnapshotMeta {
    /// RFC 3339 UTC timestamp (older versions wrote unix seconds; both load).
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub lazypkg_version: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ManagerSnapshot {
    #[serde(default)]
    pub tools: Vec<String>,
}
