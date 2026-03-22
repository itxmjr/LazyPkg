use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use crate::managers::{PackageManager, Tool, all_managers};

pub enum AppEvent {
    ManagerLoaded(String, Result<Vec<Tool>, String>),
    CheatsheetLoaded(String, String),
}

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
    pub spinner_tick: usize,
    pub tx: Sender<AppEvent>,
    pub rx: Receiver<AppEvent>,
    pub managers_loading: usize,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = channel();
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
            spinner_tick: 0,
            tx,
            rx,
            managers_loading: 0,
        }
    }

    pub fn load_tools(&mut self) {
        self.loading = true;
        let all = all_managers();
        // Keep only available managers
        self.managers = all.into_iter().filter(|m| m.is_available()).collect();

        self.tools_by_manager.clear();
        self.managers_loading = self.managers.len();

        if self.managers_loading == 0 {
            self.loading = false;
            return;
        }

        for manager in &self.managers {
            let tx = self.tx.clone();
            let m_name = manager.name().to_string();

            tokio::task::spawn_blocking(move || {
                let instance = crate::managers::all_managers()
                    .into_iter()
                    .find(|m| m.name() == m_name)
                    .unwrap();
                let res = instance.list_installed().map_err(|e| e.to_string());
                let _ = tx.send(AppEvent::ManagerLoaded(m_name, res));
            });
        }
    }

    pub fn handle_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AppEvent::ManagerLoaded(name, result) => {
                    self.managers_loading = self.managers_loading.saturating_sub(1);
                    match result {
                        Ok(tools) => {
                            self.tools_by_manager.insert(name, tools);
                        }
                        Err(e) => {
                            self.status_message = Some(format!("{}: {}", name, e));
                            self.tools_by_manager.insert(name, Vec::new());
                        }
                    }
                    if self.managers_loading == 0 {
                        self.loading = false;
                        self.load_cheatsheet();
                    }
                }
                AppEvent::CheatsheetLoaded(tool_name, content) => {
                    if let Some(t) = self.selected_tool_item() {
                        if t.name == tool_name {
                            self.cheatsheet = Some(content);
                            self.loading = false;
                        }
                    }
                }
            }
        }
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
        if self.cheatsheet.is_some() {
            return;
        }
        if let Some(tool) = self.selected_tool_item() {
            let name = tool.name.clone();
            self.cheatsheet = None;
            self.loading = true;
            let tx = self.tx.clone();

            tokio::task::spawn_blocking(move || {
                let content = crate::cheatsheet::load_cheatsheet(&name)
                    .unwrap_or_else(|| format!("No cheatsheet found for '{}'", name));
                let _ = tx.send(AppEvent::CheatsheetLoaded(name, content));
            });
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
        self.cheatsheet = None;
        if self.active_panel == Panel::Tools {
            self.load_cheatsheet();
        }
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
        self.cheatsheet = None;
        if self.active_panel == Panel::Tools {
            self.load_cheatsheet();
        }
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

    pub fn tick_spinner(&mut self) {
        if self.loading {
            self.spinner_tick = self.spinner_tick.wrapping_add(1);
        }
    }
}
