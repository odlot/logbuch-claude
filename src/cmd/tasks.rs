use anyhow::{bail, Result};
use rusqlite::Connection;

use crate::db::queries;
use crate::model::TaskList;
use crate::output::Out;

pub fn add(conn: &Connection, description: &str) -> Result<()> {
    let id = queries::insert_task(conn, description, &TaskList::Inbox)?;
    println!("Added  #{:<4} {}", id, description);
    Ok(())
}

pub fn list(conn: &Connection, out: &Out) -> Result<()> {
    let inbox = queries::list_tasks(conn, &TaskList::Inbox)?;
    let in_progress = queries::list_tasks(conn, &TaskList::InProgress)?;
    let backlog = queries::list_tasks(conn, &TaskList::Backlog)?;

    let active = queries::get_active_session(conn)?;
    let mut printed_any = false;

    // Inbox
    if !inbox.is_empty() {
        println!();
        println!("  {}", out.bold("Inbox"));
        for task in &inbox {
            println!("  #{:<4} {}", task.id, task.description);
        }
        printed_any = true;
    }

    // In Progress
    if !in_progress.is_empty() {
        println!();
        println!("  {}", out.bold("In Progress"));
        for task in &in_progress {
            let timer = active.as_ref().and_then(|s| {
                if s.task_id == task.id {
                    let begin = s.begin_at;
                    let end = begin + chrono::Duration::minutes(s.duration_min as i64);
                    let now = chrono::Local::now().naive_local();
                    let remaining = (end - now).num_seconds().max(0);
                    let mins = remaining / 60;
                    let secs = remaining % 60;
                    Some(format!("▶ {:02}:{:02} remaining", mins, secs))
                } else {
                    None
                }
            });

            if let Some(ref t) = timer {
                println!(
                    "  #{:<4} {:<40} {}",
                    task.id,
                    task.description,
                    out.green(t)
                );
            } else {
                println!("  #{:<4} {}", task.id, task.description);
            }

            // Show todos inline for in-progress tasks
            let todos = queries::list_todos(conn, task.id)?;
            for todo in &todos {
                let check = if todo.done { "[x]" } else { "[ ]" };
                let text = format!("{} {}  {}", check, todo.id, todo.description);
                let line = if todo.done { out.dim(&text) } else { text };
                println!("      {}", line);
            }
        }
        printed_any = true;
    }

    // Backlog
    if !backlog.is_empty() {
        println!();
        println!("  {}", out.bold("Backlog"));
        for task in &backlog {
            println!("  #{:<4} {}", task.id, task.description);
        }
        printed_any = true;
    }

    if printed_any {
        println!();
    } else {
        println!("No tasks yet. Use `logbuch add <description>` to create one.");
    }

    Ok(())
}

pub fn show(conn: &Connection, id: i64, out: &Out) -> Result<()> {
    let task = queries::get_task(conn, id)?;
    let todos = queries::list_todos(conn, id)?;
    let sessions = queries::list_sessions(conn, id)?;

    let header = format!(
        "#{:<4} {}    {}",
        task.id,
        task.description,
        task.list.display_name()
    );
    println!();
    println!("  {}", out.bold(&header));
    let rule = "─".repeat(40);
    println!("  {}", out.dim(&rule));

    if !todos.is_empty() {
        println!();
        println!("  {}", out.bold("Todos"));
        for todo in &todos {
            let check = if todo.done { "[x]" } else { "[ ]" };
            let text = format!("{} {}  {}", check, todo.id, todo.description);
            let line = if todo.done { out.dim(&text) } else { text };
            println!("  {}", line);
        }
    }

    if !sessions.is_empty() {
        println!();
        println!("  {}", out.bold("Sessions"));
        for session in &sessions {
            let date = session.begin_at.format("%Y-%m-%d %H:%M").to_string();
            let duration = if let Some(end_at) = session.end_at {
                let mins = (end_at - session.begin_at).num_minutes();
                format!("{}m", mins)
            } else {
                format!("{}m (running)", session.duration_min)
            };
            println!("  {}  {}", date, out.cyan(&duration));
            if !session.notes.is_empty() {
                for line in session.notes.lines() {
                    println!("    {}", out.dim(line));
                }
            }
        }
    }

    println!();
    Ok(())
}

pub fn done(conn: &Connection, id: i64) -> Result<()> {
    let task = queries::get_task(conn, id)?;
    queries::delete_task(conn, id)?;
    println!("Done: #{} \"{}\" removed.", id, task.description);
    Ok(())
}

pub fn rm(conn: &Connection, id: i64, yes: bool) -> Result<()> {
    let task = queries::get_task(conn, id)?;
    if !yes && !confirm(&format!("Delete '{}'?", task.description)) {
        println!("Aborted.");
        return Ok(());
    }
    queries::delete_task(conn, id)?;
    println!("Deleted #{} \"{}\".", id, task.description);
    Ok(())
}

pub fn defer(conn: &Connection, id: i64) -> Result<()> {
    let task = queries::get_task(conn, id)?;
    queries::move_task(conn, id, &TaskList::Backlog)?;
    println!("Deferred #{} \"{}\" to Backlog.", id, task.description);
    Ok(())
}

pub fn edit(conn: &Connection, task_id: i64, args: &[String]) -> Result<()> {
    if args.is_empty() {
        bail!("Usage: logbuch edit <task-id> <description>\n       logbuch edit <task-id> <todo-id> <description>");
    }

    // If first arg is a valid integer, treat it as a todo-id
    if let Ok(todo_id) = args[0].parse::<i64>() {
        if args.len() < 2 {
            bail!("Usage: logbuch edit <task-id> <todo-id> <description>");
        }
        let description = args[1..].join(" ");
        queries::get_task(conn, task_id)?; // verify task exists
        queries::update_todo_description(conn, todo_id, &description)?;
        println!("Updated todo {} on #{}.", todo_id, task_id);
    } else {
        let description = args.join(" ");
        queries::update_task_description(conn, task_id, &description)?;
        println!("Renamed #{} to \"{}\".", task_id, description);
    }

    Ok(())
}

fn confirm(prompt: &str) -> bool {
    use std::io::Write;
    eprint!("{} [y/N] ", prompt);
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap_or(0);
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}
