use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::exec::{self, Cmd};
use crate::managers::{all_managers, PackageManager, Tool};

/// How long a status message stays visible.
const STATUS_TTL: Duration = Duration::from_secs(4);

pub enum AppEvent {
    ManagerLoaded {
        generation: u64,
        name: String,
        result: Result<Vec<Tool>, String>,
    },
    CheatsheetLoaded {
        request: u64,
        content: String,
    },
    UninstallCompleted {
        tool: Tool,
        result: Result<(), String>,
    },
    ExportCompleted(Result<String, String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Managers,
    Tools,
    Cheatsheet,
}

/// Exactly one input mode is active at a time. Modal modes (search, confirm,
/// help) receive *all* key presses, so e.g. `j` can't move the selection while
/// a delete confirmation for a specific tool is on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    ConfirmDelete { tool: Tool, cmd: Cmd },
    Help,
}

/// What `/` searches: the selected manager's tools (from the Tools panel), or
/// every manager at once (from the Managers panel), so you can find where a
/// tool came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchScope {
    Manager,
    All,
}

pub struct App {
    pub managers: Vec<Arc<dyn PackageManager>>,
    pub tools_by_manager: HashMap<String, Vec<Tool>>,
    pub selected_manager: usize,
    pub selected_tool: usize,
    pub active_panel: Panel,
    pub mode: Mode,
    pub search_query: String,
    pub search_scope: SearchScope,

    pub cheatsheet: Option<String>,
    pub cheatsheet_loading: bool,
    cheatsheet_tool: Option<String>,
    cheatsheet_request: u64,

    pub tools_loading: bool,
    load_generation: u64,
    managers_pending: usize,

    /// Label of a long-running operation (uninstall, export) in progress.
    pub busy: Option<String>,
    /// A command needing root that the event loop must run with the TUI
    /// suspended, so the password prompt is visible and usable.
    pub pending_interactive: Option<(Tool, Cmd)>,
    quit_armed: bool,

