use rusqlite::Connection;

use logbuch::db::{self, migrations, queries};
use logbuch::model::TaskList;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    migrations::run(&conn).unwrap();
    conn
}

// ---------------------------------------------------------------------------
// Migrations
// ---------------------------------------------------------------------------

#[test]
fn migrations_creates_all_required_tables() {
    // Arrange
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

    // Act
    migrations::run(&conn).unwrap();

    // Assert: all four tables must exist
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'
             AND name IN ('task','session','todo','schema_version')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 4);
}

#[test]
fn migrations_run_is_idempotent() {
    // Arrange
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    migrations::run(&conn).unwrap();

    // Act: running a second time must not error
    let result = migrations::run(&conn);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn db_init_creates_database_file_and_schema() {
    // Arrange
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");

    // Act
    let conn = db::init(&db_path).unwrap();

    // Assert: file exists and schema is in place
    assert!(db_path.exists());
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'
             AND name IN ('task','session','todo','schema_version')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 4);
}

// ---------------------------------------------------------------------------
// Task CRUD
// ---------------------------------------------------------------------------

#[test]
fn insert_task_returns_id_and_task_appears_in_list() {
    // Arrange
    let conn = setup();

    // Act
    let id = queries::insert_task(&conn, "Buy milk", &TaskList::Inbox).unwrap();

    // Assert
    let tasks = queries::list_tasks(&conn, &TaskList::Inbox).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, id);
    assert_eq!(tasks[0].description, "Buy milk");
    assert_eq!(tasks[0].list, TaskList::Inbox);
}

#[test]
fn insert_multiple_tasks_assigns_sequential_positions() {
    // Arrange
    let conn = setup();

    // Act
    queries::insert_task(&conn, "First", &TaskList::Inbox).unwrap();
    queries::insert_task(&conn, "Second", &TaskList::Inbox).unwrap();
    queries::insert_task(&conn, "Third", &TaskList::Inbox).unwrap();

    // Assert
    let tasks = queries::list_tasks(&conn, &TaskList::Inbox).unwrap();
    assert_eq!(tasks[0].description, "First");
    assert_eq!(tasks[1].description, "Second");
    assert_eq!(tasks[2].description, "Third");
    assert!(tasks[0].position < tasks[1].position);
    assert!(tasks[1].position < tasks[2].position);
}

#[test]
fn list_tasks_returns_empty_for_list_with_no_tasks() {
    // Arrange
    let conn = setup();
    queries::insert_task(&conn, "Inbox task", &TaskList::Inbox).unwrap();

    // Act
    let tasks = queries::list_tasks(&conn, &TaskList::InProgress).unwrap();

    // Assert
    assert!(tasks.is_empty());
}

#[test]
fn get_task_returns_correct_task() {
    // Arrange
    let conn = setup();
    let id = queries::insert_task(&conn, "My task", &TaskList::Backlog).unwrap();

    // Act
    let task = queries::get_task(&conn, id).unwrap();

    // Assert
    assert_eq!(task.id, id);
    assert_eq!(task.description, "My task");
    assert_eq!(task.list, TaskList::Backlog);
}

#[test]
fn update_task_description_changes_description() {
    // Arrange
    let conn = setup();
    let id = queries::insert_task(&conn, "Old name", &TaskList::Inbox).unwrap();

    // Act
    queries::update_task_description(&conn, id, "New name").unwrap();

    // Assert
    let task = queries::get_task(&conn, id).unwrap();
    assert_eq!(task.description, "New name");
}

#[test]
fn move_task_places_task_at_end_of_target_list() {
    // Arrange
    let conn = setup();
    let id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    queries::insert_task(&conn, "Existing", &TaskList::InProgress).unwrap();

    // Act
    queries::move_task(&conn, id, &TaskList::InProgress).unwrap();

    // Assert
    let inbox = queries::list_tasks(&conn, &TaskList::Inbox).unwrap();
    let in_progress = queries::list_tasks(&conn, &TaskList::InProgress).unwrap();
    assert!(inbox.is_empty());
    assert_eq!(in_progress.len(), 2);
    assert_eq!(in_progress.last().unwrap().description, "Task");
}

