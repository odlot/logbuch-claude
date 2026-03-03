use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: i64,
    pub task_id: i64,
    pub begin_at: NaiveDateTime,
    pub end_at: Option<NaiveDateTime>,
    pub duration_min: i32,
    pub notes: String,
}
