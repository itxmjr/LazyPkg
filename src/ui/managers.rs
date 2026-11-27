use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use super::theme;
use crate::app::{App, Panel};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.active_panel == Panel::Managers {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(theme::BG))
        .border_style(Style::default().fg(border_color));

    let items: Vec<ListItem> = app
        .managers
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let label = format!("{} {}", m.icon(), m.name());
            // During a global search no single manager is selected.
            if i == app.selected_manager && !app.is_global_search() {
                ListItem::new(Line::from(Span::styled(
                    label,
                    Style::default()
                        .fg(theme::SELECTED)
                        .add_modifier(Modifier::BOLD),
                )))
            } else {
                ListItem::new(Line::from(Span::styled(
                    label,
                    Style::default().fg(theme::TEXT),
                )))
            }
        })
        .collect();

    let title = if app.is_global_search() {
        "Managers (all)"
    } else {
        "Managers"
    };
    let list = List::new(items).block(block.title(title));

    let mut state = ListState::default();
    if !app.managers.is_empty() && !app.is_global_search() {
        state.select(Some(app.selected_manager));
    }

    frame.render_stateful_widget(list, area, &mut state);
}
