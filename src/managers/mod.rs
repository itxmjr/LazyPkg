use anyhow::Result;

pub mod cargo;
pub use cargo::CargoManager;

pub struct Tool {
    pub name: String,
    pub version: Option<String>,
    pub manager: String,
}

pub trait PackageManager: Send + Sync {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn is_available(&self) -> bool;
    fn list_installed(&self) -> Result<Vec<Tool>>;
    fn uninstall(&self, tool: &Tool) -> Result<()>;
    fn install(&self, name: &str) -> Result<()>;
}

pub fn all_managers() -> Vec<Box<dyn PackageManager>> {
    vec![
        Box::new(CargoManager::new()),
    ]
}
