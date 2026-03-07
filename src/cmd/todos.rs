use anyhow::{bail, Result};
use rusqlite::Connection;

use crate::db::queries;

pub fn add_todo(conn: &Connection, task_id: i64, description: &str) -> Result<()> {
    queries::get_task(conn, task_id)?; // verify task exists
    let todo_id = queries::insert_todo(conn, task_id, description)?;
    println!("Added todo {} to #{}.", todo_id, task_id);
    Ok(())
}

pub fn check(conn: &Connection, task_id: i64, todo_id: i64) -> Result<()> {
    queries::get_task(conn, task_id)?; // verify task exists
    let todos = queries::list_todos(conn, task_id)?;
    let todo = todos
        .iter()
        .find(|t| t.id == todo_id)
        .ok_or_else(|| anyhow::anyhow!("Todo {} not found on task #{}.", todo_id, task_id))?;

    queries::toggle_todo(conn, todo_id)?;

    if todo.done {
        println!("Unchecked todo {} on #{}.", todo_id, task_id);
    } else {
        println!("Checked todo {} on #{}.", todo_id, task_id);
    }
    Ok(())
}

pub fn edit_todo(conn: &Connection, task_id: i64, todo_id: i64, description: &str) -> Result<()> {
    queries::get_task(conn, task_id)?; // verify task exists
    let todos = queries::list_todos(conn, task_id)?;
    if !todos.iter().any(|t| t.id == todo_id) {
        bail!("Todo {} not found on task #{}.", todo_id, task_id);
    }
    queries::update_todo_description(conn, todo_id, description)?;
    println!("Updated todo {} on #{}.", todo_id, task_id);
    Ok(())
}
