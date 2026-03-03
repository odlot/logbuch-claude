use rusqlite::Connection;

// We can't use `use logbuch::...` because the crate is a binary.
// Instead, we test the DB layer directly using rusqlite + the same SQL.

const MIGRATION_001: &str = "
CREATE TABLE IF NOT EXISTS task (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    description TEXT    NOT NULL,
    list        TEXT    NOT NULL CHECK (list IN ('inbox', 'in_progress', 'backlog')),
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS session (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id      INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    begin_at     TEXT    NOT NULL,
    end_at       TEXT,
    duration_min INTEGER NOT NULL DEFAULT 25,
    notes        TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS todo (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id      INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    description  TEXT    NOT NULL,
    done         INTEGER NOT NULL DEFAULT 0,
    position     INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
);

INSERT INTO schema_version (version) VALUES (1);
";

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    conn.execute_batch(MIGRATION_001).unwrap();
    conn
}

#[test]
fn test_create_task() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Test task', 'inbox', 0)",
        [],
    )
    .unwrap();

    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM task", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let desc: String = conn
        .query_row("SELECT description FROM task WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(desc, "Test task");
}

#[test]
fn test_move_task() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Task 1', 'inbox', 0)",
        [],
    )
    .unwrap();

    conn.execute(
        "UPDATE task SET list = 'in_progress', position = 0 WHERE id = 1",
        [],
    )
    .unwrap();

    let list: String = conn
        .query_row("SELECT list FROM task WHERE id = 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(list, "in_progress");
}

#[test]
fn test_delete_task_cascades() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Task', 'inbox', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO todo (task_id, description, position) VALUES (1, 'Todo 1', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO session (task_id, begin_at, duration_min) VALUES (1, '2026-03-01T10:00:00', 25)",
        [],
    )
    .unwrap();

    // Verify children exist
    let todo_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM todo", [], |row| row.get(0))
        .unwrap();
    assert_eq!(todo_count, 1);

    let session_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM session", [], |row| row.get(0))
        .unwrap();
    assert_eq!(session_count, 1);

    // Delete parent task
    conn.execute("DELETE FROM task WHERE id = 1", []).unwrap();

    // Children should be cascaded
    let todo_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM todo", [], |row| row.get(0))
        .unwrap();
    assert_eq!(todo_count, 0);

    let session_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM session", [], |row| row.get(0))
        .unwrap();
    assert_eq!(session_count, 0);
}

