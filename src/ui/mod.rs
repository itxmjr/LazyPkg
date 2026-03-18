pub mod layout;
pub mod managers;
pub mod tools;
pub mod cheatsheet;
pub mod search;
pub mod confirm;
pub mod theme;

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Paragraph},
    Frame,
};

fn render_status_bar(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let (text, style) = if let Some(ref msg) = app.status_message {
        (
            msg.as_str().to_string(),
            Style::default().fg(theme::YELLOW).bg(theme::BG),
        )
    } else {
        (
            "j/k: navigate  h/l: switch panel  d: delete  /: search  e: export  q: quit"
                .to_string(),
            Style::default().fg(theme::DIM).bg(theme::BG),
        )
    };

    let paragraph = Paragraph::new(text)
        .style(style)
        .block(Block::default().style(Style::default().bg(theme::BG)));

    frame.render_widget(paragraph, area);
}

pub fn draw<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &crate::app::App,
) -> anyhow::Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();
        // Set background
        frame.render_widget(
            ratatui::widgets::Block::default()
                .style(ratatui::style::Style::default().bg(theme::BG)),
            area,
        );
        let panels = layout::create_layout(area);
        managers::render(frame, app, panels.managers);
        tools::render(frame, app, panels.tools);
        cheatsheet::render(frame, app, panels.cheatsheet);
        // status bar
        render_status_bar(frame, app, panels.status_bar);
        // popups (drawn last, on top)
        if app.show_confirm_delete {
            confirm::render(frame, app);
        }
        if app.show_help {
            // Task 10 - stub for now
        }
    })?;
    Ok(())
}
