use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::theme;
use crate::app::{App, Mode, Panel};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.active_panel == Panel::Tools {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let searching = app.mode == Mode::Search;
    let title = if searching || !app.search_query.is_empty() {
        let cursor = if searching { "▏" } else { "" };
        format!(
            "Tools [/{}{}] {}",
            app.search_query,
            cursor,
            app.current_tools().len()
        )
    } else {
        format!("Tools [{}]", app.current_tools().len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme::BG))
        .border_style(Style::default().fg(border_color));

    if app.tools_loading {
        let paragraph = Paragraph::new(format!("{} Loading...", theme::spinner(app.spinner_tick)))
            .style(Style::default().fg(theme::SELECTED))
            .block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    let tools = app.current_tools();
    if tools.is_empty() {
        let msg = if !app.search_query.is_empty() {
            "No tools match the search (Esc to clear)"
        } else if app.managers.is_empty() {
            "No supported package managers found"
        } else {
            "No packages installed with this manager"
        };
        let paragraph = Paragraph::new(msg)
            .style(Style::default().fg(theme::DIM))
            .block(block);
        frame.render_widget(paragraph, area);
        return;
    }

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
