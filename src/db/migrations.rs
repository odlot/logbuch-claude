use anyhow::Result;
use rusqlite::Connection;

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

// Migration 002: extend the task.list CHECK constraint to allow 'done'.
// SQLite does not support ALTER COLUMN, so we recreate the table.
const MIGRATION_002: &str = "
PRAGMA foreign_keys=OFF;
CREATE TABLE task_new (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    description TEXT    NOT NULL,
    list        TEXT    NOT NULL CHECK (list IN ('inbox','in_progress','backlog','done')),
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S','now','localtime')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S','now','localtime'))
);
INSERT INTO task_new SELECT id, description, list, position, created_at, updated_at FROM task;
DROP TABLE task;
ALTER TABLE task_new RENAME TO task;
INSERT INTO schema_version (version) VALUES (2);
PRAGMA foreign_keys=ON;
";

const MIGRATIONS: &[(i32, &str)] = &[(1, MIGRATION_001), (2, MIGRATION_002)];

pub fn run(conn: &Connection) -> Result<()> {
    let current_version = get_current_version(conn);

    for (version, sql) in MIGRATIONS {
        if *version > current_version {
            conn.execute_batch(sql)?;
        }
    }

    Ok(())
}

fn get_current_version(conn: &Connection) -> i32 {
    conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
        row.get(0)
    })
    .unwrap_or(0)
}
