mod app;
mod config;
mod db;
mod event;
mod model;
mod summary;
mod ui;
mod wizard;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use app::App;
use config::Config;
use event::{Event, EventHandler};

#[derive(Parser)]
#[command(
    name = "logbuch",
    version,
    about = "TUI task management with pomodoro sessions"
)]
struct Cli {
    /// Path to config file (default: ~/.config/logbuch/config.toml)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Override the SQLite database path (also: LOGBUCH_DB_PATH)
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,

    /// Override the summary report output directory (also: LOGBUCH_SUMMARY_DIR)
    #[arg(long, value_name = "DIR")]
    summary_dir: Option<PathBuf>,

    /// Override the default session duration in minutes (also: LOGBUCH_SESSION_DURATION)
    #[arg(long, value_name = "MINUTES")]
    session_duration: Option<u32>,

    /// Print effective configuration and exit
    #[arg(long)]
    show_config: bool,

    /// Generate a summary report instead of launching TUI
    #[arg(long, value_enum)]
    summary: Option<SummaryKind>,
}

#[derive(Clone, ValueEnum)]
enum SummaryKind {
    Daily,
    Weekly,
}

fn main() -> Result<()> {
    // Restore the terminal if the app panics so the shell is not left broken.
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        orig_hook(info);
    }));

    let cli = Cli::parse();
    let config_path = config::default_config_path();
    let effective_config_path = cli.config.as_ref().unwrap_or(&config_path);

    // First run: no config file and no explicit --config override → show wizard.
    let mut config = if cli.config.is_none() && !effective_config_path.exists() {
        wizard::run(effective_config_path)?
    } else {
        Config::load(cli.config.as_ref())?
    };

    // Apply CLI flag overrides (highest priority, after env vars)
    if let Some(p) = cli.db_path {
        config.db_path = p;
    }
    if let Some(p) = cli.summary_dir {
        config.summary_export_dir = p;
    }
    if let Some(d) = cli.session_duration {
        config.session_duration_min = d;
    }

    // --show-config: print effective config and exit
    if cli.show_config {
        config.print_summary(effective_config_path);
        return Ok(());
    }

    // Handle headless summary generation
    if let Some(kind) = cli.summary {
        let conn = db::init(&config.db_path)?;
        let today = chrono::Local::now().date_naive();
        let path = match kind {
            SummaryKind::Daily => {
                summary::generate_daily(&conn, today, &config.summary_export_dir)?
            }
            SummaryKind::Weekly => {
                summary::generate_weekly(&conn, today, &config.summary_export_dir)?
            }
        };
        println!("Summary written to: {}", path.display());
        return Ok(());
    }

    // Initialize DB
    let conn = db::init(&config.db_path)?;

    // Close any orphaned sessions from a previous crash
    let orphaned = db::queries::close_orphaned_sessions(&conn)?;
    if orphaned > 0 {
        eprintln!("Closed {} orphaned session(s) from previous run", orphaned);
    }

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let mut app = App::new(conn, config)?;
    let events = EventHandler::new(Duration::from_millis(250));

    // Main loop
    let result = run_app(&mut terminal, &mut app, &events);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    events: &EventHandler,
) -> Result<()> {
    while app.running {
        terminal.draw(|frame| ui::draw(frame, app))?;

        match events.next()? {
            Event::Key(key) => {
                // Only handle key press events (not release/repeat)
                if key.kind == crossterm::event::KeyEventKind::Press {
                    app.handle_key(key)?;
                }
            }
            Event::Tick => {
                app.tick()?;
            }
            Event::Resize => {
                // Terminal handles resize automatically on next draw
            }
        }
    }
    Ok(())
}
