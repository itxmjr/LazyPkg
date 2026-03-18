use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use super::theme;

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
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

pub fn render(frame: &mut Frame, app: &App) {
    if !app.show_confirm_delete {
        return;
    }

    let area = frame.area();
    let popup_area = centered_rect(40, 5, area);

    frame.render_widget(Clear, popup_area);

    let tool_info = if let Some(tool) = app.selected_tool_item() {
        format!(
            "Delete {} ({})?\n\n[y] Yes  [n] No",
            tool.name, tool.manager
        )
    } else {
        "Delete this tool?\n\n[y] Yes  [n] No".to_string()
    };

    let block = Block::default()
        .title("Confirm Delete")
        .borders(Borders::ALL)
        .style(Style::default().bg(theme::BG))
        .border_style(Style::default().fg(theme::RED));

    let paragraph = Paragraph::new(tool_info)
        .style(Style::default().fg(theme::TEXT).bg(theme::BG))
        .block(block);

    frame.render_widget(paragraph, popup_area);
}