#[test]
fn test_todo_toggle() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Task', 'inbox', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO todo (task_id, description, position) VALUES (1, 'Do something', 0)",
        [],
    )
    .unwrap();

    // Initially not done
    let done: i32 = conn
        .query_row("SELECT done FROM todo WHERE id = 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(done, 0);

    // Toggle to done
    conn.execute(
        "UPDATE todo SET done = CASE WHEN done = 0 THEN 1 ELSE 0 END,
         completed_at = CASE WHEN done = 0 THEN strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime') ELSE NULL END
         WHERE id = 1",
        [],
    )
    .unwrap();

    let done: i32 = conn
        .query_row("SELECT done FROM todo WHERE id = 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(done, 1);

    let completed_at: Option<String> = conn
        .query_row("SELECT completed_at FROM todo WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(completed_at.is_some());

    // Toggle back to not done
    conn.execute(
        "UPDATE todo SET done = CASE WHEN done = 0 THEN 1 ELSE 0 END,
         completed_at = CASE WHEN done = 0 THEN strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime') ELSE NULL END
         WHERE id = 1",
        [],
    )
    .unwrap();

    let done: i32 = conn
        .query_row("SELECT done FROM todo WHERE id = 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(done, 0);

    let completed_at: Option<String> = conn
        .query_row("SELECT completed_at FROM todo WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(completed_at.is_none());
}

#[test]
fn test_session_lifecycle() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Task', 'inbox', 0)",
        [],
    )
    .unwrap();

    // Start a session
    conn.execute(
        "INSERT INTO session (task_id, begin_at, duration_min) VALUES (1, '2026-03-01T10:00:00', 25)",
        [],
    )
    .unwrap();

    // Session should be active (no end_at)
    let end_at: Option<String> = conn
        .query_row("SELECT end_at FROM session WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(end_at.is_none());

    // Append notes
    conn.execute(
        "UPDATE session SET notes = CASE
            WHEN notes = '' THEN 'First note'
            ELSE notes || char(10) || 'First note'
         END WHERE id = 1",
        [],
    )
    .unwrap();

    conn.execute(
        "UPDATE session SET notes = CASE
            WHEN notes = '' THEN 'Second note'
            ELSE notes || char(10) || 'Second note'
         END WHERE id = 1",
        [],
    )
    .unwrap();

    let notes: String = conn
        .query_row("SELECT notes FROM session WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(notes, "First note\nSecond note");

    // End session
    conn.execute(
        "UPDATE session SET end_at = '2026-03-01T10:25:00' WHERE id = 1",
        [],
    )
    .unwrap();

    let end_at: Option<String> = conn
        .query_row("SELECT end_at FROM session WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(end_at, Some("2026-03-01T10:25:00".to_string()));
}

#[test]
fn test_close_orphaned_sessions() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Task', 'inbox', 0)",
        [],
    )
    .unwrap();

    // Create an orphaned session (no end_at)
    conn.execute(
        "INSERT INTO session (task_id, begin_at, duration_min) VALUES (1, '2026-03-01T10:00:00', 25)",
        [],
    )
    .unwrap();

    // Close orphaned sessions
    let count = conn
        .execute(
            "UPDATE session SET end_at = strftime('%Y-%m-%dT%H:%M:%S',
                datetime(begin_at, '+' || duration_min || ' minutes'))
             WHERE end_at IS NULL",
            [],
        )
        .unwrap();
    assert_eq!(count, 1);

    let end_at: String = conn
        .query_row("SELECT end_at FROM session WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(end_at, "2026-03-01T10:25:00");
}

#[test]
fn test_list_constraint() {
    let conn = setup_db();
    let result = conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Bad', 'invalid_list', 0)",
        [],
    );
    assert!(result.is_err(), "Should reject invalid list values");
}

#[test]
fn test_task_position_ordering() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Third', 'inbox', 2)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('First', 'inbox', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Second', 'inbox', 1)",
        [],
    )
    .unwrap();

    let mut stmt = conn
        .prepare("SELECT description FROM task WHERE list = 'inbox' ORDER BY position ASC")
        .unwrap();
    let descriptions: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(descriptions, vec!["First", "Second", "Third"]);
}

#[test]
fn test_summary_queries() {
    let conn = setup_db();

    // Create task with a completed session
    conn.execute(
        "INSERT INTO task (description, list, position) VALUES ('Task A', 'inbox', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO session (task_id, begin_at, end_at, duration_min, notes) VALUES (1, '2026-03-01T10:00:00', '2026-03-01T10:25:00', 25, 'Did stuff')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO todo (task_id, description, done, position, completed_at) VALUES (1, 'Done todo', 1, 0, '2026-03-01T10:15:00')",
        [],
    )
    .unwrap();

    // Query sessions in range
    let mut stmt = conn
        .prepare(
            "SELECT t.description, s.notes FROM session s
             JOIN task t ON s.task_id = t.id
             WHERE s.begin_at >= '2026-03-01T00:00:00' AND s.begin_at <= '2026-03-01T23:59:59' AND s.end_at IS NOT NULL",
        )
        .unwrap();
    let results: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "Task A");
    assert_eq!(results[0].1, "Did stuff");

    // Query completed todos in range
    let mut stmt = conn
        .prepare(
            "SELECT t.description, td.description FROM todo td
             JOIN task t ON td.task_id = t.id
             WHERE td.completed_at >= '2026-03-01T00:00:00' AND td.completed_at <= '2026-03-01T23:59:59'",
        )
        .unwrap();
    let results: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "Task A");
    assert_eq!(results[0].1, "Done todo");
}
