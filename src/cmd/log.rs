use std::collections::BTreeMap;

use anyhow::{bail, Result};
use chrono::{Datelike, NaiveDate, Weekday};
use rusqlite::Connection;

use crate::db::queries;
use crate::model::{Session, Task, Todo};
use crate::output::Out;

pub fn run(
    conn: &Connection,
    out: &Out,
    from_arg: Option<&str>,
    to_arg: Option<&str>,
    week: bool,
) -> Result<()> {
    let today = chrono::Local::now().date_naive();

    let (from, to) = if week {
        let days_from_monday = today.weekday().num_days_from_monday() as i64;
        let monday = today - chrono::Duration::days(days_from_monday);
        let sunday = monday + chrono::Duration::days(6);
        (monday, sunday)
    } else if let Some(from_str) = from_arg {
        let from = parse_date(from_str)?;
        let to = if let Some(to_str) = to_arg {
            parse_date(to_str)?
        } else {
            from
        };
        if to < from {
            bail!("End date {} is before start date {}.", to, from);
        }
        (from, to)
    } else {
        (today, today)
    };

    let single_day = from == to;

    let from_dt = from.and_hms_opt(0, 0, 0).unwrap();
    let to_dt = to.and_hms_opt(23, 59, 59).unwrap();

    let all_sessions = queries::sessions_in_range(conn, from_dt, to_dt)?;
    let all_todos = queries::todos_completed_in_range(conn, from_dt, to_dt)?;

    if all_sessions.is_empty() && all_todos.is_empty() {
        println!();
        println!("  No activity recorded.");
        println!();
        return Ok(());
    }

    // Group by day
    let mut day_sessions: BTreeMap<NaiveDate, Vec<(Task, Session)>> = BTreeMap::new();
    let mut day_todos: BTreeMap<NaiveDate, Vec<(Task, Todo)>> = BTreeMap::new();

    for (task, session) in all_sessions {
        day_sessions
            .entry(session.begin_at.date())
            .or_default()
            .push((task, session));
    }
    for (task, todo) in all_todos {
        if let Some(completed_at) = todo.completed_at {
            day_todos
                .entry(completed_at.date())
                .or_default()
                .push((task, todo));
        }
    }

    // Collect all days in range that have any activity
    let mut current = from;
    while current <= to {
        let has_sessions = day_sessions.contains_key(&current);
        let has_todos = day_todos.contains_key(&current);

        let is_weekend = current.weekday() == Weekday::Sat || current.weekday() == Weekday::Sun;

        // Skip weekend days with no activity (unless it's a single-day query)
        if !single_day && is_weekend && !has_sessions && !has_todos {
            current += chrono::Duration::days(1);
            continue;
        }

        // Skip weekdays with no activity too when multi-day
        if !single_day && !has_sessions && !has_todos {
            current += chrono::Duration::days(1);
            continue;
        }

        let sessions = day_sessions.remove(&current).unwrap_or_default();
        let todos = day_todos.remove(&current).unwrap_or_default();

        print_day(out, current, &sessions, &todos, single_day);

        current += chrono::Duration::days(1);
    }

    Ok(())
}

fn print_day(
    out: &Out,
    date: NaiveDate,
    sessions: &[(Task, Session)],
    todos: &[(Task, Todo)],
    single_day: bool,
) {
    // Heading: "Friday 6 Mar 2026" for single day, "Fri 6 Mar" for range
    let label = if single_day {
        date.format("%A %-d %b %Y").to_string()
    } else {
        date.format("%a %-d %b").to_string()
    };
    println!();
    println!("  {}", out.bold(&label));

    if !sessions.is_empty() {
        let rule = out.dim(&"─".repeat(45));
        println!("  {}", rule);

        let mut total_mins: i64 = 0;
        for (task, session) in sessions {
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
                "  {:<28} {}–{}   {}",
                task.description,
                out.cyan(&start),
                out.cyan(&end),
                out.dim(&format!("{}m", mins))
            );
        }

        println!("  {}", rule);
        println!(
            "  {:<28}          {}",
            "Total",
            out.bold(&format!("{}m", total_mins))
        );
    }

    if !todos.is_empty() {
        println!();
        println!("  {}", out.bold("Completed todos"));
        for (task, todo) in todos {
            println!(
                "  [x] {:<36} {}",
                todo.description,
                out.dim(&task.description)
            );
        }
    }

    println!();
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("Invalid date '{}'. Expected format: yyyy-mm-dd", s))
}
