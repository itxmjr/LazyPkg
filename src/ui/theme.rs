use ratatui::style::Color;

pub const BG: Color = Color::Reset;
pub const SELECTED: Color = Color::LightGreen;
pub const BORDER_ACTIVE: Color = Color::LightGreen;
pub const BORDER_INACTIVE: Color = Color::DarkGray;
pub const TEXT: Color = Color::Reset;
pub const DIM: Color = Color::DarkGray;
pub const RED: Color = Color::Red;
pub const YELLOW: Color = Color::Yellow;

/// Braille spinner frame for the given tick.
pub fn spinner(tick: usize) -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[tick % FRAMES.len()]
}