#[test]
fn delete_task_removes_it_and_cascades_to_todos_and_sessions() {
    // Arrange
    let conn = setup();
    let id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    queries::insert_todo(&conn, id, "A todo").unwrap();
    queries::start_session(&conn, id, 25).unwrap();

    // Act
    queries::delete_task(&conn, id).unwrap();

    // Assert
    let tasks = queries::list_tasks(&conn, &TaskList::Inbox).unwrap();
    assert!(tasks.is_empty());

    let todo_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM todo", [], |row| row.get(0))
        .unwrap();
    assert_eq!(todo_count, 0);

    let session_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM session", [], |row| row.get(0))
        .unwrap();
    assert_eq!(session_count, 0);
}

// ---------------------------------------------------------------------------
// Todo CRUD
// ---------------------------------------------------------------------------

#[test]
fn insert_todo_appends_to_task_and_appears_in_list() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();

    // Act
    let todo_id = queries::insert_todo(&conn, task_id, "Write tests").unwrap();

    // Assert
    let todos = queries::list_todos(&conn, task_id).unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].id, todo_id);
    assert_eq!(todos[0].description, "Write tests");
    assert!(!todos[0].done);
}

#[test]
fn list_todos_returns_todos_in_position_order() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    queries::insert_todo(&conn, task_id, "Alpha").unwrap();
    queries::insert_todo(&conn, task_id, "Beta").unwrap();
    queries::insert_todo(&conn, task_id, "Gamma").unwrap();

    // Act
    let todos = queries::list_todos(&conn, task_id).unwrap();

    // Assert
    assert_eq!(todos[0].description, "Alpha");
    assert_eq!(todos[1].description, "Beta");
    assert_eq!(todos[2].description, "Gamma");
}

#[test]
fn update_todo_description_changes_description() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let todo_id = queries::insert_todo(&conn, task_id, "Old").unwrap();

    // Act
    queries::update_todo_description(&conn, todo_id, "New").unwrap();

    // Assert
    let todos = queries::list_todos(&conn, task_id).unwrap();
    assert_eq!(todos[0].description, "New");
}

#[test]
fn toggle_todo_marks_not_done_as_done_and_sets_completed_at() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let todo_id = queries::insert_todo(&conn, task_id, "Do something").unwrap();

    // Act
    queries::toggle_todo(&conn, todo_id).unwrap();

    // Assert
    let todos = queries::list_todos(&conn, task_id).unwrap();
    assert!(todos[0].done);
    assert!(todos[0].completed_at.is_some());
}

#[test]
fn toggle_todo_marks_done_as_not_done_and_clears_completed_at() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let todo_id = queries::insert_todo(&conn, task_id, "Do something").unwrap();
    queries::toggle_todo(&conn, todo_id).unwrap(); // → done

    // Act
    queries::toggle_todo(&conn, todo_id).unwrap(); // → not done

    // Assert
    let todos = queries::list_todos(&conn, task_id).unwrap();
    assert!(!todos[0].done);
    assert!(todos[0].completed_at.is_none());
}

#[test]
fn delete_todo_removes_todo() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let todo_id = queries::insert_todo(&conn, task_id, "To delete").unwrap();

    // Act
    queries::delete_todo(&conn, todo_id).unwrap();

    // Assert
    let todos = queries::list_todos(&conn, task_id).unwrap();
    assert!(todos.is_empty());
}

#[test]
fn move_todo_up_swaps_todo_with_the_one_above_it() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    queries::insert_todo(&conn, task_id, "First").unwrap();
    let second_id = queries::insert_todo(&conn, task_id, "Second").unwrap();

    // Act
    queries::move_todo_up(&conn, second_id, task_id).unwrap();

    // Assert
    let todos = queries::list_todos(&conn, task_id).unwrap();
    assert_eq!(todos[0].description, "Second");
    assert_eq!(todos[1].description, "First");
}

#[test]
fn move_todo_up_is_noop_when_todo_is_already_first() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let first_id = queries::insert_todo(&conn, task_id, "First").unwrap();
    queries::insert_todo(&conn, task_id, "Second").unwrap();

    // Act
    queries::move_todo_up(&conn, first_id, task_id).unwrap();

    // Assert: order unchanged
    let todos = queries::list_todos(&conn, task_id).unwrap();
    assert_eq!(todos[0].description, "First");
    assert_eq!(todos[1].description, "Second");
}

