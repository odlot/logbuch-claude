use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use rusqlite::Connection;

use crate::db::queries;
use crate::output::Out;

/// Path to the notifier PID file (sibling of the DB file).
pub fn pid_path(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("notify.pid")
}

pub fn start(conn: &Connection, task_id: i64, duration_min: u32, db_path: &Path) -> Result<()> {
    if let Some(active) = queries::get_active_session(conn)? {
        let task = queries::get_task(conn, active.task_id)?;
        bail!(
            "Session already running for #{} \"{}\". Run `logbuch stop` first.",
            active.task_id,
            task.description
        );
    }

    let task = queries::get_task(conn, task_id)?;
    let session_id = queries::start_session(conn, task_id, duration_min as i32)?;

    // Move task to In Progress if it isn't already
    if task.list != crate::model::TaskList::InProgress {
        queries::move_task(conn, task_id, &crate::model::TaskList::InProgress)?;
    }

    let seconds = (duration_min as u64) * 60;
    let exe = std::env::current_exe().context("cannot resolve current executable path")?;
    let child = std::process::Command::new(&exe)
        .args([
            "_notify",
            "--session-id",
            &session_id.to_string(),
            "--seconds",
            &seconds.to_string(),
            "--db",
            &db_path.to_string_lossy(),
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to spawn notifier process")?;

    let pid = child.id();
    let pid_file = pid_path(db_path);
    std::fs::write(&pid_file, pid.to_string())
        .with_context(|| format!("writing PID file {}", pid_file.display()))?;

    println!(
        "Session started: #{} \"{}\" ({}min)",
        task_id, task.description, duration_min
    );
    Ok(())
}

pub fn stop(conn: &Connection, db_path: &Path) -> Result<()> {
    let active = queries::get_active_session(conn)?
        .ok_or_else(|| anyhow::anyhow!("No session is running."))?;

    // Kill the notifier if its PID file exists
    let pid_file = pid_path(db_path);
    if pid_file.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // SIGTERM on Unix; on Windows this is a no-op but we close the session anyway
                #[cfg(unix)]
                {
                    unsafe { libc_kill(pid) };
                }
                #[cfg(not(unix))]
                {
                    let _ = pid; // unused on non-Unix
                }
            }
        }
        let _ = std::fs::remove_file(&pid_file);
    }

    queries::end_session(conn, active.id)?;
    let task = queries::get_task(conn, active.task_id)?;
    println!(
        "Stopped session for #{} \"{}\".",
        active.task_id, task.description
    );
    Ok(())
}

#[cfg(unix)]
unsafe fn libc_kill(pid: u32) {
    // Send SIGTERM to the notifier process. We use libc::kill via a raw syscall
    // to avoid adding a libc dependency — on Linux/macOS the syscall number is stable.
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    kill(pid as i32, 15 /* SIGTERM */);
}

pub fn note(conn: &Connection, text: &str) -> Result<()> {
    let active = queries::get_active_session(conn)?.ok_or_else(|| {
        anyhow::anyhow!("No session is running. Start one with `logbuch start <id>`.")
    })?;

    let timestamp = chrono::Local::now().format("%H:%M").to_string();
    let note = format!("{}  {}", timestamp, text);
    queries::append_session_notes(conn, active.id, &note)?;
    println!("Note added.");
    Ok(())
}

/// Returns 0 if a session is running, 1 if not.
pub fn status(conn: &Connection, out: &Out) -> Result<i32> {
    match queries::get_active_session(conn)? {
        None => Ok(1),
        Some(session) => {
            let task = queries::get_task(conn, session.task_id)?;
            let begin = session.begin_at;
            let end = begin + chrono::Duration::minutes(session.duration_min as i64);
            let now = chrono::Local::now().naive_local();
            let remaining = (end - now).num_seconds().max(0);
            let mins = remaining / 60;
            let secs = remaining % 60;

            println!(
                "#{} \"{}\"  {}",
                session.task_id,
                task.description,
                out.green(&format!("{:02}:{:02} remaining", mins, secs))
            );
            Ok(0)
        }
    }
}

/// Internal hidden subcommand: sleeps, then marks the session complete and fires a notification.
pub fn notify_process(session_id: i64, seconds: u64, db_path: &Path) -> Result<()> {
    std::thread::sleep(std::time::Duration::from_secs(seconds));

    let conn = crate::db::init(db_path)?;
    queries::end_session(&conn, session_id)?;

    // Fetch task description for the notification
    let task_desc = conn
        .query_row(
            "SELECT t.description FROM task t \
             JOIN session s ON s.task_id = t.id \
             WHERE s.id = ?1",
            rusqlite::params![session_id],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "task".to_string());

    // Remove PID file
    let pid_file = pid_path(db_path);
    let _ = std::fs::remove_file(&pid_file);

    // Fire desktop notification
    let _ = notify_rust::Notification::new()
        .summary("Pomodoro done!")
        .body(&format!("Session complete: {}", task_desc))
        .timeout(notify_rust::Timeout::Milliseconds(8000))
        .show();

    Ok(())
}