    status: Option<(String, Instant)>,
    pub spinner_tick: usize,
    tx: Sender<AppEvent>,
    rx: Receiver<AppEvent>,
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
            mode: Mode::Normal,
            search_query: String::new(),
            search_scope: SearchScope::Manager,
            cheatsheet: None,
            cheatsheet_loading: false,
            cheatsheet_tool: None,
            cheatsheet_request: 0,
            tools_loading: false,
            load_generation: 0,
            managers_pending: 0,
            busy: None,
            pending_interactive: None,
            quit_armed: false,
            status: None,
            spinner_tick: 0,
            tx,
            rx,
        }
    }

    // ── status line ───────────────────────────────────────────────────────

    pub fn status_message(&self) -> Option<&str> {
        self.status.as_ref().map(|(m, _)| m.as_str())
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), Instant::now()));
    }

    pub fn maybe_clear_status(&mut self) {
        if matches!(&self.status, Some((_, at)) if at.elapsed() >= STATUS_TTL) {
            self.status = None;
        }
    }

    pub fn is_spinning(&self) -> bool {
        self.tools_loading || self.cheatsheet_loading || self.busy.is_some()
    }

    pub fn tick_spinner(&mut self) {
        if self.is_spinning() {
            self.spinner_tick = self.spinner_tick.wrapping_add(1);
        }
    }

    // ── loading ───────────────────────────────────────────────────────────

    pub fn load_tools(&mut self) {
        self.managers = all_managers()
            .into_iter()
            .filter(|m| m.is_available())
            .collect();
        if self.selected_manager >= self.managers.len() {
            self.selected_manager = 0;
        }
        if let Some(first) = crate::managers::manager_warnings().first() {
            self.set_status(format!("Ignoring manager definition {}", first));
        }

        // A new generation makes results from any still-running earlier load
        // (e.g. a refresh pressed while loading) be ignored.
        self.load_generation += 1;
        self.tools_by_manager.clear();
        self.managers_pending = self.managers.len();
        self.tools_loading = !self.managers.is_empty();
        self.clear_cheatsheet();

        if self.managers.is_empty() {
            self.set_status("No supported package managers found on PATH");
            return;
        }

        for manager in &self.managers {
            let tx = self.tx.clone();
            let manager = Arc::clone(manager);
            let generation = self.load_generation;
            std::thread::spawn(move || {
                let result = manager.list_installed().map_err(|e| format!("{:#}", e));
                let _ = tx.send(AppEvent::ManagerLoaded {
                    generation,
                    name: manager.name().to_string(),
                    result,
                });
            });
        }
    }

    pub fn handle_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AppEvent::ManagerLoaded {
                    generation,
                    name,
                    result,
                } => {
                    if generation != self.load_generation {
                        continue; // stale result from a previous load
                    }
                    self.managers_pending = self.managers_pending.saturating_sub(1);
                    match result {
                        Ok(tools) => {
                            self.tools_by_manager.insert(name, tools);
                        }
                        Err(e) => {
                            self.set_status(format!("{}: {}", name, e));
                            self.tools_by_manager.insert(name, Vec::new());
                        }
                    }
                    if self.managers_pending == 0 {
                        self.tools_loading = false;
                        self.clamp_selection();
                        self.load_cheatsheet();
                    }
                }
                AppEvent::CheatsheetLoaded { request, content } => {
                    // Only the latest request may update the panel; anything
                    // older belongs to a tool the user has already moved past.
                    if request == self.cheatsheet_request {
                        self.cheatsheet = Some(content);
                        self.cheatsheet_loading = false;
                    }
                }
                AppEvent::UninstallCompleted { tool, result } => {
                    self.finish_uninstall(tool, result);
                }
                AppEvent::ExportCompleted(result) => {
                    self.busy = None;
                    match result {
                        Ok(msg) => self.set_status(msg),
                        Err(e) => self.set_status(format!("Export failed: {}", e)),
                    }
                }
            }
        }
    }

    // ── selection ─────────────────────────────────────────────────────────

    pub fn current_manager(&self) -> Option<&dyn PackageManager> {
        self.managers.get(self.selected_manager).map(|m| m.as_ref())
    }

    /// The tools shown in the Tools panel: the selected manager's, or every
    /// manager's (in panel order) during a global search, filtered by the query.
    pub fn current_tools(&self) -> Vec<&Tool> {
        let query = self.search_query.to_lowercase();
        let matches = |t: &&Tool| query.is_empty() || t.name.to_lowercase().contains(&query);
        match self.search_scope {
            SearchScope::All => self
                .managers
                .iter()
                .filter_map(|m| self.tools_by_manager.get(m.name()))
                .flat_map(|tools| tools.iter())
                .filter(matches)
                .collect(),
            SearchScope::Manager => {
                let Some(manager) = self.current_manager() else {
                    return Vec::new();
                };
                let Some(tools) = self.tools_by_manager.get(manager.name()) else {
                    return Vec::new();
                };
                tools.iter().filter(matches).collect()
            }
        }
    }

    pub fn is_global_search(&self) -> bool {
        self.search_scope == SearchScope::All
    }

    /// Go back to showing one manager's tools, dropping a global search.
    fn end_global_search(&mut self) {
        if self.search_scope == SearchScope::All {
            self.search_scope = SearchScope::Manager;
            self.search_query.clear();
            self.selected_tool = 0;
            if self.mode == Mode::Search {
                self.mode = Mode::Normal;
            }
        }
    }

    pub fn selected_tool_item(&self) -> Option<&Tool> {
        self.current_tools().get(self.selected_tool).copied()
    }

    fn clamp_selection(&mut self) {
        let count = self.current_tools().len();
        if self.selected_tool >= count {
            self.selected_tool = count.saturating_sub(1);
        }
    }

    fn on_tool_selection_changed(&mut self) {
        self.clear_cheatsheet();
        if self.active_panel != Panel::Managers {
            self.load_cheatsheet();
        }
    }

    pub fn next_tool(&mut self) {
        let count = self.current_tools().len();
        if count == 0 {
            return;
        }
        self.selected_tool = (self.selected_tool + 1) % count;
        self.on_tool_selection_changed();
    }

    pub fn prev_tool(&mut self) {
        let count = self.current_tools().len();
        if count == 0 {
            return;
        }
        self.selected_tool = (self.selected_tool + count - 1) % count;
        self.on_tool_selection_changed();
    }

    pub fn next_manager(&mut self) {
        let count = self.managers.len();
        if count == 0 {
            return;
        }
        if self.is_global_search() {
            // Leaving a global search just shows the selected manager again.
            self.end_global_search();
            self.clear_cheatsheet();
            return;
        }
        self.selected_manager = (self.selected_manager + 1) % count;
        self.selected_tool = 0;
        self.clear_cheatsheet();
    }

    pub fn prev_manager(&mut self) {
        let count = self.managers.len();
        if count == 0 {
            return;
        }
        if self.is_global_search() {
            // Leaving a global search just shows the selected manager again.
            self.end_global_search();
            self.clear_cheatsheet();
            return;
        }
        self.selected_manager = (self.selected_manager + count - 1) % count;
        self.selected_tool = 0;
        self.clear_cheatsheet();
    }

    // ── cheatsheet ────────────────────────────────────────────────────────

    pub fn load_cheatsheet(&mut self) {
        let Some(name) = self.selected_tool_item().map(|t| t.name.clone()) else {
            return;
        };
        let already = self.cheatsheet_tool.as_deref() == Some(name.as_str())
            && (self.cheatsheet.is_some() || self.cheatsheet_loading);
        if already {
            return;
        }

        self.cheatsheet_request += 1;
        let request = self.cheatsheet_request;
        self.cheatsheet = None;
        self.cheatsheet_tool = Some(name.clone());
        self.cheatsheet_loading = true;

        // Unit tests drive the state machine without touching the network or
        // running other programs' --help.
        if cfg!(test) {
            return;
        }
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let content = crate::cheatsheet::load_cheatsheet(&name)
                .unwrap_or_else(|| format!("No cheatsheet found for '{}'", name));
            let _ = tx.send(AppEvent::CheatsheetLoaded { request, content });
        });
    }

    /// Forget the current cheatsheet and invalidate any in-flight fetch.
    pub fn clear_cheatsheet(&mut self) {
        self.cheatsheet = None;
        self.cheatsheet_tool = None;
        self.cheatsheet_loading = false;
        self.cheatsheet_request += 1;
    }

    pub fn force_refresh_cheatsheet(&mut self) {
        let Some(name) = self.selected_tool_item().map(|t| t.name.clone()) else {
            return;
        };
        if let Some(path) = crate::cheatsheet::tldr::lazypkg_cache_path(&name) {
            let _ = std::fs::remove_file(path);
        }
        self.clear_cheatsheet();
        self.load_cheatsheet();
    }

    // ── actions ───────────────────────────────────────────────────────────

    /// Open the delete confirmation for the selected tool, showing the exact
    /// command (and whether it needs root) before anything runs.
    pub fn request_delete(&mut self) {
        if let Some(op) = &self.busy {
            let msg = format!("Please wait: {}", op);
            self.set_status(msg);
            return;
        }
        let Some(tool) = self.selected_tool_item().cloned() else {
            return;
        };
        let Some(manager) = self.managers.iter().find(|m| m.name() == tool.manager) else {
            return;
        };
        let cmd = manager.uninstall_cmd(&tool);
        self.mode = Mode::ConfirmDelete { tool, cmd };
    }

    fn confirm_delete(&mut self) {
        let Mode::ConfirmDelete { tool, cmd } = std::mem::replace(&mut self.mode, Mode::Normal)
        else {
            return;
        };
        self.busy = Some(format!("Uninstalling '{}'…", tool.name));

        if cmd.will_elevate() {
            // Needs a password prompt: the event loop suspends the TUI and runs it.
            self.pending_interactive = Some((tool, cmd));
            return;
        }

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = exec::run_checked(&cmd).map_err(|e| format!("{:#}", e));
            let _ = tx.send(AppEvent::UninstallCompleted { tool, result });
        });
    }

    pub fn finish_uninstall(&mut self, tool: Tool, result: Result<(), String>) {
        self.busy = None;
        match result {
            Ok(()) => {
                if let Some(tools) = self.tools_by_manager.get_mut(&tool.manager) {
                    tools.retain(|t| t.name != tool.name);
                }
                self.clamp_selection();
                self.set_status(format!("Uninstalled '{}' ({})", tool.name, tool.manager));
                self.on_tool_selection_changed();
            }
            Err(e) => self.set_status(format!("Failed to uninstall '{}': {}", tool.name, e)),
        }
    }

    pub fn refresh(&mut self) {
        self.search_scope = SearchScope::Manager;
        self.search_query.clear();
        self.selected_tool = 0;
        self.load_tools();
    }

    /// Export in the background so the UI never freezes while managers are listed.
    pub fn start_export(&mut self) {
        if let Some(op) = &self.busy {
            let msg = format!("Please wait: {}", op);
            self.set_status(msg);
            return;
        }
        self.busy = Some("Exporting snapshot…".to_string());
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = crate::snapshot::export::export_snapshot()
                .map(|r| {
                    let total: usize = r.exported.iter().map(|(_, n)| n).sum();
                    let mut msg = format!("Exported {} packages to {}", total, r.path.display());
                    if let Some(first) = r.warnings.first() {
                        msg.push_str(&format!(" — warning: {}", first));
                        if r.warnings.len() > 1 {
                            msg.push_str(&format!(" (+{} more)", r.warnings.len() - 1));
                        }
                    }
                    msg
                })
                .map_err(|e| format!("{:#}", e));
            let _ = tx.send(AppEvent::ExportCompleted(result));
        });
    }

    // ── input ─────────────────────────────────────────────────────────────

    /// Handle one key press. Returns `true` if the app should quit.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }

        match self.mode {
            Mode::Search => self.handle_search_key(key),
            Mode::Help => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
                ) {
                    self.mode = Mode::Normal;
                }
            }
            Mode::ConfirmDelete { .. } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => self.confirm_delete(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.mode = Mode::Normal
                }
                _ => {}
            },
            Mode::Normal => return self.handle_normal_key(key),
        }
        false
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.clear_search();
            }
            // Keep the filter and go back to navigating it
            KeyCode::Enter => self.mode = Mode::Normal,
            KeyCode::Down => self.next_tool(),
            KeyCode::Up => self.prev_tool(),
            KeyCode::Backspace => {
                let mut q = self.search_query.clone();
                q.pop();
                self.set_search_query(q);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut q = self.search_query.clone();
                q.push(c);
                self.set_search_query(q);
            }
            _ => {}
        }
    }

    /// Clear the query and return to the selected manager's tools. A global
    /// search started in the Managers panel, so cancelling it goes back there.
    fn clear_search(&mut self) {
        if self.is_global_search() {
            self.end_global_search();
            self.active_panel = Panel::Managers;
            self.on_tool_selection_changed();
        } else {
            self.set_search_query(String::new());
        }
    }

    fn set_search_query(&mut self, query: String) {
        if query == self.search_query {
            return;
        }
        self.search_query = query;
        self.selected_tool = 0;
        self.on_tool_selection_changed();
    }

    /// `/` from the Managers panel searches every manager; from the Tools
    /// panel it refines the current list (one manager's, or global results).
    fn start_search(&mut self) {
        let scope = if self.active_panel == Panel::Managers {
            SearchScope::All
        } else {
            self.search_scope
        };
        if scope != self.search_scope {
            self.search_scope = scope;
            self.search_query.clear();
            self.selected_tool = 0;
            self.clear_cheatsheet();
        }
        self.active_panel = Panel::Tools;
        self.mode = Mode::Search;
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> bool {
        if key.code != KeyCode::Char('q') {
            self.quit_armed = false;
        }
        match key.code {
            KeyCode::Char('q') => {
                if self.busy.is_some() && !self.quit_armed {
                    self.quit_armed = true;
                    self.set_status("An operation is still running — press q again to quit anyway");
                    return false;
                }
                return true;
            }
            KeyCode::Char('j') | KeyCode::Down => match self.active_panel {
                Panel::Managers => self.next_manager(),
                Panel::Tools | Panel::Cheatsheet => self.next_tool(),
            },
            KeyCode::Char('k') | KeyCode::Up => match self.active_panel {
                Panel::Managers => self.prev_manager(),
                Panel::Tools | Panel::Cheatsheet => self.prev_tool(),
            },
            KeyCode::Char('h') | KeyCode::Left => match self.active_panel {
                Panel::Managers => {}
                Panel::Tools => self.active_panel = Panel::Managers,
                Panel::Cheatsheet => self.active_panel = Panel::Tools,
            },
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => match self.active_panel {
                Panel::Managers => {
                    self.active_panel = Panel::Tools;
                    self.load_cheatsheet();
                }
                Panel::Tools => {
                    self.active_panel = Panel::Cheatsheet;
                    self.load_cheatsheet();
                }
                Panel::Cheatsheet => {}
            },
            KeyCode::Enter => {
                if self.active_panel == Panel::Tools {
                    self.active_panel = Panel::Cheatsheet;
                    self.load_cheatsheet();
                }
            }
            KeyCode::Char('d') => {
                if self.active_panel == Panel::Tools {
                    self.request_delete();
                }
            }
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('R') => {
                if self.active_panel != Panel::Managers {
                    self.force_refresh_cheatsheet();
                }
            }
            KeyCode::Char('/') => self.start_search(),
            KeyCode::Esc => self.clear_search(),
            KeyCode::Char('e') => self.start_export(),
            KeyCode::Char('i') => {
                self.set_status("Import runs outside the TUI: quit and run `lazypkg import`")
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            _ => {}
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn tool(name: &str) -> Tool {
        Tool {
            name: name.into(),
            version: None,
            manager: "cargo".into(),
        }
    }

    /// An app with the cargo manager and three fake tools, no background loads.
    fn app_with_tools() -> App {
        let mut app = App::new();
        app.managers = vec![Arc::new(crate::managers::cargo::CargoManager::new())];
        app.tools_by_manager.insert(
            "cargo".into(),
            vec![tool("bat"), tool("fd"), tool("ripgrep")],
        );
        app.active_panel = Panel::Tools;
        app
    }

    #[test]
    fn status_survives_many_frames() {
        let mut app = App::new();
        app.set_status("hello");
        for _ in 0..50 {
            app.maybe_clear_status();
        }
        assert_eq!(app.status_message(), Some("hello"));
    }

    #[test]
    fn status_expires_after_ttl() {
        let mut app = App::new();
        app.status = Some((
            "old".into(),
            Instant::now() - STATUS_TTL - Duration::from_millis(1),
        ));
        app.maybe_clear_status();
        assert_eq!(app.status_message(), None);
    }

    #[test]
    fn stale_cheatsheet_does_not_stick_loading() {
        let mut app = app_with_tools();
        app.cheatsheet_loading = true;
        let stale = app.cheatsheet_request;
        app.next_manager(); // user moves away before the fetch returns
        app.tx
            .send(AppEvent::CheatsheetLoaded {
                request: stale,
                content: "old".into(),
            })
            .unwrap();
        app.handle_events();
        assert!(!app.cheatsheet_loading);
        assert!(app.cheatsheet.is_none());
        assert!(
            !app.tools_loading,
            "a cheatsheet load must never hide the tool list"
        );
    }

    #[test]
    fn stale_manager_results_are_ignored() {
        let mut app = app_with_tools();
        app.load_generation = 5;
        app.managers_pending = 1;
        app.tools_loading = true;
        app.tx
            .send(AppEvent::ManagerLoaded {
                generation: 4,
                name: "cargo".into(),
                result: Ok(vec![]),
            })
            .unwrap();
        app.handle_events();
        assert_eq!(app.managers_pending, 1);
        assert!(app.tools_loading);
    }

    #[test]
    fn confirm_dialog_captures_input() {
        let mut app = app_with_tools();
        app.selected_tool = 1; // fd
        app.handle_key(key('d'));
        let Mode::ConfirmDelete { tool, cmd } = &app.mode else {
            panic!("expected confirm mode");
        };
        assert_eq!(tool.name, "fd");
        assert_eq!(cmd.args, vec!["uninstall", "fd"]);

        // navigation is ignored while confirming; the target can't change
        assert!(!app.handle_key(key('j')));
        assert_eq!(app.selected_tool, 1);
        assert!(matches!(app.mode, Mode::ConfirmDelete { .. }));
    }

    #[test]
    fn confirm_cancel_returns_to_normal() {
        let mut app = app_with_tools();
        app.handle_key(key('d'));
        assert!(
            !app.handle_key(key('q')),
            "q cancels the dialog, it doesn't quit"
        );
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.busy.is_none());
    }

    #[test]
    fn no_second_delete_while_busy() {
        let mut app = app_with_tools();
        app.busy = Some("Uninstalling 'bat'…".into());
        app.handle_key(key('d'));
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.status_message().unwrap().contains("Please wait"));
    }

    #[test]
    fn quit_needs_confirmation_while_busy() {
        let mut app = app_with_tools();
        app.busy = Some("x".into());
        assert!(!app.handle_key(key('q')));
        assert!(app.handle_key(key('q')));
    }

    #[test]
    fn help_mode_swallows_keys() {
        let mut app = app_with_tools();
        app.handle_key(key('?'));
        assert_eq!(app.mode, Mode::Help);
        app.handle_key(key('j'));
        assert_eq!(app.selected_tool, 0);
        app.handle_key(key('?'));
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn search_enter_keeps_filter() {
        let mut app = app_with_tools();
        app.handle_key(key('/'));
        app.handle_key(key('r'));
        app.handle_key(key('i'));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.search_query, "ri");
        let names: Vec<_> = app.current_tools().iter().map(|t| t.name.clone()).collect();
        assert_eq!(names, vec!["ripgrep"]);
        // Esc in normal mode clears it
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn uninstall_completion_clamps_selection() {
        let mut app = app_with_tools();
        app.selected_tool = 2; // ripgrep, last
        app.busy = Some("x".into());
        app.finish_uninstall(tool("ripgrep"), Ok(()));
        assert_eq!(app.selected_tool, 1);
        assert!(app.busy.is_none());
        assert_eq!(app.current_tools().len(), 2);
    }
    /// `app_with_tools()` plus a second manager (pipx) with its own tools.
    fn app_with_two_managers() -> App {
        let mut app = app_with_tools();
        app.managers
            .push(Arc::new(crate::managers::pipx::PipxManager::new()));
        app.tools_by_manager.insert(
            "pipx".into(),
            vec![Tool {
                name: "httpie".into(),
                version: None,
                manager: "pipx".into(),
            }],
        );
        app
    }

    fn type_str(app: &mut App, s: &str) {
        for c in s.chars() {
            app.handle_key(key(c));
        }
    }

    fn esc() -> KeyEvent {
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
    }

    #[test]
    fn slash_in_managers_searches_every_manager() {
        let mut app = app_with_two_managers();
        app.active_panel = Panel::Managers;
        app.handle_key(key('/'));
        assert!(app.is_global_search());
        assert_eq!(app.active_panel, Panel::Tools);
        // Empty query: everything, in manager order.
        assert_eq!(app.current_tools().len(), 4);
        type_str(&mut app, "http");
        let found = app.selected_tool_item().expect("a result");
        assert_eq!(found.name, "httpie");
        assert_eq!(found.manager, "pipx");
    }

    #[test]
    fn slash_in_tools_searches_only_the_current_manager() {
        let mut app = app_with_two_managers();
        app.handle_key(key('/'));
        assert!(!app.is_global_search());
        type_str(&mut app, "http");
        assert!(app.current_tools().is_empty());
    }

    #[test]
    fn esc_ends_a_global_search() {
        let mut app = app_with_two_managers();
        app.active_panel = Panel::Managers;
        app.handle_key(key('/'));
        type_str(&mut app, "http");
        app.handle_key(esc());
        assert!(!app.is_global_search());
        assert_eq!(app.active_panel, Panel::Managers);
        assert_eq!(app.search_query, "");
        assert_eq!(app.current_tools().len(), 3); // cargo's tools again
    }

    #[test]
    fn changing_manager_ends_a_kept_global_search() {
        let mut app = app_with_two_managers();
        app.active_panel = Panel::Managers;
        app.handle_key(key('/'));
        type_str(&mut app, "http");
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.is_global_search(), "Enter keeps the results");
        app.handle_key(key('h'));
        app.handle_key(key('j'));
        assert!(!app.is_global_search());
        assert_eq!(app.selected_manager, 0, "first move only leaves the search");
    }
}
