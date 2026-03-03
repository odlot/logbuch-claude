pub mod board;
pub mod input;
pub mod session_view;
pub mod task_detail;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    match &app.view {
        crate::app::View::Board => board::draw(frame, app),
        crate::app::View::TaskDetail(task_id) => task_detail::draw(frame, app, *task_id),
        crate::app::View::ActiveSession(session_id) => session_view::draw(frame, app, *session_id),
    }

    if app.show_help {
        draw_help_overlay(frame, app);
    }
}

fn draw_help_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 20.min(area.height.saturating_sub(4));
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
             d         Delete task\n\
             H (Shift) Move task left\n\
             L (Shift) Move task right\n\
             r d       Daily summary\n\
             r w       Weekly summary\n\
             q         Quit\n\
             ?         Toggle help"
        }
        crate::app::View::TaskDetail(_) => {
            "Task Detail View\n\n\
             Esc         Back to board\n\
             Tab         Next section\n\
             Shift+Tab   Previous section\n\
             j/k         Navigate in section\n\
             e           Edit description\n\
             a           Add todo\n\
             x           Toggle todo\n\
             D           Delete todo\n\
             s           Start session\n\
             ?           Toggle help"
        }
        crate::app::View::ActiveSession(_) => {
            "Active Session View\n\n\
             Esc       End session\n\
             Enter     Submit note\n\
             Type      Add notes\n\
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

pub fn status_bar_text(app: &App) -> String {
    let mut parts = Vec::new();

    // View name
    match &app.view {
        crate::app::View::Board => parts.push("Board".to_string()),
        crate::app::View::TaskDetail(_) => parts.push("Task Detail".to_string()),
        crate::app::View::ActiveSession(_) => parts.push("Session".to_string()),
    };

    // Status message
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
