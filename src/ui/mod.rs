pub mod archive;
pub mod board;
pub mod input;
pub mod keybindings;
pub mod session_view;
pub mod task_detail;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, View};
use keybindings::Keybinding;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.view.clone() {
        View::Board => board::draw(frame, app),
        View::TaskDetail(task_id) => task_detail::draw(frame, app, task_id),
        View::ActiveSession(session_id) => session_view::draw(frame, app, session_id),
        View::Archive => archive::draw(frame, app),
    }

    if app.show_help {
        draw_help_overlay(frame, app);
    }

    if app.show_search {
        draw_search_overlay(frame, app);
    }
}

fn bindings_for_view(view: &View) -> (&'static str, &'static [Keybinding]) {
    match view {
        View::Board => ("Board", keybindings::BOARD),
        View::TaskDetail(_) => ("Task Detail", keybindings::TASK_DETAIL),
        View::ActiveSession(_) => ("Active Session", keybindings::ACTIVE_SESSION),
        View::Archive => ("Archive", keybindings::ARCHIVE),
    }
}

fn draw_help_overlay(frame: &mut Frame, app: &App) {
    let (title, bindings) = bindings_for_view(&app.view);

    let area = frame.area();
    let popup_height = ((bindings.len() + 3) as u16).min(area.height.saturating_sub(4));
    let popup_width = 62u16.min(area.width.saturating_sub(4));
    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    let mut lines: Vec<Line> = Vec::with_capacity(bindings.len() + 2);
    lines.push(Line::from(Span::styled(
        format!("{} keybindings", title),
        Style::default().fg(Color::Yellow).bold(),
    )));
    lines.push(Line::raw(""));
    for kb in bindings {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:14}", kb.keys),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(kb.description),
        ]));
    }

    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Help — press ? to close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup_area);
}

fn draw_search_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_width = (area.width * 60 / 100).min(area.width.saturating_sub(4));
    let popup_height = 20u16.min(area.height.saturating_sub(4));
    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Search tasks ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if inner.height == 0 {
        return;
    }

    let input_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    input::draw_input_line(frame, input_area, "/ ", &app.input_buffer, app.input_cursor);

    if inner.height <= 1 {
        return;
    }

    let results_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: inner.height - 1,
    };

    if app.search_results.is_empty() && !app.input_buffer.is_empty() {
        frame.render_widget(
            Paragraph::new("No results").style(Style::default().fg(Color::DarkGray)),
            results_area,
        );
    } else {
        let max_items = results_area.height as usize;
        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .enumerate()
            .take(max_items)
            .map(|(i, task)| {
                let label = format!("[{}] {}", task.list.display_name(), task.description);
                let style = if i == app.search_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default()
                };
                ListItem::new(label).style(style)
            })
            .collect();
        frame.render_widget(List::new(items), results_area);
    }
}

pub fn status_bar_text(app: &App) -> String {
    let mut parts = Vec::new();

    let (_, bindings) = bindings_for_view(&app.view);
    let hint = bindings
        .iter()
        .take(6)
        .map(|kb| {
            format!(
                "{}:{}",
                kb.keys,
                kb.description.split(' ').next().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("  ");
    parts.push(hint);
    parts.push("?:help".to_string());

    if let Some((msg, _)) = &app.status_message {
        parts.push(msg.clone());
    }

    parts.join(" | ")
}

pub fn session_indicator(app: &App) -> Option<String> {
    if let Some(remaining) = app.session_remaining_secs() {
        let mins = remaining / 60;
        let secs = remaining % 60;
        if remaining > 0 {
            Some(format!("Session: {:02}:{:02}", mins, secs))
        } else {
            Some("Session: DONE".to_string())
        }
    } else {
        None
    }
}
