use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{Datelike, NaiveDate, NaiveTime};
use rusqlite::Connection;

use crate::db::queries;

pub fn generate_daily(conn: &Connection, date: NaiveDate, export_dir: &Path) -> Result<PathBuf> {
    let from = date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let to = date.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap());

    let sessions = queries::sessions_in_range(conn, from, to)?;
    let completed_todos = queries::todos_completed_in_range(conn, from, to)?;

    let filename = format!("logbuch-daily-{}.md", date.format("%Y-%m-%d"));
    let path = export_dir.join(&filename);

    let content = format_report(
        &format!("Logbuch Daily Summary — {}", date.format("%Y-%m-%d")),
        &sessions,
        &completed_todos,
    );

    std::fs::create_dir_all(export_dir)?;
    std::fs::write(&path, &content)?;

    Ok(path)
}

pub fn generate_weekly(conn: &Connection, date: NaiveDate, export_dir: &Path) -> Result<PathBuf> {
    // Find Monday of the current week
    let weekday = date.weekday().num_days_from_monday();
    let monday = date - chrono::Duration::days(weekday as i64);
    let sunday = monday + chrono::Duration::days(6);

    let from = monday.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let to = sunday.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap());

    let sessions = queries::sessions_in_range(conn, from, to)?;
    let completed_todos = queries::todos_completed_in_range(conn, from, to)?;

    let iso_week = date.iso_week().week();
    let filename = format!("logbuch-weekly-{}-W{:02}.md", date.format("%Y"), iso_week);
    let path = export_dir.join(&filename);

    let content = format_report(
        &format!(
            "Logbuch Weekly Summary — {} to {} (W{:02})",
            monday.format("%Y-%m-%d"),
            sunday.format("%Y-%m-%d"),
            iso_week
        ),
        &sessions,
        &completed_todos,
    );

    std::fs::create_dir_all(export_dir)?;
    std::fs::write(&path, &content)?;

    Ok(path)
}

fn format_report(
    title: &str,
    sessions: &[(crate::model::Task, crate::model::Session)],
    completed_todos: &[(crate::model::Task, crate::model::Todo)],
) -> String {
    let mut out = String::new();
    writeln!(out, "# {}\n", title).unwrap();

    // Sessions section
    let total_minutes: i64 = sessions
        .iter()
        .map(|(_, s)| {
            if let Some(end) = s.end_at {
                (end - s.begin_at).num_minutes()
            } else {
                s.duration_min as i64
            }
        })
        .sum();
    let hours = total_minutes / 60;
    let mins = total_minutes % 60;

    writeln!(
        out,
        "## Sessions ({} total, {}h {}m)\n",
        sessions.len(),
        hours,
        mins
    )
    .unwrap();

    // Group sessions by task
    let mut by_task: BTreeMap<i64, (String, Vec<&crate::model::Session>)> = BTreeMap::new();
    for (task, session) in sessions {
        by_task
            .entry(task.id)
            .or_insert_with(|| (task.description.clone(), Vec::new()))
            .1
            .push(session);
    }

    for (description, task_sessions) in by_task.values() {
        writeln!(out, "### {}\n", description).unwrap();
        for session in task_sessions {
            let begin = session.begin_at.format("%H:%M");
            let end = session
                .end_at
                .map(|e| e.format("%H:%M").to_string())
                .unwrap_or_else(|| "?".to_string());
            let duration = if let Some(end_at) = session.end_at {
                (end_at - session.begin_at).num_minutes()
            } else {
                session.duration_min as i64
            };
            writeln!(out, "- {} - {} ({}m)", begin, end, duration).unwrap();

            if !session.notes.is_empty() {
                for line in session.notes.lines() {
                    writeln!(out, "  > {}", line).unwrap();
                }
            }
        }
        writeln!(out).unwrap();
    }

    // Completed todos section
    if !completed_todos.is_empty() {
        writeln!(out, "## Completed Todos ({})\n", completed_todos.len()).unwrap();

        let mut todos_by_task: BTreeMap<i64, (String, Vec<&crate::model::Todo>)> = BTreeMap::new();
        for (task, todo) in completed_todos {
            todos_by_task
                .entry(task.id)
                .or_insert_with(|| (task.description.clone(), Vec::new()))
                .1
                .push(todo);
        }

        for (description, todos) in todos_by_task.values() {
            writeln!(out, "### {}\n", description).unwrap();
            for todo in todos {
                writeln!(out, "- [x] {}", todo.description).unwrap();
            }
            writeln!(out).unwrap();
        }
    }

    out
}
