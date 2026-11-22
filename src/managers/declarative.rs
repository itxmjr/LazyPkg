//! Package managers defined in TOML instead of Rust.
//!
//! Most package managers only need three commands (list / install / uninstall)
//! and a simple output format, so they can be described in a small TOML file:
//!
//! ```toml
//! name = "pacman"
//! icon = "🏹"
//! binary = "pacman"     # must be on PATH for the manager to show up
//! system = true         # OS package manager: installed first during import
//!
//! [list]
//! cmd = ["pacman", "-Qen"]
//! format = "columns"    # lines | columns | parens
//!
//! [install]
//! cmd = ["pacman", "-S", "--needed", "--noconfirm", "{name}"]
//! root = true
//!
//! [uninstall]
//! cmd = ["pacman", "-Rs", "{name}"]
//! root = true
//! ```
//!
//! Built-in definitions live in `src/managers/defs/` and are compiled into the
//! binary. Users can add or override managers with their own files in
//! `~/.config/lazypkg/managers/*.toml`.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

use super::{list_output, PackageManager, Tool};
use crate::exec::Cmd;

/// Definitions shipped with lazypkg: (file name, contents).
const BUILTIN: &[(&str, &str)] = &[
    ("pacman.toml", include_str!("defs/pacman.toml")),
    ("apt.toml", include_str!("defs/apt.toml")),
    ("brew.toml", include_str!("defs/brew.toml")),
    ("gem.toml", include_str!("defs/gem.toml")),
];

const PLACEHOLDER: &str = "{name}";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagerDef {
    pub name: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    /// Executable that must be on PATH for this manager to be available.
    pub binary: String,
    /// OS-level package manager (installed first during import).
    #[serde(default)]
    pub system: bool,
    pub list: ListDef,
    pub install: CmdDef,
    pub uninstall: CmdDef,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListDef {
    pub cmd: Vec<String>,
    #[serde(default)]
    pub format: Format,
    /// Ignore output lines containing any of these strings (headers, warnings…).
    #[serde(default)]
    pub skip_lines_containing: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CmdDef {
    pub cmd: Vec<String>,
    #[serde(default)]
    pub root: bool,
}

/// How to read the list command's output.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    /// One package name per line (`apt-mark showmanual`, `brew leaves`).
    #[default]
    Lines,
    /// `name version` separated by whitespace (`pacman -Qen`).
    Columns,
    /// `name (version, older-version)` (`gem list`).
    Parens,
}

fn default_icon() -> String {
    "📦".to_string()
}

impl ManagerDef {
    pub fn parse(source: &str) -> Result<Self> {
        let def: ManagerDef = toml::from_str(source)?;
        def.validate()?;
        Ok(def)
    }

    fn validate(&self) -> Result<()> {
        let valid_name = !self.name.is_empty()
            && self
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid_name {
            bail!(
                "name '{}' must be lowercase letters, digits or '-'",
                self.name
            );
        }
        if self.binary.trim().is_empty() {
            bail!("binary must not be empty");
        }
        if self.list.cmd.is_empty() {
            bail!("[list] cmd must not be empty");
        }
        for (section, def) in [("install", &self.install), ("uninstall", &self.uninstall)] {
            if def.cmd.is_empty() {
                bail!("[{}] cmd must not be empty", section);
            }
            if !def.cmd.iter().skip(1).any(|a| a.contains(PLACEHOLDER)) {
                bail!(
                    "[{}] cmd must contain the {} placeholder in an argument",
                    section,
                    PLACEHOLDER
                );
            }
        }
        Ok(())
    }
}

pub struct DeclarativeManager {
    def: ManagerDef,
}

impl DeclarativeManager {
    pub fn new(def: ManagerDef) -> Self {
        DeclarativeManager { def }
    }

    fn build(&self, def: &CmdDef, name: &str) -> Cmd {
        let args: Vec<String> = def.cmd[1..]
            .iter()
            .map(|a| a.replace(PLACEHOLDER, name))
            .collect();
        Cmd::new(&def.cmd[0], &args).with_root(def.root)
    }

    /// Parse list output according to the definition's format.
    pub fn parse_list(&self, output: &str) -> Vec<Tool> {
        parse_list(
            &self.def.name,
            self.def.list.format,
            &self.def.list.skip_lines_containing,
            output,
        )
    }
}

impl PackageManager for DeclarativeManager {
    fn name(&self) -> &str {
        &self.def.name
    }

    fn icon(&self) -> &str {
        &self.def.icon
    }

    fn is_available(&self) -> bool {
        crate::exec::which(&self.def.binary).is_some()
    }

    fn list_installed(&self) -> Result<Vec<Tool>> {
        let cmd = Cmd::new(&self.def.list.cmd[0], &self.def.list.cmd[1..]);
        Ok(self.parse_list(&list_output(&cmd)?))
    }

    fn install_cmd(&self, name: &str) -> Cmd {
        self.build(&self.def.install, name)
    }

    fn uninstall_cmd(&self, tool: &Tool) -> Cmd {
        self.build(&self.def.uninstall, &tool.name)
    }

    fn is_system(&self) -> bool {
        self.def.system
    }
}

pub fn parse_list(manager: &str, format: Format, skip: &[String], output: &str) -> Vec<Tool> {
    let mut tools: Vec<Tool> = output
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| !skip.iter().any(|s| l.contains(s.as_str())))
        .filter_map(|line| {
            let (name, version) = match format {
                Format::Lines => (line.split_whitespace().next()?, None),
                Format::Columns => {
                    let mut cols = line.split_whitespace();
                    (cols.next()?, cols.next().map(str::to_string))
                }
                Format::Parens => match line.split_once(" (") {
                    Some((name, rest)) => {
                        let first = rest.trim_end_matches(')').split(',').next().unwrap_or("");
                        let version = first.trim_start_matches("default:").trim();
                        (
                            name.trim(),
                            (!version.is_empty()).then(|| version.to_string()),
                        )
                    }
                    None => (line.split_whitespace().next()?, None),
                },
            };
            Some(Tool {
                name: name.to_string(),
                version,
                manager: manager.to_string(),
            })
        })
        .collect();
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools.dedup_by(|a, b| a.name == b.name);
    tools
}

