use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Todo {
    pub id: i64,
    pub task_id: i64,
    pub description: String,
    pub done: bool,
    pub position: i32,
    pub completed_at: Option<NaiveDateTime>,
}
