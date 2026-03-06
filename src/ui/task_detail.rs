use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::{App, DetailSection, InputMode, InputTarget};
use crate::db::queries;

pub fn draw(frame: &mut Frame, app: &mut App, task_id: i64) {
    let task = match queries::get_task(&app.db, task_id) {
        Ok(t) => t,
        Err(_) => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3), // title
        Constraint::Length(5), // description
        Constraint::Min(6),    // todos
        Constraint::Length(6), // sessions
        Constraint::Length(3), // session notes
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

    let selected_todo = app.todo_list_state.selected().unwrap_or(0);
    let is_editing_todo = app.input_mode == InputMode::Editing
        && (app.input_target == InputTarget::EditTodo || app.input_target == InputTarget::NewTodo);

    let items: Vec<ListItem> = app
        .todos
        .iter()
        .enumerate()
        .map(|(i, todo)| {
            let check = if todo.done { "[x]" } else { "[ ]" };
            let is_selected = app.detail_section == DetailSection::Todos && i == selected_todo;
            let marker = if is_selected { ">> " } else { "   " };

            // Show editing indicator for selected item when in EditTodo mode
            let text = if is_selected
                && app.input_mode == InputMode::Editing
                && app.input_target == InputTarget::EditTodo
            {
                format!("{}{}  ", marker, check)
            } else {
                format!("{}{} {}", marker, check, todo.description)
            };

            let style = if is_selected && app.detail_section == DetailSection::Todos {
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
    frame.render_stateful_widget(list, inner_area, &mut app.todo_list_state);

    // Inline edit input for EditTodo — overlay on the selected row
    if app.input_mode == InputMode::Editing && app.input_target == InputTarget::EditTodo {
        let scroll = app.todo_list_state.offset();
        let row = selected_todo.saturating_sub(scroll);
        if row < inner_area.height as usize {
            let edit_area = Rect {
                x: inner_area.x + 6, // after ">> [x] "
                y: inner_area.y + row as u16,
                width: inner_area.width.saturating_sub(6),
                height: 1,
            };
            super::input::draw_input_line(
                frame,
                edit_area,
                "",
                &app.input_buffer,
                app.input_cursor,
            );
        }
    }

    // New todo input line at the bottom of the todo area
    if app.input_mode == InputMode::Editing && app.input_target == InputTarget::NewTodo {
        let available_height = inner_area.height as usize;
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

    // Suppress unused variable warning when neither editing mode is active
    let _ = is_editing_todo;

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

    let selected_session = app.session_list_state.selected().unwrap_or(0);

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
                let truncated = if first_line.chars().count() > 30 {
                    let t: String = first_line.chars().take(27).collect();
                    format!(" \"{}...\"", t)
                } else {
                    format!(" \"{}\"", first_line)
                };
                truncated
            };

            let is_selected =
                app.detail_section == DetailSection::Sessions && i == selected_session;
            let marker = if is_selected { ">> " } else { "   " };

            let text = format!(
                "{}{} - {} ({}){}",
                marker, begin, end, duration, notes_preview
            );
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let session_inner = session_block.inner(chunks[3]);
    let session_list = List::new(session_items).block(session_block);
    frame.render_stateful_widget(session_list, chunks[3], &mut app.session_list_state);
    let _ = session_inner;

    // Session notes panel
    let notes_text = app
        .sessions
        .get(selected_session)
        .map(|s| {
            if s.notes.is_empty() {
                "(no notes)".to_string()
            } else {
                s.notes.clone()
            }
        })
        .unwrap_or_else(|| "(no sessions)".to_string());
    let notes_style = if app.detail_section == DetailSection::Sessions {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let notes_block = Block::default()
        .title(" Session Notes ")
        .borders(Borders::ALL)
        .border_style(notes_style);
    let notes_paragraph = Paragraph::new(notes_text)
        .block(notes_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(notes_paragraph, chunks[4]);

    // Status bar
    draw_detail_status(frame, app, chunks[5]);
}

fn draw_detail_status(frame: &mut Frame, app: &App, area: Rect) {
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

    super::draw_status_bar(frame, app, area);
}
