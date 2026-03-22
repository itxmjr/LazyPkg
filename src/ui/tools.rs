use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, Panel};
use super::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.active_panel == Panel::Tools {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let title = if app.search_active || !app.search_query.is_empty() {
        format!("Tools [/{}]", app.search_query)
    } else {
        let tools = app.current_tools();
        format!("Tools [{}]", tools.len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme::BG))
        .border_style(Style::default().fg(border_color));

    if app.loading {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let curr_frame = frames[app.spinner_tick % frames.len()];
        let paragraph = Paragraph::new(format!("{} Loading...", curr_frame))
            .style(Style::default().fg(theme::SELECTED))
            .block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    let tools = app.current_tools();

    let items: Vec<ListItem> = tools
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            let name_span = if i == app.selected_tool {
                Span::styled(
                    tool.name.clone(),
                    Style::default()
                        .fg(theme::SELECTED)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(tool.name.clone(), Style::default().fg(theme::TEXT))
            };

            let line = if let Some(ref ver) = tool.version {
                Line::from(vec![
                    name_span,
                    Span::raw("  "),
                    Span::styled(ver.clone(), Style::default().fg(theme::DIM)),
                ])
            } else {
                Line::from(vec![name_span])
            };

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    if !tools.is_empty() {
        state.select(Some(app.selected_tool));
    }

    frame.render_stateful_widget(list, area, &mut state);
}
