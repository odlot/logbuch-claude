use std::path::PathBuf;
use std::process;

use anyhow::Result;
use clap::{Parser, Subcommand};

use logbuch::cmd::{log as cmd_log, sessions, tasks, todos};
use logbuch::config::Config;
use logbuch::db;
use logbuch::db::queries;
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
    /// Start a pomodoro session (default 45 min)
    Start {
        /// Task ID
        id: i64,
        /// Session duration in minutes
        #[arg(long, value_name = "N")]
        min: Option<u32>,
    },
    /// Cancel the running session
    Stop,
    /// Start a new session on the most recently worked task
    Resume {
        /// Session duration in minutes (default: config value)
        #[arg(long, value_name = "N")]
        min: Option<u32>,
    },
    /// Attach a timestamped note to the active session (alias: n)
    #[command(alias = "n")]
    Note {
        /// Note text
        text: Vec<String>,
    },
    /// Show the running session and time remaining (exits 1 if no session)
    Status,
    /// Show activity for a date or date range
    ///
    /// Usage:
    ///   logbuch log              # today
    ///   logbuch log --week       # this week (Mon–Sun)
    ///   logbuch log 2026-03-05   # specific date
    ///   logbuch log 2026-03-01 2026-03-07   # date range
    Log {
        /// Start date (yyyy-mm-dd), default: today
        from: Option<String>,
        /// End date (yyyy-mm-dd), default: same as start date
        to: Option<String>,
        /// Show the current week (Mon–Sun)
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
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
            return Ok(());
        }
    };

    // _notify runs without loading the full config
    if let Commands::Notify {
        session_id,
        seconds,
        db: db_path,
    } = &command
    {
        return sessions::notify_process(*session_id, *seconds, db_path);
    }

    let mut config = Config::load(cli.config.as_ref())?;
    if let Some(p) = cli.db {
        config.db_path = p;
    }

    let conn = db::init(&config.db_path)?;

    // Close any sessions left open by a crashed notifier
    let _ = queries::close_orphaned_sessions(&conn);

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
        Commands::Resume { min } => {
            let duration = min.unwrap_or(config.session_duration_min);
            sessions::resume(&conn, duration, &config.db_path)?;
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
        Commands::Log { from, to, week } => {
            cmd_log::run(&conn, &out, from.as_deref(), to.as_deref(), week)?;
        }
        Commands::Notify { .. } => unreachable!(),
    }

    Ok(())
}
