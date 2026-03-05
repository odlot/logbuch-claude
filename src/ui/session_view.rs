use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};

use crate::app::{App, InputMode};

pub fn draw(frame: &mut Frame, app: &mut App, _session_id: i64) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // title
        Constraint::Length(5), // timer
        Constraint::Length(3), // progress bar
        Constraint::Min(6),    // notes
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    // Title
    let title_block = Block::default()
        .title(format!(" Session: {} ", app.session_task_description))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let duration_text = if let Some(ref session) = app.active_session {
        format!("Duration: {} minutes", session.duration_min)
    } else {
        String::new()
    };
    let title = Paragraph::new(duration_text)
        .block(title_block)
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Timer display
    let remaining = app.session_remaining_secs().unwrap_or(0);
    let is_done = remaining == 0 && app.notification_sent;
    let mins = remaining / 60;
    let secs = remaining % 60;

    let timer_text = if is_done {
        "00:00 — DONE".to_string()
    } else {
        format!("{:02}:{:02}", mins, secs)
    };

    let timer_style = if is_done {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    };

    let timer_block = Block::default().borders(Borders::NONE);
    let timer = Paragraph::new(format!("\n{}\nremaining", timer_text))
        .block(timer_block)
        .alignment(Alignment::Center)
        .style(timer_style);
    frame.render_widget(timer, chunks[1]);

    // Progress bar
    let progress = app.session_progress().unwrap_or(0.0);
    let gauge_color = if is_done { Color::Green } else { Color::Cyan };
    let gauge_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let gauge = Gauge::default()
        .block(gauge_block)
        .gauge_style(Style::default().fg(gauge_color))
        .ratio(progress);
    frame.render_widget(gauge, chunks[2]);

    // Notes
    let notes_block = Block::default()
        .title(" Notes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = notes_block.inner(chunks[3]);
    frame.render_widget(notes_block, chunks[3]);

    let existing_notes = app
        .active_session
        .as_ref()
        .map(|s| s.notes.as_str())
        .unwrap_or("");

    // Split inner area: notes display + input line
    if inner.height >= 2 {
        let notes_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        };
        let input_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };

        let notes = Paragraph::new(existing_notes)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));
        frame.render_widget(notes, notes_area);

        if app.input_mode == InputMode::Editing {
            let input_text = format!("> {}", app.input_buffer);
            let input = Paragraph::new(input_text).style(Style::default().fg(Color::Yellow));
            frame.render_widget(input, input_area);

            frame.set_cursor_position(Position::new(
                input_area.x + "> ".len() as u16 + app.input_cursor as u16,
                input_area.y,
            ));
        } else {
            let prompt = Paragraph::new("> (press Enter to add note)")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(prompt, input_area);
        }
    }

    // Status bar
    let status = super::status_bar_text(app);
    let indicator = super::session_indicator(app).unwrap_or_default();

    let left_width = chunks[4].width.saturating_sub(indicator.len() as u16 + 1);
    let bar_chunks =
        Layout::horizontal([Constraint::Length(left_width), Constraint::Min(1)]).split(chunks[4]);

    let left = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    let right = Paragraph::new(indicator)
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Right);
    frame.render_widget(left, bar_chunks[0]);
    frame.render_widget(right, bar_chunks[1]);
}
