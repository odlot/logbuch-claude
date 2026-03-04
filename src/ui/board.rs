use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::{App, InputMode, InputTarget};
use crate::model::TaskList;

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(1),    // board
        Constraint::Length(1), // input or status bar
    ])
    .split(frame.area());

    let columns = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(chunks[0]);

    draw_column(frame, app, &TaskList::Inbox, columns[0]);
    draw_column(frame, app, &TaskList::InProgress, columns[1]);
    draw_column(frame, app, &TaskList::Backlog, columns[2]);

    // Bottom bar: input or status
    draw_bottom_bar(frame, app, chunks[1]);
}

fn draw_column(frame: &mut Frame, app: &App, list: &TaskList, area: Rect) {
    let is_active = app.active_column == *list;
    let tasks = app.tasks_for_list(list);
    let selected = app.selected_index[list.index()];

    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" {} ({}) ", list.display_name(), tasks.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let marker = if is_active && i == selected {
                ">> "
            } else {
                "   "
            };
            let desc = if task.description.len() > 40 {
                format!("{}...", &task.description[..37])
            } else {
                task.description.clone()
            };
            let content = format!("{}{}", marker, desc);

            let style = if is_active && i == selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let inner = block.inner(area);
    let list_widget = List::new(items).block(block);
    frame.render_widget(list_widget, area);

    // Empty-state hint
    if tasks.is_empty() && is_active {
        let hint = Paragraph::new("Press n to add a task")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        let hint_area = Rect {
            x: inner.x,
            y: inner.y + inner.height / 2,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(hint, hint_area);
    }
}

fn draw_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    if app.input_mode == InputMode::Editing && app.input_target == InputTarget::NewTask {
        let input = Paragraph::new(format!("New task: {}", app.input_buffer))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(input, area);

        // Show cursor
        frame.set_cursor_position(Position::new(
            area.x + "New task: ".len() as u16 + app.input_cursor as u16,
            area.y,
        ));
    } else {
        let status_left = super::status_bar_text(app);
        let status_right = super::session_indicator(app).unwrap_or_default();

        let left_width = area.width.saturating_sub(status_right.len() as u16 + 1);

        let bar_chunks =
            Layout::horizontal([Constraint::Length(left_width), Constraint::Min(1)]).split(area);

        let left = Paragraph::new(status_left).style(Style::default().fg(Color::DarkGray));
        let right = Paragraph::new(status_right)
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Right);

        frame.render_widget(left, bar_chunks[0]);
        frame.render_widget(right, bar_chunks[1]);
    }
}
