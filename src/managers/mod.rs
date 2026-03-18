use anyhow::Result;

pub mod cargo;
pub mod dnf;
pub mod npm;
pub mod pip;
pub mod pipx;
pub mod pnpm;


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
        Box::new(cargo::CargoManager::new()),
        Box::new(dnf::DnfManager::new()),
        Box::new(pipx::PipxManager::new()),
        Box::new(pip::PipManager::new()),
        Box::new(npm::NpmManager::new()),
        Box::new(pnpm::PnpmManager::new()),
    ]
}
