use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::{App, Panel};
use super::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.active_panel == Panel::Managers {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    let block = Block::default()
        .title("Managers")
        .borders(Borders::ALL)
        .style(Style::default().bg(theme::BG))
        .border_style(Style::default().fg(border_color));

    let items: Vec<ListItem> = app
        .managers
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let label = format!("{} {}", m.icon(), m.name());
            if i == app.selected_manager {
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

    let list = List::new(items).block(block);

    let mut state = ListState::default();
    if !app.managers.is_empty() {
        state.select(Some(app.selected_manager));
    }

    frame.render_stateful_widget(list, area, &mut state);
}
