use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::theme;
use crate::app::{App, Panel};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.active_panel == Panel::Cheatsheet {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = if let Some(tool) = app.selected_tool_item() {
        format!("Cheatsheet: {}", tool.name)
    } else {
        "Cheatsheet".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme::BG))
        .border_style(Style::default().fg(border_color));

    if app.cheatsheet_loading {
        let paragraph = Paragraph::new(format!(
            "{} Loading cheatsheet...",
            theme::spinner(app.spinner_tick)
        ))
        .style(Style::default().fg(theme::DIM).bg(theme::BG))
        .block(block);
        frame.render_widget(paragraph, area);
    } else if let Some(ref text) = app.cheatsheet {
        let paragraph = Paragraph::new(text.as_str())
            .style(Style::default().fg(theme::TEXT).bg(theme::BG))
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("Select a tool to see its cheatsheet")
            .style(Style::default().fg(theme::DIM).bg(theme::BG))
            .block(block);
        frame.render_widget(paragraph, area);
    }
}
