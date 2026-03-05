//! First-run setup wizard.
//!
//! Shown the very first time Logbuch starts (no config file found). Lets the
//! user confirm or change the three configurable paths/values, then writes the
//! config file so future runs skip the wizard entirely.

use std::io;
use std::path::Path;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::config::Config;

// ── wizard state ─────────────────────────────────────────────────────────────

struct Field {
    label: &'static str,
    hint: &'static str,
    value: String,
    cursor: usize,
}

impl Field {
    fn new(label: &'static str, hint: &'static str, default: String) -> Self {
        let cursor = default.len();
        Self {
            label,
            hint,
            value: default,
            cursor,
        }
    }

    fn insert(&mut self, ch: char) {
        self.value.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            // step back one char boundary
            let new_cursor = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.remove(new_cursor);
            self.cursor = new_cursor;
        }
    }
}

struct WizardState {
    fields: Vec<Field>,
    step: usize,   // 0..fields.len() = input steps; fields.len() = confirm step
    aborted: bool, // user pressed Esc — use defaults
}

impl WizardState {
    fn new(defaults: &Config) -> Self {
        Self {
            fields: vec![
                Field::new(
                    "Database path",
                    "SQLite file that stores your tasks, todos, and sessions.",
                    defaults.db_path.to_string_lossy().into_owned(),
                ),
                Field::new(
                    "Reports directory",
                    "Where daily/weekly Markdown reports are written.",
                    defaults.summary_export_dir.to_string_lossy().into_owned(),
                ),
                Field::new(
                    "Session duration (minutes)",
                    "Default length of a new Pomodoro work session.",
                    defaults.session_duration_min.to_string(),
                ),
            ],
            step: 0,
            aborted: false,
        }
    }

    fn current(&self) -> &Field {
        &self.fields[self.step]
    }

    fn current_mut(&mut self) -> &mut Field {
        &mut self.fields[self.step]
    }

    fn on_confirm_step(&self) -> bool {
        self.step == self.fields.len()
    }

    fn done(&self) -> bool {
        self.aborted || self.on_confirm_step()
    }

    /// Build a Config from the wizard's final field values.
    fn into_config(self) -> Config {
        let mut cfg = Config::default();
        let db = self.fields[0].value.trim().to_string();
        let reports = self.fields[1].value.trim().to_string();
        let duration = self.fields[2].value.trim().parse::<u32>().unwrap_or(25);

        if !db.is_empty() {
            cfg.db_path = crate::config::expand_tilde(db.into());
        }
        if !reports.is_empty() {
            cfg.summary_export_dir = crate::config::expand_tilde(reports.into());
        }
        cfg.session_duration_min = duration;
        cfg
    }
}

// ── public entry point ────────────────────────────────────────────────────────

/// Run the first-run wizard, write the resulting config to `config_path`, and
/// return the chosen `Config`. If the user presses `Esc`, defaults are used.
pub fn run(config_path: &Path) -> Result<Config> {
    let defaults = Config::default();
    let mut state = WizardState::new(&defaults);

    // Set up a dedicated terminal session for the wizard.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_wizard_loop(&mut terminal, &mut state);

    // Always restore the terminal, even on error.
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result?;

    let config = if state.aborted {
        defaults
    } else {
        state.into_config()
    };

    // Persist the chosen values so the wizard never runs again.
    config.write_to(config_path)?;

    Ok(config)
}

// ── event loop ────────────────────────────────────────────────────────────────

fn run_wizard_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut WizardState,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, state))?;

        if state.done() {
            break;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc => {
                    state.aborted = true;
                }
                KeyCode::Enter => {
                    state.step += 1;
                }
                KeyCode::Backspace => {
                    if !state.on_confirm_step() {
                        state.current_mut().backspace();
                    }
                }
                KeyCode::Char(ch) => {
                    if !state.on_confirm_step() {
                        state.current_mut().insert(ch);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

// ── rendering ─────────────────────────────────────────────────────────────────

fn draw(frame: &mut Frame, state: &WizardState) {
    let area = frame.area();

    // Dim background
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    // Centered dialog: 60 wide, tall enough for content
    let dialog_width = area.width.min(64);
    let dialog_height = 14u16;
    let x = area.width.saturating_sub(dialog_width) / 2;
    let y = area.height.saturating_sub(dialog_height) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let total = state.fields.len();
    let title = if state.aborted {
        " Logbuch — Setup skipped ".to_string()
    } else if state.on_confirm_step() {
        " Logbuch — Setup complete ".to_string()
    } else {
        format!(
            " Logbuch — First-time setup  [{}/{}] ",
            state.step + 1,
            total
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title)
        .title_alignment(Alignment::Center);

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // intro / step label
            Constraint::Length(2), // hint
            Constraint::Length(3), // input box
            Constraint::Min(1),    // spacer
            Constraint::Length(1), // footer
        ])
        .split(inner.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }));

    if state.aborted || state.on_confirm_step() {
        draw_finish(frame, state, chunks);
    } else {
        draw_step(frame, state, chunks);
    }
}

fn draw_step(frame: &mut Frame, state: &WizardState, chunks: std::rc::Rc<[Rect]>) {
    let field = state.current();

    // Field label
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(field.label, Style::default().fg(Color::Yellow).bold()),
        ])),
        chunks[0],
    );

    // Hint text
    frame.render_widget(
        Paragraph::new(field.hint)
            .style(Style::default().fg(Color::DarkGray))
            .wrap(Wrap { trim: true }),
        chunks[1],
    );

    // Input box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let input_inner = input_block.inner(chunks[2]);
    frame.render_widget(input_block, chunks[2]);
    frame.render_widget(
        Paragraph::new(field.value.as_str()).style(Style::default().fg(Color::White)),
        input_inner,
    );

    // Cursor
    frame.set_cursor_position(Position {
        x: input_inner.x + field.cursor as u16,
        y: input_inner.y,
    });

    // Footer
    frame.render_widget(
        Paragraph::new("[Enter] confirm   [Esc] skip setup")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[4],
    );
}

fn draw_finish(frame: &mut Frame, state: &WizardState, chunks: std::rc::Rc<[Rect]>) {
    let (msg, color) = if state.aborted {
        (
            "Setup skipped — using built-in defaults.\nYou can edit ~/.config/logbuch/config.toml at any time.",
            Color::Yellow,
        )
    } else {
        (
            "Configuration saved!\nPress Enter to launch Logbuch.",
            Color::Green,
        )
    };

    frame.render_widget(
        Paragraph::new(msg)
            .style(Style::default().fg(color))
            .wrap(Wrap { trim: true }),
        chunks[0],
    );

    if state.on_confirm_step() {
        frame.render_widget(
            Paragraph::new("[Enter] launch")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            chunks[4],
        );
    }
}
