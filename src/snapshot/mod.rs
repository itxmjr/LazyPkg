use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod export;
pub mod import;

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub meta: SnapshotMeta,
    pub packages: HashMap<String, ManagerSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub date: String,
    pub hostname: String,
    pub lazypkg_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ManagerSnapshot {
    pub tools: Vec<String>,
}
