use chrono::NaiveDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskList {
    Inbox,
    InProgress,
    Backlog,
}

impl TaskList {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskList::Inbox => "inbox",
            TaskList::InProgress => "in_progress",
            TaskList::Backlog => "backlog",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "inbox" => Some(TaskList::Inbox),
            "in_progress" => Some(TaskList::InProgress),
            "backlog" => Some(TaskList::Backlog),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TaskList::Inbox => "Inbox",
            TaskList::InProgress => "In Progress",
            TaskList::Backlog => "Backlog",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub description: String,
    pub list: TaskList,
    pub position: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
