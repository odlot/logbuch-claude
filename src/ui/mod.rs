pub mod archive;
pub mod board;
pub mod input;
pub mod session_view;
pub mod task_detail;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.view.clone() {
        crate::app::View::Board => board::draw(frame, app),
        crate::app::View::TaskDetail(task_id) => task_detail::draw(frame, app, task_id),
        crate::app::View::ActiveSession(session_id) => session_view::draw(frame, app, session_id),
        crate::app::View::Archive => archive::draw(frame, app),
    }

    if app.show_help {
        draw_help_overlay(frame, app);
    }

    if app.show_search {
        draw_search_overlay(frame, app);
    }
}

fn draw_help_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 22.min(area.height.saturating_sub(4));
    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    let help_text = match &app.view {
        crate::app::View::Board => {
            "Board View\n\n\
             h/Left    Focus left column\n\
             l/Right   Focus right column\n\
             j/Down    Select next task\n\
             k/Up      Select previous task\n\
             Enter     Open task detail\n\
             n         New task\n\
             d         Delete task (confirm: d)\n\
             A (Shift) Archive task\n\
             H (Shift) Move task left\n\
             L (Shift) Move task right\n\
             a         Open archive view\n\
             r d       Daily summary\n\
             r w       Weekly summary\n\
             /         Search tasks\n\
             q         Quit\n\
             ?         Toggle help"
        }
        crate::app::View::TaskDetail(_) => {
            "Task Detail View\n\n\
             Esc         Back to board\n\
             Tab         Next section\n\
             Shift+Tab   Previous section\n\
             j/k         Navigate in section\n\
             e           Edit description / todo\n\
             a           Add todo\n\
             x           Toggle todo\n\
             D           Delete todo/session (confirm: D)\n\
             J/K         Move todo up/down\n\
             s           Start session\n\
             /           Search tasks\n\
             ?           Toggle help"
        }
        crate::app::View::ActiveSession(_) => {
            "Active Session View\n\n\
             Esc       End session\n\
             Enter     Submit note\n\
             Type      Add notes\n\
             ?         Toggle help"
        }
        crate::app::View::Archive => {
            "Archive View\n\n\
             j/Down    Select next task\n\
             k/Up      Select previous task\n\
             Enter     View task detail\n\
             r         Restore task to Inbox\n\
             d         Delete permanently (confirm: d)\n\
             Esc       Back to board\n\
             ?         Toggle help"
        }
    };

    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let paragraph = Paragraph::new(help_text)
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

    // Input line
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

    // Results list
    let results_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: inner.height - 1,
    };

    if app.search_results.is_empty() && !app.input_buffer.is_empty() {
        let no_results = Paragraph::new("No results").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(no_results, results_area);
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
        let list = List::new(items);
        frame.render_widget(list, results_area);
    }
}

pub fn status_bar_text(app: &App) -> String {
    let mut parts = Vec::new();

    match &app.view {
        crate::app::View::Board => parts.push(
            "n:new  d:del  A:archive  a:view-archive  H/L:move  Enter:open  ?:help".to_string(),
        ),
        crate::app::View::Archive => {
            parts.push("r:restore  d:delete  Enter:view  Esc:back  ?:help".to_string())
        }
        crate::app::View::TaskDetail(_) => parts
            .push("e:edit  a:todo  x:toggle  D:delete  s:session  Esc:back  ?:help".to_string()),
        crate::app::View::ActiveSession(_) => parts.push("Enter:note  Esc:end  ?:help".to_string()),
    };

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
