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
        } else if app.show_help {
            let overlay_area = centered_rect(50, 60, area);
            frame.render_widget(ratatui::widgets::Clear, overlay_area);

            let help_text = vec![
                ratatui::text::Line::from(ratatui::text::Span::styled(" Keyboard Shortcuts ", Style::default().fg(theme::SELECTED).add_modifier(ratatui::style::Modifier::BOLD))),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from("  j / k / ↑ / ↓  : Navigate list"),
                ratatui::text::Line::from("  h / l / ← / →  : Navigate panels"),
                ratatui::text::Line::from("  Tab            : Next panel"),
                ratatui::text::Line::from("  Enter          : Open cheatsheet"),
                ratatui::text::Line::from("  d              : Delete selected tool"),
                ratatui::text::Line::from("  r              : Refresh installed tools"),
                ratatui::text::Line::from("  /              : Search tools"),
                ratatui::text::Line::from("  e              : Export snapshot"),
                ratatui::text::Line::from("  i              : Import snapshot (CLI only)"),
                ratatui::text::Line::from("  ?              : Toggle this help menu"),
                ratatui::text::Line::from("  q / Ctrl+C     : Quit"),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from(ratatui::text::Span::styled(" Press ? or Esc to close ", Style::default().fg(theme::DIM))),
            ];

            let block = Block::default()
                .title(" Help ")
                .borders(ratatui::widgets::Borders::ALL)
                .style(Style::default().bg(theme::BG))
                .border_style(Style::default().fg(theme::BORDER_ACTIVE));

            let paragraph = Paragraph::new(help_text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Left);

            frame.render_widget(paragraph, overlay_area);
        }
    })?;
    Ok(())
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
