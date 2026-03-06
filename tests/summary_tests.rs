use chrono::NaiveDate;
use rusqlite::Connection;

use logbuch::db::{migrations, queries};
use logbuch::model::TaskList;
use logbuch::summary;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    migrations::run(&conn).unwrap();
    conn
}

fn insert_completed_session(conn: &Connection, task_id: i64, begin: &str, end: &str, notes: &str) {
    conn.execute(
        "INSERT INTO session (task_id, begin_at, end_at, duration_min, notes)
         VALUES (?1, ?2, ?3, 25, ?4)",
        rusqlite::params![task_id, begin, end, notes],
    )
    .unwrap();
}

fn insert_completed_todo(conn: &Connection, task_id: i64, desc: &str, completed_at: &str) {
    conn.execute(
        "INSERT INTO todo (task_id, description, done, position, completed_at)
         VALUES (?1, ?2, 1, 0, ?3)",
        rusqlite::params![task_id, desc, completed_at],
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// generate_daily
// ---------------------------------------------------------------------------

#[test]
fn generate_daily_creates_markdown_file_in_export_dir() {
    // Arrange
    let conn = setup();
    let dir = tempfile::TempDir::new().unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();

    // Act
    let path = summary::generate_daily(&conn, date, dir.path()).unwrap();

    // Assert
    assert!(path.exists());
    assert_eq!(path.file_name().unwrap(), "logbuch-daily-2026-03-01.md");
}

#[test]
fn generate_daily_content_includes_task_description_and_session_times() {
    // Arrange
    let conn = setup();
    let dir = tempfile::TempDir::new().unwrap();
    let task_id = queries::insert_task(&conn, "Write tests", &TaskList::Inbox).unwrap();
    insert_completed_session(
        &conn,
        task_id,
        "2026-03-01T10:00:00",
        "2026-03-01T10:25:00",
        "",
    );
    let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();

    // Act
    let path = summary::generate_daily(&conn, date, dir.path()).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    // Assert
    assert!(content.contains("Write tests"));
    assert!(content.contains("10:00"));
    assert!(content.contains("10:25"));
}

#[test]
fn generate_daily_content_includes_session_notes() {
    // Arrange
    let conn = setup();
    let dir = tempfile::TempDir::new().unwrap();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    insert_completed_session(
        &conn,
        task_id,
        "2026-03-01T10:00:00",
        "2026-03-01T10:25:00",
        "Important note",
    );
    let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();

    // Act
    let path = summary::generate_daily(&conn, date, dir.path()).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    // Assert
    assert!(content.contains("Important note"));
}

#[test]
fn generate_daily_content_includes_completed_todos() {
    // Arrange
    let conn = setup();
    let dir = tempfile::TempDir::new().unwrap();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    insert_completed_todo(&conn, task_id, "Reviewed PR", "2026-03-01T14:00:00");
    let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();

    // Act
    let path = summary::generate_daily(&conn, date, dir.path()).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    // Assert
    assert!(content.contains("Reviewed PR"));
    assert!(content.contains("[x]"));
}

#[test]
fn generate_daily_produces_valid_report_with_no_sessions_or_todos() {
    // Arrange
    let conn = setup();
    let dir = tempfile::TempDir::new().unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 3, 5).unwrap();

    // Act
    let path = summary::generate_daily(&conn, date, dir.path()).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    // Assert: header still present, 0 sessions
    assert!(content.contains("2026-03-05"));
    assert!(content.contains("0 total"));
}

// ---------------------------------------------------------------------------
// generate_weekly
// ---------------------------------------------------------------------------

#[test]
fn generate_weekly_creates_markdown_file_in_export_dir() {
    // Arrange
    let conn = setup();
    let dir = tempfile::TempDir::new().unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 3, 4).unwrap(); // Wednesday of week 10

    // Act
    let path = summary::generate_weekly(&conn, date, dir.path()).unwrap();

    // Assert
    assert!(path.exists());
    let filename = path.file_name().unwrap().to_str().unwrap();
    assert!(filename.starts_with("logbuch-weekly-2026-W"));
}

#[test]
fn generate_weekly_content_includes_monday_to_sunday_range() {
    // Arrange
    let conn = setup();
    let dir = tempfile::TempDir::new().unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 3, 4).unwrap(); // Wednesday → week starts Mon 2026-03-02

    // Act
    let path = summary::generate_weekly(&conn, date, dir.path()).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    // Assert: report spans Mon→Sun of that week
    assert!(content.contains("2026-03-02"));
    assert!(content.contains("2026-03-08"));
}

#[test]
fn generate_weekly_content_includes_sessions_from_the_week() {
    // Arrange
    let conn = setup();
    let dir = tempfile::TempDir::new().unwrap();
    let task_id = queries::insert_task(&conn, "Weekly task", &TaskList::InProgress).unwrap();
    insert_completed_session(
        &conn,
        task_id,
        "2026-03-03T09:00:00",
        "2026-03-03T09:25:00",
        "",
    );
    let date = NaiveDate::from_ymd_opt(2026, 3, 4).unwrap();

    // Act
    let path = summary::generate_weekly(&conn, date, dir.path()).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    // Assert
    assert!(content.contains("Weekly task"));
}
