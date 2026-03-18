pub mod layout;
pub mod managers;
pub mod tools;
pub mod cheatsheet;
pub mod search;
pub mod confirm;
pub mod theme;

pub fn draw<B: ratatui::backend::Backend>(
    _terminal: &mut ratatui::Terminal<B>,
    _app: &crate::app::App,
) -> anyhow::Result<()> {
    Ok(())
}
