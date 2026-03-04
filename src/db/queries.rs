use anyhow::Result;
use chrono::NaiveDateTime;
use rusqlite::{params, Connection};

use crate::model::{Session, Task, TaskList, Todo};

const DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S";

// --- Tasks ---

pub fn list_tasks(conn: &Connection, list: &TaskList) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, description, list, position, created_at, updated_at
         FROM task WHERE list = ?1 ORDER BY position ASC",
    )?;
    let rows = stmt.query_map(params![list.as_str()], |row| {
        Ok(Task {
            id: row.get(0)?,
            description: row.get(1)?,
            list: TaskList::from_str(&row.get::<_, String>(2)?).unwrap_or(TaskList::Inbox),
            position: row.get(3)?,
            created_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(4)?, DATETIME_FMT)
                .unwrap_or_default(),
            updated_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(5)?, DATETIME_FMT)
                .unwrap_or_default(),
        })
    })?;
    let mut tasks = Vec::new();
    for row in rows {
        tasks.push(row?);
    }
    Ok(tasks)
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Task> {
    let task = conn.query_row(
        "SELECT id, description, list, position, created_at, updated_at
         FROM task WHERE id = ?1",
        params![id],
        |row| {
            Ok(Task {
                id: row.get(0)?,
                description: row.get(1)?,
                list: TaskList::from_str(&row.get::<_, String>(2)?).unwrap_or(TaskList::Inbox),
                position: row.get(3)?,
                created_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(4)?, DATETIME_FMT)
                    .unwrap_or_default(),
                updated_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(5)?, DATETIME_FMT)
                    .unwrap_or_default(),
            })
        },
    )?;
    Ok(task)
}

pub fn insert_task(conn: &Connection, description: &str, list: &TaskList) -> Result<i64> {
    let max_pos: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(position), -1) FROM task WHERE list = ?1",
            params![list.as_str()],
            |row| row.get(0),
        )
        .unwrap_or(-1);

    conn.execute(
        "INSERT INTO task (description, list, position) VALUES (?1, ?2, ?3)",
        params![description, list.as_str(), max_pos + 1],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_task_description(conn: &Connection, id: i64, description: &str) -> Result<()> {
    conn.execute(
        "UPDATE task SET description = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime') WHERE id = ?2",
        params![description, id],
    )?;
    Ok(())
}

pub fn move_task(conn: &Connection, id: i64, target_list: &TaskList) -> Result<()> {
    let max_pos: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(position), -1) FROM task WHERE list = ?1",
            params![target_list.as_str()],
            |row| row.get(0),
        )
        .unwrap_or(-1);

    conn.execute(
        "UPDATE task SET list = ?1, position = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime') WHERE id = ?3",
        params![target_list.as_str(), max_pos + 1, id],
    )?;
    Ok(())
}

pub fn delete_task(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM task WHERE id = ?1", params![id])?;
    Ok(())
}

// --- Todos ---

pub fn list_todos(conn: &Connection, task_id: i64) -> Result<Vec<Todo>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, description, done, position, completed_at
         FROM todo WHERE task_id = ?1 ORDER BY position ASC",
    )?;
    let rows = stmt.query_map(params![task_id], |row| {
        Ok(Todo {
            id: row.get(0)?,
            task_id: row.get(1)?,
            description: row.get(2)?,
            done: row.get::<_, i32>(3)? != 0,
            position: row.get(4)?,
            completed_at: row
                .get::<_, Option<String>>(5)?
                .and_then(|s| NaiveDateTime::parse_from_str(&s, DATETIME_FMT).ok()),
        })
    })?;
    let mut todos = Vec::new();
    for row in rows {
        todos.push(row?);
    }
    Ok(todos)
}

pub fn insert_todo(conn: &Connection, task_id: i64, description: &str) -> Result<i64> {
    let max_pos: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(position), -1) FROM todo WHERE task_id = ?1",
            params![task_id],
            |row| row.get(0),
        )
        .unwrap_or(-1);

    conn.execute(
        "INSERT INTO todo (task_id, description, position) VALUES (?1, ?2, ?3)",
        params![task_id, description, max_pos + 1],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn toggle_todo(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE todo SET done = CASE WHEN done = 0 THEN 1 ELSE 0 END,
         completed_at = CASE WHEN done = 0 THEN strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime') ELSE NULL END
         WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn delete_todo(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM todo WHERE id = ?1", params![id])?;
    Ok(())
}

// --- Sessions ---

pub fn list_sessions(conn: &Connection, task_id: i64) -> Result<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, begin_at, end_at, duration_min, notes
         FROM session WHERE task_id = ?1 ORDER BY begin_at DESC",
    )?;
    let rows = stmt.query_map(params![task_id], |row| {
        Ok(Session {
            id: row.get(0)?,
            task_id: row.get(1)?,
            begin_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(2)?, DATETIME_FMT)
                .unwrap_or_default(),
            end_at: row
                .get::<_, Option<String>>(3)?
                .and_then(|s| NaiveDateTime::parse_from_str(&s, DATETIME_FMT).ok()),
            duration_min: row.get(4)?,
            notes: row.get(5)?,
        })
    })?;
    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(row?);
    }
    Ok(sessions)
}

