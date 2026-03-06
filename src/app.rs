use std::time::Instant;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;
use rusqlite::Connection;

use crate::config::Config;
use crate::db::queries;
use crate::model::{Session, Task, TaskList, Todo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Board,
    TaskDetail(i64),
    ActiveSession(i64),
    Archive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTarget {
    NewTask,
    EditDescription,
    NewTodo,
    EditTodo,
    SessionNote,
    SessionDuration,
}

#[derive(Debug, Clone)]
pub enum ConfirmDelete {
    Task(i64),
    Todo(i64),
    Session(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailSection {
    Description,
    Todos,
    Sessions,
}

impl DetailSection {
    pub fn next(&self) -> Self {
        match self {
            DetailSection::Description => DetailSection::Todos,
            DetailSection::Todos => DetailSection::Sessions,
            DetailSection::Sessions => DetailSection::Description,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            DetailSection::Description => DetailSection::Sessions,
            DetailSection::Todos => DetailSection::Description,
            DetailSection::Sessions => DetailSection::Todos,
        }
    }
}

pub struct App {
    pub view: View,
    pub input_mode: InputMode,
    pub input_target: InputTarget,
    pub running: bool,
    pub show_help: bool,

    // Board state
    pub active_column: TaskList,
    pub selected_index: [usize; 3],

    // Cached data
    pub tasks_inbox: Vec<Task>,
    pub tasks_in_progress: Vec<Task>,
    pub tasks_backlog: Vec<Task>,

    // Task detail state
    pub detail_section: DetailSection,
    pub todo_list_state: ListState,
    pub session_list_state: ListState,
    pub todos: Vec<Todo>,
    pub sessions: Vec<Session>,

    // Pending delete confirmation
    pub confirm_delete: Option<ConfirmDelete>,

    // Active session state
    pub active_session: Option<Session>,
    pub session_start: Option<Instant>,
    pub notification_sent: bool,
    pub session_task_description: String,

    // Text input
    pub input_buffer: String,
    pub input_cursor: usize,

    // Status message
    pub status_message: Option<(String, Instant)>,

    // Pending report key (for r d / r w sequence)
    pub pending_r: bool,

    // Search overlay
    pub show_search: bool,
    pub search_results: Vec<Task>,
    pub search_selected: usize,

    // Archive view
    pub archive_tasks: Vec<Task>,
    pub archive_list_state: ListState,

    // DB and config
    pub db: Connection,
    pub config: Config,
}

impl App {
    pub fn new(db: Connection, config: Config) -> Result<Self> {
        let mut app = Self {
            view: View::Board,
            input_mode: InputMode::Normal,
            input_target: InputTarget::NewTask,
            running: true,
            show_help: false,

            active_column: TaskList::Inbox,
            selected_index: [0; 3],

            tasks_inbox: Vec::new(),
            tasks_in_progress: Vec::new(),
            tasks_backlog: Vec::new(),

            detail_section: DetailSection::Description,
            todo_list_state: ListState::default(),
            session_list_state: ListState::default(),
            todos: Vec::new(),
            sessions: Vec::new(),

            confirm_delete: None,

            active_session: None,
            session_start: None,
            notification_sent: false,
            session_task_description: String::new(),

            input_buffer: String::new(),
            input_cursor: 0,

            status_message: None,
            pending_r: false,

            show_search: false,
            search_results: Vec::new(),
            search_selected: 0,

            archive_tasks: Vec::new(),
            archive_list_state: ListState::default(),

            db,
            config,
        };
        app.reload_tasks()?;
        let purged = queries::purge_stale_tasks(&app.db, 28)?;
        if !purged.is_empty() {
            app.set_status(format!(
                "Auto-deleted {} stale task(s) (28+ days inactive)",
                purged.len()
            ));
        }
        Ok(app)
    }

    pub fn reload_tasks(&mut self) -> Result<()> {
        self.tasks_inbox = queries::list_tasks(&self.db, &TaskList::Inbox)?;
        self.tasks_in_progress = queries::list_tasks(&self.db, &TaskList::InProgress)?;
        self.tasks_backlog = queries::list_tasks(&self.db, &TaskList::Backlog)?;
        // Clamp selection indices
        self.clamp_selection(TaskList::Inbox);
        self.clamp_selection(TaskList::InProgress);
        self.clamp_selection(TaskList::Backlog);
        Ok(())
    }

    pub fn reload_archive(&mut self) -> Result<()> {
        self.archive_tasks = queries::list_tasks(&self.db, &TaskList::Done)?;
        if self.archive_tasks.is_empty() {
            self.archive_list_state.select(None);
        } else {
            let idx = self
                .archive_list_state
                .selected()
                .unwrap_or(0)
                .min(self.archive_tasks.len() - 1);
            self.archive_list_state.select(Some(idx));
        }
        Ok(())
    }

    fn handle_archive_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.view = View::Board;
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.archive_tasks.is_empty() {
                    let idx = self
                        .archive_list_state
                        .selected()
                        .unwrap_or(0)
                        .saturating_add(1)
                        .min(self.archive_tasks.len() - 1);
                    self.archive_list_state.select(Some(idx));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.archive_tasks.is_empty() {
                    let idx = self
                        .archive_list_state
                        .selected()
                        .unwrap_or(0)
                        .saturating_sub(1);
                    self.archive_list_state.select(Some(idx));
                }
            }
            KeyCode::Enter => {
                if let Some(idx) = self.archive_list_state.selected() {
                    if let Some(task) = self.archive_tasks.get(idx) {
                        let task_id = task.id;
                        self.view = View::TaskDetail(task_id);
                        self.reload_detail(task_id)?;
                    }
                }
            }
            KeyCode::Char('r') => {
                if let Some(idx) = self.archive_list_state.selected() {
                    if let Some(task) = self.archive_tasks.get(idx) {
                        let task_id = task.id;
                        let desc = task.description.clone();
                        queries::restore_task(&self.db, task_id)?;
                        self.reload_archive()?;
                        self.reload_tasks()?;
                        self.set_status(format!("Restored '{}' to Inbox", desc));
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(idx) = self.archive_list_state.selected() {
                    if let Some(task) = self.archive_tasks.get(idx) {
                        let task_id = task.id;
                        match self.confirm_delete.take() {
                            Some(ConfirmDelete::Task(id)) if id == task_id => {
                                queries::delete_task(&self.db, task_id)?;
                                self.reload_archive()?;
                                self.set_status("Task permanently deleted");
                            }
                            _ => {
                                self.confirm_delete = Some(ConfirmDelete::Task(task_id));
                                self.set_status(
                                    "Press d again to permanently delete, any other key to cancel",
                                );
                            }
                        }
                    }
                }
            }
            _ => {
                self.confirm_delete = None;
            }
        }
        Ok(())
    }

    pub fn selected_todo_index(&self) -> usize {
        self.todo_list_state.selected().unwrap_or(0)
    }

    pub fn selected_session_index(&self) -> usize {
        self.session_list_state.selected().unwrap_or(0)
    }

    fn reload_detail(&mut self, task_id: i64) -> Result<()> {
        self.todos = queries::list_todos(&self.db, task_id)?;
        self.sessions = queries::list_sessions(&self.db, task_id)?;
        // Clamp todo selection
        if self.todos.is_empty() {
            self.todo_list_state.select(None);
        } else {
            let idx = self.selected_todo_index().min(self.todos.len() - 1);
            self.todo_list_state.select(Some(idx));
        }
        // Clamp session selection
        if self.sessions.is_empty() {
            self.session_list_state.select(None);
        } else {
            let idx = self.selected_session_index().min(self.sessions.len() - 1);
            self.session_list_state.select(Some(idx));
        }
        Ok(())
    }

    pub fn tasks_for_list(&self, list: &TaskList) -> &[Task] {
        match list {
            TaskList::Inbox => &self.tasks_inbox,
            TaskList::InProgress => &self.tasks_in_progress,
            TaskList::Backlog => &self.tasks_backlog,
            TaskList::Done => &self.archive_tasks,
        }
    }

    fn clamp_selection(&mut self, list: TaskList) {
        let len = self.tasks_for_list(&list).len();
        let idx = list.index();
        if len == 0 {
            self.selected_index[idx] = 0;
        } else if self.selected_index[idx] >= len {
            self.selected_index[idx] = len - 1;
        }
    }

    fn selected_task(&self) -> Option<&Task> {
        let tasks = self.tasks_for_list(&self.active_column);
        let idx = self.selected_index[self.active_column.index()];
        tasks.get(idx)
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    /// Returns the byte index in `input_buffer` that corresponds to the current
    /// char-based `input_cursor`. `String::insert` and `String::remove` require
    /// byte indices, so every mutation goes through this helper.
    fn input_byte_idx(&self) -> usize {
        self.input_buffer
            .char_indices()
            .nth(self.input_cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.input_buffer.len())
    }

    fn start_input(&mut self, target: InputTarget, prefill: &str) {
        self.input_mode = InputMode::Editing;
        self.input_target = target;
        self.input_buffer = prefill.to_string();
        self.input_cursor = self.input_buffer.chars().count();
    }

    fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.input_cursor = 0;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Clear status after 3 seconds
        if let Some((_, time)) = &self.status_message {
            if time.elapsed().as_secs() >= 3 {
                self.status_message = None;
            }
        }

        if self.show_help {
            self.show_help = false;
            return Ok(());
        }

        if self.show_search {
            return self.handle_search_key(key);
        }

        match self.input_mode {
            InputMode::Editing => self.handle_editing_key(key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.show_search = false;
                self.input_buffer.clear();
                self.input_cursor = 0;
                self.search_results.clear();
                self.search_selected = 0;
            }
            KeyCode::Enter => {
                if let Some(task) = self.search_results.get(self.search_selected) {
                    let task_id = task.id;
                    self.show_search = false;
                    self.input_buffer.clear();
                    self.input_cursor = 0;
                    self.search_results.clear();
                    self.search_selected = 0;
                    self.view = View::TaskDetail(task_id);
                    self.detail_section = DetailSection::Description;
                    self.todo_list_state = ListState::default();
                    self.session_list_state = ListState::default();
                    self.reload_detail(task_id)?;
                }
            }
            KeyCode::Down => {
                if !self.search_results.is_empty()
                    && self.search_selected < self.search_results.len() - 1
                {
                    self.search_selected += 1;
                }
            }
            KeyCode::Up => {
                if self.search_selected > 0 {
                    self.search_selected -= 1;
                }
            }
            KeyCode::Char(c) => {
                self.input_buffer.insert(self.input_byte_idx(), c);
                self.input_cursor += 1;
                self.update_search_results();
            }
            KeyCode::Backspace => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    self.input_buffer.remove(self.input_byte_idx());
                    self.update_search_results();
                }
            }
            KeyCode::Left => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.input_cursor < self.input_buffer.chars().count() {
                    self.input_cursor += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_editing_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                let text = self.input_buffer.trim().to_string();
                if text.is_empty() {
                    self.cancel_input();
                    return Ok(());
                }
                match self.input_target {
                    InputTarget::NewTask => {
                        queries::insert_task(&self.db, &text, &self.active_column)?;
                        self.reload_tasks()?;
                        let tasks = self.tasks_for_list(&self.active_column);
                        self.selected_index[self.active_column.index()] =
                            if tasks.is_empty() { 0 } else { tasks.len() - 1 };
                        self.set_status("Task created");
                    }
                    InputTarget::EditDescription => {
                        if let View::TaskDetail(task_id) = self.view {
                            queries::update_task_description(&self.db, task_id, &text)?;
                            self.reload_tasks()?;
                            self.set_status("Description updated");
                        }
                    }
                    InputTarget::NewTodo => {
                        if let View::TaskDetail(task_id) = self.view {
                            queries::insert_todo(&self.db, task_id, &text)?;
                            self.reload_detail(task_id)?;
                            if !self.todos.is_empty() {
                                self.todo_list_state.select(Some(self.todos.len() - 1));
                            }
                            self.set_status("Todo added");
                        }
                    }
                    InputTarget::EditTodo => {
                        if let View::TaskDetail(task_id) = self.view {
                            let idx = self.selected_todo_index();
                            if let Some(todo) = self.todos.get(idx) {
                                let todo_id = todo.id;
                                queries::update_todo_description(&self.db, todo_id, &text)?;
                            }
                            self.reload_detail(task_id)?;
                            self.set_status("Todo updated");
                        }
                    }
                    InputTarget::SessionDuration => {
                        if let View::TaskDetail(task_id) = self.view {
                            match text.parse::<u32>() {
                                Ok(minutes) if minutes > 0 => {
                                    let duration = minutes as i32;
                                    let session_id =
                                        queries::start_session(&self.db, task_id, duration)?;
                                    let session = queries::get_active_session(&self.db)?;
                                    if let Some(s) = session {
                                        let task = queries::get_task(&self.db, task_id)?;
                                        self.session_task_description = task.description.clone();
                                        self.active_session = Some(s);
                                        self.session_start = Some(Instant::now());
                                        self.notification_sent = false;
                                        self.view = View::ActiveSession(session_id);
                                        self.cancel_input();
                                        self.start_input(InputTarget::SessionNote, "");
                                        self.set_status("Session started");
                                        return Ok(());
                                    }
                                }
                                _ => {
                                    self.set_status("Invalid duration: enter a number > 0");
                                    self.cancel_input();
                                    return Ok(());
                                }
                            }
                        }
                    }
                    InputTarget::SessionNote => {
                        if let Some(ref session) = self.active_session {
                            queries::append_session_notes(&self.db, session.id, &text)?;
                            // Reload session to get updated notes
                            if let Some(updated) = queries::get_active_session(&self.db)? {
                                self.active_session = Some(updated);
                            }
                        }
                        // Stay in editing mode for session notes
                        self.input_buffer.clear();
                        self.input_cursor = 0;
                        return Ok(());
                    }
                }
                self.cancel_input();
            }
            KeyCode::Esc => {
                self.cancel_input();
            }
            KeyCode::Char(c) => {
                self.input_buffer.insert(self.input_byte_idx(), c);
                self.input_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    self.input_buffer.remove(self.input_byte_idx());
                }
            }
            KeyCode::Left => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.input_cursor < self.input_buffer.chars().count() {
                    self.input_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.input_cursor = 0;
            }
            KeyCode::End => {
                self.input_cursor = self.input_buffer.chars().count();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle pending 'r' key for report generation
        if self.pending_r {
            self.pending_r = false;
            if let View::Board = self.view {
                match key.code {
                    KeyCode::Char('d') => {
                        return self.generate_daily_summary();
                    }
                    KeyCode::Char('w') => {
                        return self.generate_weekly_summary();
                    }
                    _ => {
                        return Ok(());
                    }
                }
            }
            return Ok(());
        }

        match &self.view.clone() {
            View::Board => self.handle_board_key(key),
            View::TaskDetail(task_id) => self.handle_detail_key(key, *task_id),
            View::ActiveSession(_session_id) => self.handle_session_key(key),
            View::Archive => self.handle_archive_key(key),
        }
    }

    fn handle_board_key(&mut self, key: KeyEvent) -> Result<()> {
        // Cancel pending confirmation on any key other than 'd'
        if self.confirm_delete.is_some() && !matches!(key.code, KeyCode::Char('d')) {
            self.confirm_delete = None;
        }

        match key.code {
            KeyCode::Char('q') => {
                if self.active_session.is_some() {
                    self.set_status("Cannot quit during active session");
                } else {
                    self.running = false;
                }
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if let Some(left) = self.active_column.left() {
                    self.active_column = left;
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if let Some(right) = self.active_column.right() {
                    self.active_column = right;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let idx = self.active_column.index();
                let len = self.tasks_for_list(&self.active_column).len();
                if len > 0 && self.selected_index[idx] < len - 1 {
                    self.selected_index[idx] += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let idx = self.active_column.index();
                if self.selected_index[idx] > 0 {
                    self.selected_index[idx] -= 1;
                }
            }
            KeyCode::Enter => {
                if let Some(task) = self.selected_task() {
                    let task_id = task.id;
                    self.view = View::TaskDetail(task_id);
                    self.detail_section = DetailSection::Description;
                    self.todo_list_state = ListState::default();
                    self.session_list_state = ListState::default();
                    self.reload_detail(task_id)?;
                }
            }
            KeyCode::Char('n') => {
                self.start_input(InputTarget::NewTask, "");
            }
            KeyCode::Char('d') => match self.confirm_delete.take() {
                Some(ConfirmDelete::Task(id)) => {
                    queries::delete_task(&self.db, id)?;
                    self.reload_tasks()?;
                    self.set_status("Task deleted");
                }
                _ => {
                    if let Some(task) = self.selected_task() {
                        let id = task.id;
                        let preview: String = task.description.chars().take(30).collect();
                        self.confirm_delete = Some(ConfirmDelete::Task(id));
                        self.set_status(format!("Delete '{}'? Press d again to confirm", preview));
                    }
                }
            },
            KeyCode::Char('H') => {
                if let Some(task) = self.selected_task() {
                    if let Some(target) = self.active_column.left() {
                        let task_id = task.id;
                        queries::move_task(&self.db, task_id, &target)?;
                        self.reload_tasks()?;
                        self.set_status(format!("Moved to {}", target.display_name()));
                    }
                }
            }
            KeyCode::Char('L') => {
                if let Some(task) = self.selected_task() {
                    if let Some(target) = self.active_column.right() {
                        let task_id = task.id;
                        queries::move_task(&self.db, task_id, &target)?;
                        self.reload_tasks()?;
                        self.set_status(format!("Moved to {}", target.display_name()));
                    }
                }
            }
            KeyCode::Char('r') => {
                self.pending_r = true;
            }
            KeyCode::Char('a') => {
                self.reload_archive()?;
                self.view = View::Archive;
            }
            KeyCode::Char('A') => {
                if let Some(task) = self.selected_task() {
                    let task_id = task.id;
                    let desc = task.description.clone();
                    queries::archive_task(&self.db, task_id)?;
                    self.reload_tasks()?;
                    self.set_status(format!("Archived '{}'", desc));
                }
            }
            KeyCode::Char('/') => {
                self.show_search = true;
                self.input_buffer.clear();
                self.input_cursor = 0;
                self.reload_tasks()?;
                self.update_search_results();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_detail_key(&mut self, key: KeyEvent, task_id: i64) -> Result<()> {
        // Cancel pending confirmation on any key other than 'D'
        if self.confirm_delete.is_some() && !matches!(key.code, KeyCode::Char('D')) {
            self.confirm_delete = None;
        }

        match key.code {
            KeyCode::Esc => {
                self.confirm_delete = None;
                self.view = View::Board;
                self.reload_tasks()?;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Tab => {
                self.detail_section = self.detail_section.next();
            }
            KeyCode::BackTab => {
                self.detail_section = self.detail_section.prev();
            }
            KeyCode::Char('j') | KeyCode::Down => match self.detail_section {
                DetailSection::Todos => {
                    let idx = self.selected_todo_index();
                    if !self.todos.is_empty() && idx < self.todos.len() - 1 {
                        self.todo_list_state.select(Some(idx + 1));
                    }
                }
                DetailSection::Sessions => {
                    let idx = self.selected_session_index();
                    if !self.sessions.is_empty() && idx < self.sessions.len() - 1 {
                        self.session_list_state.select(Some(idx + 1));
                    }
                }
                _ => {}
            },
            KeyCode::Char('k') | KeyCode::Up => match self.detail_section {
                DetailSection::Todos => {
                    let idx = self.selected_todo_index();
                    if idx > 0 {
                        self.todo_list_state.select(Some(idx - 1));
                    }
                }
                DetailSection::Sessions => {
                    let idx = self.selected_session_index();
                    if idx > 0 {
                        self.session_list_state.select(Some(idx - 1));
                    }
                }
                _ => {}
            },
            KeyCode::Char('e') => match self.detail_section {
                DetailSection::Todos => {
                    let idx = self.selected_todo_index();
                    if let Some(desc) = self.todos.get(idx).map(|t| t.description.clone()) {
                        self.start_input(InputTarget::EditTodo, &desc);
                    }
                }
                _ => {
                    let task = queries::get_task(&self.db, task_id)?;
                    self.start_input(InputTarget::EditDescription, &task.description);
                }
            },
            KeyCode::Char('a') => {
                self.detail_section = DetailSection::Todos;
                self.start_input(InputTarget::NewTodo, "");
            }
            KeyCode::Char('x') => {
                if self.detail_section == DetailSection::Todos {
                    if let Some(todo) = self.todos.get(self.selected_todo_index()) {
                        let todo_id = todo.id;
                        queries::toggle_todo(&self.db, todo_id)?;
                        self.reload_detail(task_id)?;
                        self.set_status("Todo toggled");
                    }
                }
            }
            KeyCode::Char('D') => match self.confirm_delete.take() {
                Some(ConfirmDelete::Todo(id)) => {
                    queries::delete_todo(&self.db, id)?;
                    self.reload_detail(task_id)?;
                    self.set_status("Todo deleted");
                }
                Some(ConfirmDelete::Session(id)) => {
                    queries::delete_session(&self.db, id)?;
                    self.reload_detail(task_id)?;
                    self.set_status("Session deleted");
                }
                _ => match self.detail_section {
                    DetailSection::Todos => {
                        let idx = self.selected_todo_index();
                        if let Some(todo) = self.todos.get(idx) {
                            let preview: String = todo.description.chars().take(25).collect();
                            self.confirm_delete = Some(ConfirmDelete::Todo(todo.id));
                            self.set_status(format!(
                                "Delete todo '{}'? Press D again to confirm",
                                preview
                            ));
                        }
                    }
                    DetailSection::Sessions => {
                        let idx = self.selected_session_index();
                        if let Some(session) = self.sessions.get(idx) {
                            if self.active_session.as_ref().map(|s| s.id) == Some(session.id) {
                                self.set_status("Cannot delete active session");
                            } else {
                                let preview = session.begin_at.format("%Y-%m-%d %H:%M").to_string();
                                self.confirm_delete = Some(ConfirmDelete::Session(session.id));
                                self.set_status(format!(
                                    "Delete session {}? Press D again to confirm",
                                    preview
                                ));
                            }
                        }
                    }
                    _ => {}
                },
            },
            KeyCode::Char('J') => {
                if self.detail_section == DetailSection::Todos {
                    let idx = self.selected_todo_index();
                    if let Some(todo) = self.todos.get(idx) {
                        let todo_id = todo.id;
                        queries::move_todo_down(&self.db, todo_id, task_id)?;
                        self.reload_detail(task_id)?;
                        if idx + 1 < self.todos.len() {
                            self.todo_list_state.select(Some(idx + 1));
                        }
                    }
                }
            }
            KeyCode::Char('K') => {
                if self.detail_section == DetailSection::Todos {
                    let idx = self.selected_todo_index();
                    if let Some(todo) = self.todos.get(idx) {
                        let todo_id = todo.id;
                        queries::move_todo_up(&self.db, todo_id, task_id)?;
                        self.reload_detail(task_id)?;
                        if idx > 0 {
                            self.todo_list_state.select(Some(idx - 1));
                        }
                    }
                }
            }
            KeyCode::Char('/') => {
                self.show_search = true;
                self.input_buffer.clear();
                self.input_cursor = 0;
                self.reload_tasks()?;
                self.update_search_results();
            }
            KeyCode::Char('s') => {
                if self.active_session.is_some() {
                    self.set_status("A session is already active");
                } else {
                    let preset = self
                        .sessions
                        .first()
                        .map(|s| s.duration_min as u32)
                        .unwrap_or(self.config.session_duration_min);
                    self.start_input(InputTarget::SessionDuration, &preset.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_session_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.end_current_session()?;
            }
            _ => {
                // In session view, all keys go to editing (notes)
                if self.input_mode == InputMode::Normal {
                    self.start_input(InputTarget::SessionNote, "");
                }
                // Forward to editing handler
                if self.input_mode == InputMode::Editing {
                    // Don't re-handle Esc, it's caught above
                    if key.code != KeyCode::Esc {
                        self.handle_editing_key(key)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn end_current_session(&mut self) -> Result<()> {
        if let Some(ref session) = self.active_session {
            let session_id = session.id;
            let task_id = session.task_id;
            queries::end_session(&self.db, session_id)?;
            self.active_session = None;
            self.session_start = None;
            self.notification_sent = false;
            self.cancel_input();
            self.view = View::TaskDetail(task_id);
            self.reload_detail(task_id)?;
            self.set_status("Session ended");
        }
        Ok(())
    }

    pub fn tick(&mut self) -> Result<()> {
        // Clear expired status message
        if let Some((_, time)) = &self.status_message {
            if time.elapsed().as_secs() >= 3 {
                self.status_message = None;
            }
        }

        // Check session timer
        if let (Some(ref session), Some(start)) = (&self.active_session, self.session_start) {
            let elapsed = start.elapsed();
            let total = std::time::Duration::from_secs(session.duration_min as u64 * 60);
            if elapsed >= total && !self.notification_sent {
                self.notification_sent = true;
                self.send_notification();
                // End the session in DB
                queries::end_session(&self.db, session.id)?;
            }
        }
        Ok(())
    }

    fn send_notification(&self) {
        let desc = &self.session_task_description;
        let mut notification = notify_rust::Notification::new();
        notification
            .summary("Logbuch: Session Complete")
            .body(&format!("Pomodoro session finished for: {}", desc))
            .icon("dialog-information");

        #[cfg(target_os = "linux")]
        {
            notification
                .urgency(notify_rust::Urgency::Critical)
                .timeout(notify_rust::Timeout::Never);
        }

        let _ = notification.show();
    }

    pub fn session_remaining_secs(&self) -> Option<i64> {
        if let (Some(ref session), Some(start)) = (&self.active_session, self.session_start) {
            let elapsed = start.elapsed().as_secs() as i64;
            let total = session.duration_min as i64 * 60;
            Some((total - elapsed).max(0))
        } else {
            None
        }
    }

    pub fn session_progress(&self) -> Option<f64> {
        if let (Some(ref session), Some(start)) = (&self.active_session, self.session_start) {
            let elapsed = start.elapsed().as_secs_f64();
            let total = session.duration_min as f64 * 60.0;
            Some((elapsed / total).min(1.0))
        } else {
            None
        }
    }

    pub fn fuzzy_match(query: &str, target: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let target_lower = target.to_lowercase();
        let query_lower = query.to_lowercase();
        let mut target_chars = target_lower.chars();
        for qc in query_lower.chars() {
            if !target_chars.any(|tc| tc == qc) {
                return false;
            }
        }
        true
    }

    fn update_search_results(&mut self) {
        let query = self.input_buffer.clone();
        self.search_results = self
            .tasks_inbox
            .iter()
            .chain(self.tasks_in_progress.iter())
            .chain(self.tasks_backlog.iter())
            .filter(|t| Self::fuzzy_match(&query, &t.description))
            .cloned()
            .collect();
        self.search_selected = 0;
    }

    fn generate_daily_summary(&mut self) -> Result<()> {
        let path = crate::summary::generate_daily(
            &self.db,
            chrono::Local::now().date_naive(),
            &self.config.summary_export_dir,
        )?;
        self.set_status(format!("Daily summary: {}", path.display()));
        Ok(())
    }

    fn generate_weekly_summary(&mut self) -> Result<()> {
        let path = crate::summary::generate_weekly(
            &self.db,
            chrono::Local::now().date_naive(),
            &self.config.summary_export_dir,
        )?;
        self.set_status(format!("Weekly summary: {}", path.display()));
        Ok(())
    }
}
