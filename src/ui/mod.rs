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
    if let Some(ref msg) = app.status_message {
        let p = Paragraph::new(ratatui::text::Line::from(ratatui::text::Span::styled(
            msg.as_str(),
            Style::default().fg(theme::YELLOW).bg(theme::BG),
        ))).block(Block::default().style(Style::default().bg(theme::BG)));
        frame.render_widget(p, area);
    } else {
        let layout = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Min(0),
                ratatui::layout::Constraint::Length(35),
            ])
            .split(area);

        let left_part = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(
                " nav: j/k | switch: h/l | del: d | search: / | export: e | quit: q | ?: keybindings",
                Style::default().fg(theme::DIM).bg(theme::BG),
            ),
        ]);

        let right_part = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(
                "Donate ",
                Style::default().fg(ratatui::style::Color::Magenta).bg(theme::BG),
            ),
            ratatui::text::Span::styled(
                "Ask_Question ",
                Style::default().fg(theme::YELLOW).bg(theme::BG),
            ),
            ratatui::text::Span::styled(
                env!("CARGO_PKG_VERSION"),
                Style::default().fg(theme::DIM).bg(theme::BG),
            ),
        ]);

        let left_p = Paragraph::new(left_part)
            .block(Block::default().style(Style::default().bg(theme::BG)));
        
        let right_p = Paragraph::new(right_part)
            .alignment(ratatui::layout::Alignment::Right)
            .block(Block::default().style(Style::default().bg(theme::BG)));

        frame.render_widget(left_p, layout[0]);
        frame.render_widget(right_p, layout[1]);
    }
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