#[test]
fn move_todo_down_swaps_todo_with_the_one_below_it() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let first_id = queries::insert_todo(&conn, task_id, "First").unwrap();
    queries::insert_todo(&conn, task_id, "Second").unwrap();

    // Act
    queries::move_todo_down(&conn, first_id, task_id).unwrap();

    // Assert
    let todos = queries::list_todos(&conn, task_id).unwrap();
    assert_eq!(todos[0].description, "Second");
    assert_eq!(todos[1].description, "First");
}

#[test]
fn move_todo_down_is_noop_when_todo_is_already_last() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    queries::insert_todo(&conn, task_id, "First").unwrap();
    let last_id = queries::insert_todo(&conn, task_id, "Last").unwrap();

    // Act
    queries::move_todo_down(&conn, last_id, task_id).unwrap();

    // Assert: order unchanged
    let todos = queries::list_todos(&conn, task_id).unwrap();
    assert_eq!(todos[0].description, "First");
    assert_eq!(todos[1].description, "Last");
}

// ---------------------------------------------------------------------------
// Session CRUD
// ---------------------------------------------------------------------------

#[test]
fn start_session_creates_an_open_session_for_the_task() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();

    // Act
    let session_id = queries::start_session(&conn, task_id, 25).unwrap();

    // Assert
    let sessions = queries::list_sessions(&conn, task_id).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, session_id);
    assert_eq!(sessions[0].task_id, task_id);
    assert_eq!(sessions[0].duration_min, 25);
    assert!(sessions[0].end_at.is_none());
}

#[test]
fn end_session_sets_end_at_on_the_session() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let session_id = queries::start_session(&conn, task_id, 25).unwrap();

    // Act
    queries::end_session(&conn, session_id).unwrap();

    // Assert
    let sessions = queries::list_sessions(&conn, task_id).unwrap();
    assert!(sessions[0].end_at.is_some());
}

#[test]
fn append_session_notes_concatenates_lines_with_newline() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let session_id = queries::start_session(&conn, task_id, 25).unwrap();

    // Act
    queries::append_session_notes(&conn, session_id, "Line one").unwrap();
    queries::append_session_notes(&conn, session_id, "Line two").unwrap();

    // Assert
    let sessions = queries::list_sessions(&conn, task_id).unwrap();
    assert_eq!(sessions[0].notes, "Line one\nLine two");
}

#[test]
fn get_active_session_returns_the_open_session() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let session_id = queries::start_session(&conn, task_id, 25).unwrap();

    // Act
    let active = queries::get_active_session(&conn).unwrap();

    // Assert
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, session_id);
}

#[test]
fn get_active_session_returns_none_when_no_open_session_exists() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let session_id = queries::start_session(&conn, task_id, 25).unwrap();
    queries::end_session(&conn, session_id).unwrap();

    // Act
    let active = queries::get_active_session(&conn).unwrap();

    // Assert
    assert!(active.is_none());
}

#[test]
fn list_sessions_returns_sessions_newest_first() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    conn.execute(
        "INSERT INTO session (task_id, begin_at, end_at, duration_min, notes)
         VALUES (?1, '2026-01-01T09:00:00', '2026-01-01T09:25:00', 25, '')",
        [task_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO session (task_id, begin_at, end_at, duration_min, notes)
         VALUES (?1, '2026-01-02T09:00:00', '2026-01-02T09:25:00', 25, '')",
        [task_id],
    )
    .unwrap();

    // Act
    let sessions = queries::list_sessions(&conn, task_id).unwrap();

    // Assert: newest begin_at appears first
    assert_eq!(sessions.len(), 2);
    assert!(sessions[0].begin_at > sessions[1].begin_at);
}

#[test]
fn delete_session_removes_the_session() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    let session_id = queries::start_session(&conn, task_id, 25).unwrap();

    // Act
    queries::delete_session(&conn, session_id).unwrap();

    // Assert
    let sessions = queries::list_sessions(&conn, task_id).unwrap();
    assert!(sessions.is_empty());
}

#[test]
fn close_orphaned_sessions_closes_every_open_session() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    queries::start_session(&conn, task_id, 25).unwrap();
    queries::start_session(&conn, task_id, 25).unwrap();

    // Act
    let closed = queries::close_orphaned_sessions(&conn).unwrap();

    // Assert
    assert_eq!(closed, 2);
    let active = queries::get_active_session(&conn).unwrap();
    assert!(active.is_none());
}

