use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::Out;

pub fn log_daily(conn: &Connection, out: &Out) -> Result<()> {
    let today = chrono::Local::now().date_naive();
    let from = today.and_hms_opt(0, 0, 0).unwrap();
    let to = today.and_hms_opt(23, 59, 59).unwrap();

    let day_label = today.format("%A %-d %b %Y").to_string();
    println!();
    println!("  {}", out.bold(&day_label));

    print_range(conn, out, from, to)
}

pub fn log_weekly(conn: &Connection, out: &Out) -> Result<()> {
    use chrono::Datelike;
    let today = chrono::Local::now().date_naive();
    let days_from_monday = today.weekday().num_days_from_monday() as i64;
    let monday = today - chrono::Duration::days(days_from_monday);
    let sunday = monday + chrono::Duration::days(6);

    let from = monday.and_hms_opt(0, 0, 0).unwrap();
    let to = sunday.and_hms_opt(23, 59, 59).unwrap();

    let label = format!(
        "{} – {}",
        monday.format("%-d %b"),
        sunday.format("%-d %b %Y")
    );
    println!();
    println!("  {}", out.bold(&label));

    print_range(conn, out, from, to)
}

fn print_range(
    conn: &Connection,
    out: &Out,
    from: chrono::NaiveDateTime,
    to: chrono::NaiveDateTime,
) -> Result<()> {
    let sessions = queries::sessions_in_range(conn, from, to)?;
    let completed_todos = queries::todos_completed_in_range(conn, from, to)?;

    if sessions.is_empty() && completed_todos.is_empty() {
        println!();
        println!("  No activity recorded.");
        println!();
        return Ok(());
    }

    // Sessions table
    if !sessions.is_empty() {
        println!();
        println!("  {}", out.bold("Sessions"));

        let rule = out.dim(&"─".repeat(45));
        println!("  {}", rule);

        let mut total_mins: i64 = 0;
        for (task, session) in &sessions {
            let start = session.begin_at.format("%H:%M").to_string();
            let end = session
                .end_at
                .map(|e| e.format("%H:%M").to_string())
                .unwrap_or_else(|| "?".to_string());
            let mins = session
                .end_at
                .map(|e| (e - session.begin_at).num_minutes())
                .unwrap_or(session.duration_min as i64);
            total_mins += mins;

            println!(
                "  {:<24} {}–{}   {}",
                task.description,
                out.cyan(&start),
                out.cyan(&end),
                out.dim(&format!("{}m", mins))
            );
        }

        println!("  {}", rule);
        println!(
            "  {:<24}              {}",
            "Total",
            out.bold(&format!("{}m", total_mins))
        );
    }

    // Completed todos
    if !completed_todos.is_empty() {
        println!();
        println!("  {}", out.bold("Completed todos"));
        for (task, todo) in &completed_todos {
            println!(
                "  [x] {:<36} {}",
                todo.description,
                out.dim(&task.description)
            );
        }
    }

    println!();
    Ok(())
}
