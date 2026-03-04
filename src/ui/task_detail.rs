use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::{App, DetailSection, InputMode, InputTarget};
use crate::db::queries;

pub fn draw(frame: &mut Frame, app: &App, task_id: i64) {
    let task = match queries::get_task(&app.db, task_id) {
        Ok(t) => t,
        Err(_) => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3), // title
        Constraint::Length(5), // description
        Constraint::Min(6),    // todos
        Constraint::Length(8), // sessions
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    // Title
    let title_block = Block::default()
        .title(format!(" Task: {} ", task.description))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let title_text = format!(
        "List: {} | Created: {}",
        task.list.display_name(),
        task.created_at.format("%Y-%m-%d %H:%M")
    );
    let title = Paragraph::new(title_text).block(title_block);
    frame.render_widget(title, chunks[0]);

    // Description
    let desc_style = if app.detail_section == DetailSection::Description {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    if app.input_mode == InputMode::Editing && app.input_target == InputTarget::EditDescription {
        let block = Block::default()
            .title(" Description (editing) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(chunks[1]);
        frame.render_widget(block, chunks[1]);
        super::input::draw_input_line(frame, inner, "", &app.input_buffer, app.input_cursor);
    } else {
        let desc_block = Block::default()
            .title(" Description ")
            .borders(Borders::ALL)
            .border_style(desc_style);
        let desc = Paragraph::new(task.description.as_str())
            .block(desc_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(desc, chunks[1]);
    }

    // Todos
    let todo_style = if app.detail_section == DetailSection::Todos {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let todo_title = format!(" Todos ({}) ", app.todos.len());
    let todo_block = Block::default()
        .title(todo_title)
        .borders(Borders::ALL)
        .border_style(todo_style);

    let inner_area = todo_block.inner(chunks[2]);
    frame.render_widget(todo_block, chunks[2]);

    // Build todo items + optional input line
    let available_height = inner_area.height as usize;
    let items: Vec<ListItem> = app
        .todos
        .iter()
        .enumerate()
        .map(|(i, todo)| {
            let check = if todo.done { "[x]" } else { "[ ]" };
            let marker =
                if app.detail_section == DetailSection::Todos && i == app.selected_todo_index {
                    ">> "
                } else {
                    "   "
                };
            let text = format!("{}{} {}", marker, check, todo.description);

            let style =
                if app.detail_section == DetailSection::Todos && i == app.selected_todo_index {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else if todo.done {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner_area);

    // New todo input
    if app.input_mode == InputMode::Editing && app.input_target == InputTarget::NewTodo {
        let input_y = inner_area.y + app.todos.len().min(available_height.saturating_sub(1)) as u16;
        if input_y < inner_area.y + inner_area.height {
            let input_area = Rect {
                x: inner_area.x,
                y: input_y,
                width: inner_area.width,
                height: 1,
            };
            super::input::draw_input_line(
                frame,
                input_area,
                "New todo: ",
                &app.input_buffer,
                app.input_cursor,
            );
        }
    }

    // Sessions
    let session_style = if app.detail_section == DetailSection::Sessions {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let total_minutes: i64 = app
        .sessions
        .iter()
        .map(|s| {
            if let Some(end) = s.end_at {
                (end - s.begin_at).num_minutes()
            } else {
                s.duration_min as i64
            }
        })
        .sum();
    let session_title = if total_minutes >= 60 {
        format!(
            " Sessions ({}, {}h {}m) ",
            app.sessions.len(),
            total_minutes / 60,
            total_minutes % 60
        )
    } else {
        format!(" Sessions ({}, {}m) ", app.sessions.len(), total_minutes)
    };
    let session_block = Block::default()
        .title(session_title)
        .borders(Borders::ALL)
        .border_style(session_style);

    let session_items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let begin = session.begin_at.format("%Y-%m-%d %H:%M").to_string();
            let end = session
                .end_at
                .map(|e| e.format("%H:%M").to_string())
                .unwrap_or_else(|| "running".to_string());
            let duration = if let Some(end_at) = session.end_at {
                let mins = (end_at - session.begin_at).num_minutes();
                format!("{}m", mins)
            } else {
                format!("{}m", session.duration_min)
            };
            let notes_preview = if session.notes.is_empty() {
                String::new()
            } else {
                let first_line = session.notes.lines().next().unwrap_or("");
                let truncated = if first_line.len() > 30 {
                    format!(" \"{}...\"", &first_line[..27])
                } else {
                    format!(" \"{}\"", first_line)
                };
                truncated
            };

            let marker = if app.detail_section == DetailSection::Sessions
                && i == app.selected_session_index
            {
                ">> "
            } else {
                "   "
            };

            let text = format!(
                "{}{} - {} ({}){}",
                marker, begin, end, duration, notes_preview
            );
            let style = if app.detail_section == DetailSection::Sessions
                && i == app.selected_session_index
            {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let session_list = List::new(session_items).block(session_block);
    frame.render_widget(session_list, chunks[3]);

    // Status bar
    draw_detail_status(frame, app, chunks[4]);
}

fn draw_detail_status(frame: &mut Frame, app: &App, area: Rect) {
    // Session duration prompt replaces the status bar
    if app.input_mode == InputMode::Editing && app.input_target == InputTarget::SessionDuration {
        super::input::draw_input_line(
            frame,
            area,
            "Session duration (min): ",
            &app.input_buffer,
            app.input_cursor,
        );
        return;
    }

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
