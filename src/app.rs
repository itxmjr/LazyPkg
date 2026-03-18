use std::collections::HashMap;
use crate::managers::{PackageManager, Tool, all_managers};

#[derive(Clone, PartialEq)]
pub enum Panel {
    Managers,
    Tools,
    Cheatsheet,
}

pub struct App {
    pub managers: Vec<Box<dyn PackageManager>>,
    pub tools_by_manager: HashMap<String, Vec<Tool>>,
    pub selected_manager: usize,
    pub selected_tool: usize,
    pub active_panel: Panel,
    pub cheatsheet: Option<String>,
    pub search_query: String,
    pub search_active: bool,
    pub show_confirm_delete: bool,
    pub show_help: bool,
    pub status_message: Option<String>,
    pub loading: bool,
    pub status_shown: bool,
}

impl App {
    pub fn new() -> Self {
        App {
            managers: Vec::new(),
            tools_by_manager: HashMap::new(),
            selected_manager: 0,
            selected_tool: 0,
            active_panel: Panel::Managers,
            cheatsheet: None,
            search_query: String::new(),
            search_active: false,
            show_confirm_delete: false,
            show_help: false,
            status_message: None,
            loading: false,
            status_shown: false,
        }
    }

    pub fn load_tools(&mut self) {
        self.loading = true;
        let all = all_managers();
        // Keep only available managers
        self.managers = all.into_iter().filter(|m| m.is_available()).collect();

        self.tools_by_manager.clear();
        for manager in &self.managers {
            match manager.list_installed() {
                Ok(tools) => {
                    self.tools_by_manager.insert(manager.name().to_string(), tools);
                }
                Err(e) => {
                    self.status_message = Some(format!("{}: {}", manager.name(), e));
                    self.tools_by_manager.insert(manager.name().to_string(), Vec::new());
                }
            }
        }
        self.loading = false;
        self.load_cheatsheet();
    }

    pub fn current_manager(&self) -> Option<&dyn PackageManager> {
        self.managers.get(self.selected_manager).map(|m| m.as_ref())
    }

    pub fn current_tools(&self) -> Vec<&Tool> {
        if let Some(manager) = self.current_manager() {
            let name = manager.name().to_string();
            if let Some(tools) = self.tools_by_manager.get(&name) {
                let query = self.search_query.to_lowercase();
                if query.is_empty() {
                    return tools.iter().collect();
                } else {
                    return tools
                        .iter()
                        .filter(|t| t.name.to_lowercase().contains(&query))
                        .collect();
                }
            }
        }
        Vec::new()
    }

    pub fn selected_tool_item(&self) -> Option<&Tool> {
        let tools = self.current_tools();
        tools.get(self.selected_tool).copied()
    }

    pub fn load_cheatsheet(&mut self) {
        if let Some(tool) = self.selected_tool_item() {
            let name = tool.name.clone();
            // For v1: blocking load (will block UI briefly but acceptable)
            self.cheatsheet = crate::cheatsheet::load_cheatsheet(&name)
                .or_else(|| Some(format!("No cheatsheet found for '{}'", name)));
        }
    }

    pub fn next_tool(&mut self) {
        let count = self.current_tools().len();
        if count == 0 {
            return;
        }
        if self.selected_tool + 1 < count {
            self.selected_tool += 1;
        } else {
            self.selected_tool = 0;
        }
        // Clear stale cheatsheet; it will reload when user navigates to Cheatsheet panel
        self.cheatsheet = None;
    }

    pub fn prev_tool(&mut self) {
        let count = self.current_tools().len();
        if count == 0 {
            return;
        }
        if self.selected_tool > 0 {
            self.selected_tool -= 1;
        } else {
            self.selected_tool = count - 1;
        }
        // Clear stale cheatsheet; it will reload when user navigates to Cheatsheet panel
        self.cheatsheet = None;
    }

    pub fn next_manager(&mut self) {
        let count = self.managers.len();
        if count == 0 {
            return;
        }
        if self.selected_manager + 1 < count {
            self.selected_manager += 1;
        } else {
            self.selected_manager = 0;
        }
        self.selected_tool = 0;
        // Clear stale cheatsheet; it will reload when user navigates to Cheatsheet panel
        self.cheatsheet = None;
    }

    pub fn prev_manager(&mut self) {
        let count = self.managers.len();
        if count == 0 {
            return;
        }
        if self.selected_manager > 0 {
            self.selected_manager -= 1;
        } else {
            self.selected_manager = count - 1;
        }
        self.selected_tool = 0;
        // Clear stale cheatsheet; it will reload when user navigates to Cheatsheet panel
        self.cheatsheet = None;
    }

    pub fn delete_selected_tool(&mut self) -> anyhow::Result<()> {
        // Get the tool name and manager before modifying state
        let (tool_name, manager_name) = {
            let tool = match self.selected_tool_item() {
                Some(t) => t,
                None => return Ok(()),
            };
            (tool.name.clone(), tool.manager.clone())
        };

        // Find the manager and run uninstall
        let manager = self
            .managers
            .iter()
            .find(|m| m.name() == manager_name)
            .ok_or_else(|| anyhow::anyhow!("Manager not found: {}", manager_name))?;

        // Build a temporary Tool for the uninstall call
        let tool_ref = {
            let tools = self.tools_by_manager.get(&manager_name);
            tools
                .and_then(|ts| ts.iter().find(|t| t.name == tool_name))
                .map(|t| crate::managers::Tool {
                    name: t.name.clone(),
                    version: t.version.clone(),
                    manager: t.manager.clone(),
                })
        };

        let tool_obj = tool_ref.ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_name))?;

        manager.uninstall(&tool_obj)?;

        // Remove from local cache
        if let Some(tools) = self.tools_by_manager.get_mut(&manager_name) {
            tools.retain(|t| t.name != tool_name);
        }

        // Clamp selected_tool
        let count = self.current_tools().len();
        if self.selected_tool >= count && count > 0 {
            self.selected_tool = count - 1;
        } else if count == 0 {
            self.selected_tool = 0;
        }

        self.status_message = Some(format!("Deleted '{}'", tool_name));
        Ok(())
    }

    pub fn refresh(&mut self) {
        self.search_query.clear();
        self.search_active = false;
        self.selected_tool = 0;
        self.load_tools();
    }

    pub fn export_snapshot(&mut self) -> anyhow::Result<String> {
        let path = crate::snapshot::export::export_snapshot()?;
        let msg = format!("Exported to {}", path.display());
        self.status_message = Some(msg.clone());
        Ok(msg)
    }

    pub fn maybe_clear_status(&mut self) {
        if self.status_shown {
            self.status_message = None;
            self.status_shown = false;
        } else if self.status_message.is_some() {
            self.status_shown = true;
        }
    }
}