#[test]
fn last_worked_task_returns_task_id_of_most_recent_session() {
    // Arrange
    let conn = setup();
    let task_a = queries::insert_task(&conn, "Task A", &TaskList::Inbox).unwrap();
    let task_b = queries::insert_task(&conn, "Task B", &TaskList::Inbox).unwrap();
    conn.execute(
        "INSERT INTO session (task_id, begin_at, end_at, duration_min, notes)
         VALUES (?1, '2026-01-01T09:00:00', '2026-01-01T09:45:00', 45, '')",
        [task_a],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO session (task_id, begin_at, end_at, duration_min, notes)
         VALUES (?1, '2026-01-02T09:00:00', '2026-01-02T09:45:00', 45, '')",
        [task_b],
    )
    .unwrap();

    // Act
    let result = queries::last_worked_task(&conn).unwrap();

    // Assert: Task B has the more recent session
    assert_eq!(result, Some(task_b));
}

#[test]
fn last_worked_task_returns_none_when_no_sessions_exist() {
    // Arrange
    let conn = setup();
    queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();

    // Act
    let result = queries::last_worked_task(&conn).unwrap();

    // Assert
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// Range queries
// ---------------------------------------------------------------------------

#[test]
fn sessions_in_range_returns_completed_sessions_within_window() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task A", &TaskList::Inbox).unwrap();
    conn.execute(
        "INSERT INTO session (task_id, begin_at, end_at, duration_min, notes)
         VALUES (?1, '2026-03-01T10:00:00', '2026-03-01T10:25:00', 25, 'did work')",
        [task_id],
    )
    .unwrap();
    // Outside the window
    conn.execute(
        "INSERT INTO session (task_id, begin_at, end_at, duration_min, notes)
         VALUES (?1, '2026-03-02T10:00:00', '2026-03-02T10:25:00', 25, '')",
        [task_id],
    )
    .unwrap();

    let from = "2026-03-01T00:00:00"
        .parse::<chrono::NaiveDateTime>()
        .unwrap();
    let to = "2026-03-01T23:59:59"
        .parse::<chrono::NaiveDateTime>()
        .unwrap();

    // Act
    let results = queries::sessions_in_range(&conn, from, to).unwrap();

    // Assert
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0.description, "Task A");
    assert_eq!(results[0].1.notes, "did work");
}

#[test]
fn sessions_in_range_excludes_open_sessions() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task", &TaskList::Inbox).unwrap();
    conn.execute(
        "INSERT INTO session (task_id, begin_at, duration_min, notes)
         VALUES (?1, '2026-03-01T10:00:00', 25, '')",
        [task_id],
    )
    .unwrap();

    let from = "2026-03-01T00:00:00"
        .parse::<chrono::NaiveDateTime>()
        .unwrap();
    let to = "2026-03-01T23:59:59"
        .parse::<chrono::NaiveDateTime>()
        .unwrap();

    // Act
    let results = queries::sessions_in_range(&conn, from, to).unwrap();

    // Assert
    assert!(results.is_empty());
}

#[test]
fn todos_completed_in_range_returns_todos_completed_within_window() {
    // Arrange
    let conn = setup();
    let task_id = queries::insert_task(&conn, "Task B", &TaskList::Inbox).unwrap();
    conn.execute(
        "INSERT INTO todo (task_id, description, done, position, completed_at)
         VALUES (?1, 'Write report', 1, 0, '2026-03-01T15:00:00')",
        [task_id],
    )
    .unwrap();
    // Outside window
    conn.execute(
        "INSERT INTO todo (task_id, description, done, position, completed_at)
         VALUES (?1, 'Other todo', 1, 1, '2026-03-02T15:00:00')",
        [task_id],
    )
    .unwrap();

    let from = "2026-03-01T00:00:00"
        .parse::<chrono::NaiveDateTime>()
        .unwrap();
    let to = "2026-03-01T23:59:59"
        .parse::<chrono::NaiveDateTime>()
        .unwrap();

    // Act
    let results = queries::todos_completed_in_range(&conn, from, to).unwrap();

    // Assert
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0.description, "Task B");
    assert_eq!(results[0].1.description, "Write report");
}
