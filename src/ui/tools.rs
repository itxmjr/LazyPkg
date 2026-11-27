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
    let global = app.is_global_search();
    let label = if global { "All tools" } else { "Tools" };
    let title = if searching || !app.search_query.is_empty() {
        let cursor = if searching { "▏" } else { "" };
        format!(
            "{} [/{}{}] {}",
            label,
            app.search_query,
            cursor,
            app.current_tools().len()
        )
    } else {
        format!("{} [{}]", label, app.current_tools().len())
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
        } else if global {
            "No packages found in any manager"
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

            let mut spans = vec![name_span];
            if let Some(ref ver) = tool.version {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(ver.clone(), Style::default().fg(theme::DIM)));
            }
            // In a global search, show which manager each result came from.
            if global {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    tool.manager.clone(),
                    Style::default().fg(theme::YELLOW),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    if !tools.is_empty() {
        state.select(Some(app.selected_tool));
    }

    frame.render_stateful_widget(list, area, &mut state);
}
