use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    // Task list
    let items: Vec<ListItem> = app
        .archive_tasks
        .iter()
        .map(|task| {
            let date = task.updated_at.format("%Y-%m-%d").to_string();
            ListItem::new(format!(" {}  (done {})", task.description, date))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Archive — Done tasks ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Green))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, chunks[0], &mut app.archive_list_state);

    if app.archive_tasks.is_empty() {
        let inner = chunks[0].inner(ratatui::layout::Margin {
            horizontal: 2,
            vertical: 2,
        });
        frame.render_widget(
            Paragraph::new("No archived tasks yet.\nPress A on the board to archive a task.")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
    }

    // Status bar
    let mut status_parts = vec!["r:restore  d:delete  Enter:view  Esc:back  ?:help".to_string()];
    if let Some((msg, _)) = &app.status_message {
        status_parts.push(msg.clone());
    }
    frame.render_widget(
        Paragraph::new(status_parts.join(" | ")).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}
