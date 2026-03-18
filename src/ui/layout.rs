use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct Panels {
    pub managers: Rect,
    pub tools: Rect,
    pub cheatsheet: Rect,
    pub status_bar: Rect,
}

pub fn create_layout(area: Rect) -> Panels {
    // First split: main area (top) + status bar (bottom, 1 line)
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let status_bar = vertical[1];

    // Then split main area horizontally: 20% | 35% | 45%
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(35),
            Constraint::Percentage(45),
        ])
        .split(main_area);

    Panels {
        managers: horizontal[0],
        tools: horizontal[1],
        cheatsheet: horizontal[2],
        status_bar,
    }
}