pub fn start_session(conn: &Connection, task_id: i64, duration_min: i32) -> Result<i64> {
    let now = chrono::Local::now()
        .naive_local()
        .format(DATETIME_FMT)
        .to_string();
    conn.execute(
        "INSERT INTO session (task_id, begin_at, duration_min) VALUES (?1, ?2, ?3)",
        params![task_id, now, duration_min],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn end_session(conn: &Connection, id: i64) -> Result<()> {
    let now = chrono::Local::now()
        .naive_local()
        .format(DATETIME_FMT)
        .to_string();
    conn.execute(
        "UPDATE session SET end_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}

pub fn append_session_notes(conn: &Connection, id: i64, note: &str) -> Result<()> {
    conn.execute(
        "UPDATE session SET notes = CASE
            WHEN notes = '' THEN ?1
            ELSE notes || char(10) || ?1
         END WHERE id = ?2",
        params![note, id],
    )?;
    Ok(())
}

pub fn get_active_session(conn: &Connection) -> Result<Option<Session>> {
    let result = conn.query_row(
        "SELECT id, task_id, begin_at, end_at, duration_min, notes
         FROM session WHERE end_at IS NULL LIMIT 1",
        [],
        |row| {
            Ok(Session {
                id: row.get(0)?,
                task_id: row.get(1)?,
                begin_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(2)?, DATETIME_FMT)
                    .unwrap_or_default(),
                end_at: None,
                duration_min: row.get(4)?,
                notes: row.get(5)?,
            })
        },
    );
    match result {
        Ok(session) => Ok(Some(session)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn close_orphaned_sessions(conn: &Connection) -> Result<u32> {
    let count = conn.execute(
        "UPDATE session SET end_at = strftime('%Y-%m-%dT%H:%M:%S',
            datetime(begin_at, '+' || duration_min || ' minutes'))
         WHERE end_at IS NULL",
        [],
    )?;
    Ok(count as u32)
}

// --- Summary queries ---

pub fn sessions_in_range(
    conn: &Connection,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Result<Vec<(Task, Session)>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.description, t.list, t.position, t.created_at, t.updated_at,
                s.id, s.task_id, s.begin_at, s.end_at, s.duration_min, s.notes
         FROM session s
         JOIN task t ON s.task_id = t.id
         WHERE s.begin_at >= ?1 AND s.begin_at <= ?2 AND s.end_at IS NOT NULL
         ORDER BY s.begin_at ASC",
    )?;
    let from_str = from.format(DATETIME_FMT).to_string();
    let to_str = to.format(DATETIME_FMT).to_string();
    let rows = stmt.query_map(params![from_str, to_str], |row| {
        let task = Task {
            id: row.get(0)?,
            description: row.get(1)?,
            list: TaskList::from_str(&row.get::<_, String>(2)?).unwrap_or(TaskList::Inbox),
            position: row.get(3)?,
            created_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(4)?, DATETIME_FMT)
                .unwrap_or_default(),
            updated_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(5)?, DATETIME_FMT)
                .unwrap_or_default(),
        };
        let session = Session {
            id: row.get(6)?,
            task_id: row.get(7)?,
            begin_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(8)?, DATETIME_FMT)
                .unwrap_or_default(),
            end_at: row
                .get::<_, Option<String>>(9)?
                .and_then(|s| NaiveDateTime::parse_from_str(&s, DATETIME_FMT).ok()),
            duration_min: row.get(10)?,
            notes: row.get(11)?,
        };
        Ok((task, session))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

pub fn todos_completed_in_range(
    conn: &Connection,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Result<Vec<(Task, Todo)>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.description, t.list, t.position, t.created_at, t.updated_at,
                td.id, td.task_id, td.description, td.done, td.position, td.completed_at
         FROM todo td
         JOIN task t ON td.task_id = t.id
         WHERE td.completed_at >= ?1 AND td.completed_at <= ?2
         ORDER BY td.completed_at ASC",
    )?;
    let from_str = from.format(DATETIME_FMT).to_string();
    let to_str = to.format(DATETIME_FMT).to_string();
    let rows = stmt.query_map(params![from_str, to_str], |row| {
        let task = Task {
            id: row.get(0)?,
            description: row.get(1)?,
            list: TaskList::from_str(&row.get::<_, String>(2)?).unwrap_or(TaskList::Inbox),
            position: row.get(3)?,
            created_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(4)?, DATETIME_FMT)
                .unwrap_or_default(),
            updated_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(5)?, DATETIME_FMT)
                .unwrap_or_default(),
        };
        let todo = Todo {
            id: row.get(6)?,
            task_id: row.get(7)?,
            description: row.get(8)?,
            done: row.get::<_, i32>(9)? != 0,
            position: row.get(10)?,
            completed_at: row
                .get::<_, Option<String>>(11)?
                .and_then(|s| NaiveDateTime::parse_from_str(&s, DATETIME_FMT).ok()),
        };
        Ok((task, todo))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}