/// Built-in definitions. These are compiled in, so a failure is a bug caught by tests.
pub fn builtin_defs() -> Vec<ManagerDef> {
    BUILTIN
        .iter()
        .filter_map(|(file, src)| match ManagerDef::parse(src) {
            Ok(def) => Some(def),
            Err(e) => {
                debug_assert!(false, "invalid built-in manager {}: {:#}", file, e);
                None
            }
        })
        .collect()
}

/// User definitions from `~/.config/lazypkg/managers/*.toml`, plus one error
/// message per file that couldn't be loaded.
pub fn user_defs() -> (Vec<ManagerDef>, Vec<String>) {
    match crate::paths::config_dir() {
        Some(dir) => load_dir(&dir.join("managers")),
        None => (Vec::new(), Vec::new()),
    }
}

fn load_dir(dir: &Path) -> (Vec<ManagerDef>, Vec<String>) {
    let mut defs = Vec::new();
    let mut errors = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (defs, errors); // no directory = no user managers
    };
    let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();
    for path in paths {
        let result = std::fs::read_to_string(&path)
            .context("cannot read file")
            .and_then(|src| ManagerDef::parse(&src));
        match result {
            Ok(def) => defs.push(def),
            Err(e) => errors.push(format!("{}: {:#}", path.display(), e)),
        }
    }
    (defs, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn all_builtin_definitions_are_valid() {
        for (file, src) in BUILTIN {
            ManagerDef::parse(src).unwrap_or_else(|e| panic!("{}: {:#}", file, e));
        }
        assert_eq!(builtin_defs().len(), BUILTIN.len());
    }

    #[test]
    fn pacman_columns() {
        let out = include_str!("../../tests/fixtures/pacman/Qen.txt");
        let tools = parse_list("pacman", Format::Columns, &[], out);
        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["base", "bat", "linux", "neovim", "ripgrep"]);
        assert_eq!(tools[3].version.as_deref(), Some("0.10.2-1"));
        assert_eq!(tools[2].version.as_deref(), Some("6.12.1.arch1-1"));
    }

    #[test]
    fn apt_lines() {
        let out = include_str!("../../tests/fixtures/apt/showmanual.txt");
        let tools = parse_list("apt", Format::Lines, &[], out);
        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["build-essential", "curl", "fd-find", "git"]);
        assert!(tools.iter().all(|t| t.version.is_none()));
    }

    #[test]
    fn brew_lines() {
        let out = include_str!("../../tests/fixtures/brew/leaves.txt");
        let tools = parse_list("brew", Format::Lines, &[], out);
        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["gh", "jq", "wget"]);
    }

    #[test]
    fn gem_parens_skips_default_gems() {
        let out = include_str!("../../tests/fixtures/gem/list.txt");
        let tools = parse_list(
            "gem",
            Format::Parens,
            &s(&["(default:", "*** LOCAL GEMS ***"]),
            out,
        );
        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["bundler", "rails", "rake"]);
        assert_eq!(tools[0].version.as_deref(), Some("2.5.6"));
    }

    #[test]
    fn parens_without_skip_strips_default_prefix() {
        let tools = parse_list("gem", Format::Parens, &[], "json (default: 2.7.1)\n");
        assert_eq!(tools[0].version.as_deref(), Some("2.7.1"));
    }

    #[test]
    fn duplicates_and_blank_lines_are_ignored() {
        let tools = parse_list("x", Format::Lines, &[], "\n  a \n\na\nb\n");
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn placeholder_is_substituted_as_one_argument() {
        let def = ManagerDef::parse(include_str!("defs/pacman.toml")).unwrap();
        let m = DeclarativeManager::new(def);
        let cmd = m.install_cmd("neovim");
        assert_eq!(cmd.program, "pacman");
        assert_eq!(cmd.args.last().unwrap(), "neovim");
        assert!(cmd.needs_root);
        assert!(m.is_system());
    }

    #[test]
    fn rejects_invalid_definitions() {
        let base = |extra: &str| {
            format!(
                "name = \"x\"\nbinary = \"x\"\n{extra}\n[list]\ncmd = [\"x\", \"ls\"]\n\
                 [install]\ncmd = [\"x\", \"add\", \"{{name}}\"]\n\
                 [uninstall]\ncmd = [\"x\", \"rm\", \"{{name}}\"]\n"
            )
        };
        assert!(ManagerDef::parse(&base("")).is_ok());
        // unknown key (typo) is an error, not silently ignored
        assert!(ManagerDef::parse(&base("sytem = true")).is_err());
        // missing placeholder
        let no_placeholder = base("").replace("\"rm\", \"{name}\"", "\"rm\"");
        assert!(ManagerDef::parse(&no_placeholder).is_err());
        // bad name
        assert!(
            ManagerDef::parse(&base("").replace("name = \"x\"", "name = \"Bad Name\"")).is_err()
        );
    }

    #[test]
    fn user_dir_loads_valid_and_reports_invalid() {
        let dir = std::env::temp_dir().join(format!("lazypkg-defs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("good.toml"), include_str!("defs/gem.toml")).unwrap();
        std::fs::write(dir.join("bad.toml"), "name = ").unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();

        let (defs, errors) = load_dir(&dir);
        assert_eq!(defs.len(), 1);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("bad.toml"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
