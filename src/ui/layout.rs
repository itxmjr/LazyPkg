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

/// Is the terminal cell (`column`, `row`) inside `area`?
pub fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column < area.right() && row >= area.y && row < area.bottom()
}

/// Which item of a bordered `List` drawn in `area` is on screen `row`?
///
/// We build a fresh `ListState` every frame, so ratatui scrolls only as far as
/// needed to keep `selected` visible: the first visible item is
/// `selected + 1 - height` once the selection passes the bottom, else 0.
pub fn list_index_at(area: Rect, row: u16, selected: usize, len: usize) -> Option<usize> {
    let top = area.y + 1; // skip the top border
    let height = area.height.saturating_sub(2) as usize;
    if height == 0 || row < top || (row - top) as usize >= height {
        return None;
    }
    let offset = (selected + 1).saturating_sub(height);
    let index = offset + (row - top) as usize;
    (index < len).then_some(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A bordered list at y=2 with 5 inner rows (y=3..=7).
    const AREA: Rect = Rect {
        x: 0,
        y: 2,
        width: 20,
        height: 7,
    };

    #[test]
    fn borders_and_outside_rows_hit_nothing() {
        assert_eq!(list_index_at(AREA, 2, 0, 10), None); // top border
        assert_eq!(list_index_at(AREA, 8, 0, 10), None); // bottom border
        assert_eq!(list_index_at(AREA, 1, 0, 10), None); // above the panel
    }

    #[test]
    fn rows_map_to_items_without_scrolling() {
        assert_eq!(list_index_at(AREA, 3, 0, 10), Some(0));
        assert_eq!(list_index_at(AREA, 7, 4, 10), Some(4));
    }

    #[test]
    fn empty_space_below_the_last_item_hits_nothing() {
        assert_eq!(list_index_at(AREA, 5, 0, 2), None);
    }

    #[test]
    fn scrolled_list_accounts_for_the_offset() {
        // Selecting item 7 of 10 scrolls so items 3..=7 are visible.
        assert_eq!(list_index_at(AREA, 3, 7, 10), Some(3));
        assert_eq!(list_index_at(AREA, 7, 7, 10), Some(7));
    }

    #[test]
    fn contains_checks_all_edges() {
        assert!(contains(AREA, 0, 2));
        assert!(contains(AREA, 19, 8));
        assert!(!contains(AREA, 20, 5));
        assert!(!contains(AREA, 5, 9));
    }
}
