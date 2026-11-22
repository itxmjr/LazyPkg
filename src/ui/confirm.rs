use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::theme;
use crate::app::{App, Mode};

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let height = height.min(area.height);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}

/// Delete confirmation. Shows the exact command that will run, and warns when
/// it needs administrator rights, before anything happens.
pub fn render(frame: &mut Frame, app: &App) {
    let Mode::ConfirmDelete { tool, cmd } = &app.mode else {
        return;
    };

    let mut text = format!(
        "Uninstall {} ({})?\n\nThis will run:\n  $ {}\n",
        tool.name,
        tool.manager,
        cmd.display()
    );
    if cmd.will_elevate() {
        text.push_str(
            "\n⚠ Needs administrator rights: lazypkg will pause\n  so you can enter your password.\n",
        );
    }
    text.push_str("\n[y] Yes   [n] No");

    let height = text.lines().count() as u16 + 2;
    let popup_area = centered_rect(60, height, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Confirm uninstall ")
        .borders(Borders::ALL)
        .style(Style::default().bg(theme::BG))
        .border_style(Style::default().fg(theme::RED));

    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(theme::TEXT).bg(theme::BG))
        .wrap(Wrap { trim: false })
        .block(block);

    frame.render_widget(paragraph, popup_area);
}
