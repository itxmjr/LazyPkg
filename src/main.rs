mod app;
mod cheatsheet;
mod exec;
mod managers;
mod paths;
mod snapshot;
mod ui;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{IsTerminal, Write};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "lazypkg",
    about = "Manage CLI tools across package managers",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Export installed packages to the snapshot file
    Export {
        /// Write to this file instead of ~/.config/lazypkg/snapshot.toml
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Install packages from a snapshot that are missing on this machine
    Import {
        /// Path to snapshot file (default: ~/.config/lazypkg/snapshot.toml)
        file: Option<std::path::PathBuf>,
        /// Show what would be installed, then exit without changing anything
        #[arg(long)]
        dry_run: bool,
        /// Don't ask for confirmation (for scripts and non-interactive shells)
        #[arg(short, long)]
        yes: bool,
    },
    /// List all installed packages
    List {
        /// Only show packages from this package manager
        #[arg(long)]
        manager: Option<String>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {:#}", e);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    if cli.command.is_some() {
        for warning in managers::manager_warnings() {
            eprintln!("warning: ignoring manager definition {}", warning);
        }
    }
    match cli.command {
        Some(Commands::Export { output }) => cmd_export(output),
        Some(Commands::Import { file, dry_run, yes }) => cmd_import(file, dry_run, yes),
        Some(Commands::List { manager }) => cmd_list(manager),
        None => run_tui().map(|()| ExitCode::SUCCESS),
    }
}

fn cmd_export(output: Option<std::path::PathBuf>) -> Result<ExitCode> {
    let report = match output {
        Some(path) => snapshot::export::export_to(&path)?,
        None => snapshot::export::export_snapshot()?,
    };
    for (manager, count) in &report.exported {
        println!("  {:<6} {} packages", manager, count);
    }
    for warning in &report.warnings {
        eprintln!("warning: {}", warning);
    }
    if let Some(bak) = &report.backup {
        println!("Previous snapshot backed up to: {}", bak.display());
    }
    println!("Snapshot exported to: {}", report.path.display());
    Ok(ExitCode::SUCCESS)
}

fn cmd_import(file: Option<std::path::PathBuf>, dry_run: bool, yes: bool) -> Result<ExitCode> {
    let path = match file {
        Some(p) => p,
        None => snapshot::export::default_snapshot_path()?,
    };
    if !path.exists() {
        anyhow::bail!(
            "snapshot file not found: {} (create one with `lazypkg export`)",
            path.display()
        );
    }
    let snap = snapshot::import::read_snapshot(&path)?;
    let plan = snapshot::import::compute_plan(&snap);

    for (manager, reason) in &plan.skipped {
        eprintln!("skipped [{}]: {}", manager, reason);
    }

    if plan.diffs.is_empty() {
        if plan.skipped.is_empty() {
            println!("All packages already installed. Nothing to do.");
            return Ok(ExitCode::SUCCESS);
        }
        println!("Nothing else to install.");
        return Ok(ExitCode::from(2));
    }

    let total: usize = plan.diffs.iter().map(|d| d.missing_tools.len()).sum();
    println!("Missing packages to install ({}):", total);
    for diff in &plan.diffs {
        for tool in &diff.missing_tools {
            println!("  [{}] {}", diff.manager, tool);
        }
    }

    if dry_run {
        println!("\nDry run: nothing was installed.");
        return Ok(ExitCode::SUCCESS);
    }

    if !yes {
        if !std::io::stdin().is_terminal() {
            anyhow::bail!(
                "stdin is not a terminal; re-run with --yes to install without confirmation"
            );
        }
        print!("\nInstall {} missing packages? [y/N] ", total);
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return Ok(ExitCode::SUCCESS);
        }
    }

    let summary = snapshot::import::install_missing(&plan.diffs, |manager, tool, cmd| {
        println!("\n==> [{}] {}\n$ {}", manager, tool, cmd.display());
    });

    println!();
    for pkg in &summary.installed {
        println!("  ✓ {}", pkg);
    }
    for (pkg, err) in &summary.failed {
        println!("  ✗ {}: {}", pkg, err);
    }
    println!(
        "\nInstalled {}, failed {}, skipped managers {}.",
        summary.installed.len(),
        summary.failed.len(),
        plan.skipped.len()
    );

    // 0 = everything done, 2 = partial (some failed or skipped)
    if summary.failed.is_empty() && plan.skipped.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(2))
    }
}

fn cmd_list(filter: Option<String>) -> Result<ExitCode> {
    let all = managers::all_managers();
    if let Some(name) = &filter {
        if !all.iter().any(|m| m.name() == name) {
            let known: Vec<&str> = all.iter().map(|m| m.name()).collect();
            anyhow::bail!("unknown manager '{}' (known: {})", name, known.join(", "));
        }
    }

    let mut had_error = false;
    for mgr in all.iter().filter(|m| m.is_available()) {
        if filter.as_deref().is_some_and(|f| f != mgr.name()) {
            continue;
        }
        match mgr.list_installed() {
            Ok(tools) => {
                for tool in tools {
                    match tool.version {
                        Some(v) => println!("{}: {}@{}", tool.manager, tool.name, v),
                        None => println!("{}: {}", tool.manager, tool.name),
                    }
                }
            }
            Err(e) => {
                had_error = true;
                eprintln!("error listing {} packages: {:#}", mgr.name(), e);
            }
        }
    }
    Ok(if had_error {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

// ── TUI ──────────────────────────────────────────────────────────────────

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

type Term = Terminal<CrosstermBackend<std::io::Stdout>>;

fn run_tui() -> Result<()> {
    if !std::io::stdout().is_terminal() {
        anyhow::bail!("the interactive UI needs a terminal; use `lazypkg list` for scripts");
    }

    // Restore the terminal if anything panics, so the shell isn't left broken.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(e.into());
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut app = app::App::new();
    app.load_tools();
    let result = run_app(&mut terminal, &mut app);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
    result
}

fn run_app(terminal: &mut Term, app: &mut app::App) -> Result<()> {
    loop {
        app.handle_events();
        app.tick_spinner();
        app.maybe_clear_status();
        ui::draw(terminal, app)?;

        // A confirmed action that needs root: hand the terminal to the command
        // so its password prompt and output are visible, then come back.
        if let Some((tool, cmd)) = app.pending_interactive.take() {
            let result = run_suspended(terminal, &cmd)?;
            app.finish_uninstall(tool, result);
            continue;
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            // Only react to presses: some terminals also report repeats/releases.
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && app.handle_key(key) {
                    return Ok(());
                }
            }
        }
    }
}

/// Leave the TUI, run `cmd` attached to the real terminal, wait for Enter, and
/// restore the TUI. Outer `Err` = terminal failure; inner = the command's result.
fn run_suspended(terminal: &mut Term, cmd: &exec::Cmd) -> Result<Result<(), String>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    println!(
        "\nlazypkg needs administrator rights for this action:\n\n  $ {}\n",
        cmd.display()
    );
    let result = exec::run_interactive(cmd).map_err(|e| format!("{:#}", e));
    match &result {
        Ok(()) => println!("\n✓ Done."),
        Err(e) => println!("\n✗ {}", e),
    }
    print!("\nPress Enter to return to lazypkg…");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);

    enable_raw_mode().context("failed to re-enter raw mode")?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;
    Ok(result)
}
