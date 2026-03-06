use std::path::PathBuf;
use std::process;

use anyhow::Result;
use clap::{Parser, Subcommand};

use logbuch::cmd::{log as cmd_log, sessions, tasks, todos};
use logbuch::config::Config;
use logbuch::db;
use logbuch::output::Out;

#[derive(Parser)]
#[command(
    name = "logbuch",
    version,
    about = "Developer productivity — tasks, todos, pomodoro sessions"
)]
struct Cli {
    /// Path to config file (default: ~/.config/logbuch/config.toml)
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Override the SQLite database path (env: LOGBUCH_DB_PATH)
    #[arg(long, global = true, value_name = "PATH")]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a task to the inbox
    Add {
        /// Task description
        description: Vec<String>,
    },
    /// Show all tasks (alias: ls)
    #[command(alias = "ls")]
    List,
    /// Full task detail: todos and sessions
    Show {
        /// Task ID
        id: i64,
    },
    /// Mark a task complete and remove it
    Done {
        /// Task ID
        id: i64,
    },
    /// Delete a task (prompts unless --yes)
    Rm {
        /// Task ID
        id: i64,
        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Move a task to the backlog
    Defer {
        /// Task ID
        id: i64,
    },
    /// Rename a task, or a todo when <todo-id> is given
    ///
    /// Usage:
    ///   logbuch edit <task-id> <description>
    ///   logbuch edit <task-id> <todo-id> <description>
    Edit {
        /// Task ID
        task_id: i64,
        /// Description words (optionally prefixed by a todo ID)
        args: Vec<String>,
    },
    /// Add a todo to a task
    Todo {
        /// Task ID
        task_id: i64,
        /// Todo description
        description: Vec<String>,
    },
    /// Toggle a todo done/undone
    Check {
        /// Task ID
        task_id: i64,
        /// Todo ID
        todo_id: i64,
    },
    /// Start a pomodoro session (default 25 min)
    Start {
        /// Task ID
        id: i64,
        /// Session duration in minutes
        #[arg(long, value_name = "N")]
        min: Option<u32>,
    },
    /// Cancel the running session
    Stop,
    /// Attach a timestamped note to the active session (alias: n)
    #[command(alias = "n")]
    Note {
        /// Note text
        text: Vec<String>,
    },
    /// Show the running session and time remaining (exits 1 if no session)
    Status,
    /// Daily or weekly summary
    Log {
        /// Show this week instead of today
        #[arg(long)]
        week: bool,
    },
    /// Internal: detached notifier process (hidden)
    #[command(hide = true)]
    Notify {
        #[arg(long)]
        session_id: i64,
        #[arg(long)]
        seconds: u64,
        #[arg(long, value_name = "PATH")]
        db: PathBuf,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let command = match cli.command {
        Some(c) => c,
        None => {
            // No subcommand: print usage and exit 0
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
            return Ok(());
        }
    };

    // The hidden _notify subcommand runs without loading the full config.
    if let Commands::Notify {
        session_id,
        seconds,
        db: db_path,
    } = &command
    {
        return sessions::notify_process(*session_id, *seconds, db_path);
    }

    // Load config (env vars + config file + CLI flag overrides)
    let mut config = Config::load(cli.config.as_ref())?;
    if let Some(p) = cli.db {
        config.db_path = p;
    }

    let conn = db::init(&config.db_path)?;
    let out = Out::new();

    match command {
        Commands::Add { description } => {
            tasks::add(&conn, &description.join(" "))?;
        }
        Commands::List => {
            tasks::list(&conn, &out)?;
        }
        Commands::Show { id } => {
            tasks::show(&conn, id, &out)?;
        }
        Commands::Done { id } => {
            tasks::done(&conn, id)?;
        }
        Commands::Rm { id, yes } => {
            tasks::rm(&conn, id, yes)?;
        }
        Commands::Defer { id } => {
            tasks::defer(&conn, id)?;
        }
        Commands::Edit { task_id, args } => {
            tasks::edit(&conn, task_id, &args)?;
        }
        Commands::Todo {
            task_id,
            description,
        } => {
            todos::add_todo(&conn, task_id, &description.join(" "))?;
        }
        Commands::Check { task_id, todo_id } => {
            todos::check(&conn, task_id, todo_id)?;
        }
        Commands::Start { id, min } => {
            let duration = min.unwrap_or(config.session_duration_min);
            sessions::start(&conn, id, duration, &config.db_path)?;
        }
        Commands::Stop => {
            sessions::stop(&conn, &config.db_path)?;
        }
        Commands::Note { text } => {
            sessions::note(&conn, &text.join(" "))?;
        }
        Commands::Status => {
            let exit_code = sessions::status(&conn, &out)?;
            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
        Commands::Log { week } => {
            if week {
                cmd_log::log_weekly(&conn, &out)?;
            } else {
                cmd_log::log_daily(&conn, &out)?;
            }
        }
        Commands::Notify { .. } => unreachable!(),
    }

    Ok(())
}
