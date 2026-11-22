pub mod cheatsheet;
pub mod confirm;
pub mod layout;
pub mod managers;
pub mod search;
pub mod theme;
pub mod tools;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, Mode};

/// Priority: a fresh status message, then a running operation, then key hints.
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let bar = Block::default().style(Style::default().bg(theme::BG));

    if let Some(msg) = app.status_message() {
        let p = Paragraph::new(Line::from(Span::styled(
            format!(" {}", msg),
            Style::default().fg(theme::YELLOW).bg(theme::BG),
        )))
        .block(bar);
        frame.render_widget(p, area);
        return;
    }

    if let Some(op) = &app.busy {
        let p = Paragraph::new(Line::from(Span::styled(
            format!(" {} {}", theme::spinner(app.spinner_tick), op),
            Style::default().fg(theme::SELECTED).bg(theme::BG),
        )))
        .block(bar);
        frame.render_widget(p, area);
        return;
    }

    let hints = match app.mode {
        Mode::Search => " type to filter | enter: keep filter | esc: clear | ↑/↓: move",
        _ => " nav: j/k | panels: h/l | del: d | search: / | export: e | refresh: r | help: ? | quit: q",
    };
    let version = format!("lazypkg {} ", env!("CARGO_PKG_VERSION"));

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(version.chars().count() as u16),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(
            hints,
            Style::default().fg(theme::DIM).bg(theme::BG),
        ))
        .block(bar.clone()),
        layout[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            version,
            Style::default().fg(theme::DIM).bg(theme::BG),
        ))
        .alignment(Alignment::Right)
        .block(bar),
        layout[1],
    );
}

fn render_help(frame: &mut Frame, area: Rect) {
    let rows = [
        ("j / k / ↑ / ↓", "Move in the list"),
        ("h / l / ← / →", "Switch panel"),
        ("Tab", "Next panel"),
        ("Enter", "Open cheatsheet"),
        ("d", "Uninstall selected tool (asks first)"),
        ("r", "Refresh installed tools"),
        ("R", "Re-download the cheatsheet"),
        ("/", "Search (Enter keeps the filter)"),
        ("Esc", "Clear the search filter"),
        ("e", "Export snapshot"),
        ("?", "Toggle this help"),
        ("q / Ctrl+C", "Quit"),
    ];

    let mut lines = vec![
        Line::from(Span::styled(
            " Keyboard Shortcuts ",
            Style::default()
                .fg(theme::SELECTED)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    lines.extend(
        rows.iter()
            .map(|(k, v)| Line::from(format!("  {:<15} {}", k, v))),
    );
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Import a snapshot with `lazypkg import` (outside the TUI). ",
        Style::default().fg(theme::DIM),
    )));
    lines.push(Line::from(Span::styled(
        " Press ? or Esc to close ",
        Style::default().fg(theme::DIM),
    )));

    let height = lines.len() as u16 + 2;
    let overlay = centered_rect_fixed(64, height, area);
    frame.render_widget(Clear, overlay);
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(theme::BG))
        .border_style(Style::default().fg(theme::BORDER_ACTIVE));
    frame.render_widget(Paragraph::new(lines).block(block), overlay);
}

pub fn draw<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &App,
) -> anyhow::Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();
        frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

        let panels = layout::create_layout(area);
        managers::render(frame, app, panels.managers);
        tools::render(frame, app, panels.tools);
        cheatsheet::render(frame, app, panels.cheatsheet);
        render_status_bar(frame, app, panels.status_bar);

        // popups (drawn last, on top)
        match app.mode {
            Mode::ConfirmDelete { .. } => confirm::render(frame, app),
            Mode::Help => render_help(frame, area),
            Mode::Normal | Mode::Search => {}
        }
    })?;
    Ok(())
}

/// A popup `width` columns wide and `height` rows tall, centred and clamped to `r`.
fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let width = width.min(r.width);
    let height = height.min(r.height);
    Rect {
        x: r.x + (r.width - width) / 2,
        y: r.y + (r.height - height) / 2,
        width,
        height,
    }
}
