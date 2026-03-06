use logbuch::model::TaskList;

// ---------------------------------------------------------------------------
// TaskList::as_str
// ---------------------------------------------------------------------------

#[test]
fn task_list_as_str_returns_correct_string_for_each_variant() {
    // Arrange / Act / Assert (pure enum mapping — no setup needed)
    assert_eq!(TaskList::Inbox.as_str(), "inbox");
    assert_eq!(TaskList::InProgress.as_str(), "in_progress");
    assert_eq!(TaskList::Backlog.as_str(), "backlog");
    assert_eq!(TaskList::Done.as_str(), "done");
}

// ---------------------------------------------------------------------------
// TaskList::from_str
// ---------------------------------------------------------------------------

#[test]
fn task_list_from_str_parses_all_valid_strings() {
    assert_eq!(TaskList::from_str("inbox"), Some(TaskList::Inbox));
    assert_eq!(
        TaskList::from_str("in_progress"),
        Some(TaskList::InProgress)
    );
    assert_eq!(TaskList::from_str("backlog"), Some(TaskList::Backlog));
    assert_eq!(TaskList::from_str("done"), Some(TaskList::Done));
}

#[test]
fn task_list_from_str_returns_none_for_unknown_string() {
    assert_eq!(TaskList::from_str("INBOX"), None);
    assert_eq!(TaskList::from_str(""), None);
    assert_eq!(TaskList::from_str("unknown"), None);
}

// ---------------------------------------------------------------------------
// TaskList::display_name
// ---------------------------------------------------------------------------

#[test]
fn task_list_display_name_returns_human_readable_label_for_each_variant() {
    assert_eq!(TaskList::Inbox.display_name(), "Inbox");
    assert_eq!(TaskList::InProgress.display_name(), "In Progress");
    assert_eq!(TaskList::Backlog.display_name(), "Backlog");
    assert_eq!(TaskList::Done.display_name(), "Done");
}

// ---------------------------------------------------------------------------
// TaskList::index
// ---------------------------------------------------------------------------

#[test]
fn task_list_index_returns_correct_index_for_each_variant() {
    assert_eq!(TaskList::Inbox.index(), 0);
    assert_eq!(TaskList::InProgress.index(), 1);
    assert_eq!(TaskList::Backlog.index(), 2);
    assert_eq!(TaskList::Done.index(), 3);
}

// ---------------------------------------------------------------------------
// TaskList::left / right (board navigation)
// ---------------------------------------------------------------------------

#[test]
fn task_list_left_returns_none_for_inbox_and_done() {
    assert_eq!(TaskList::Inbox.left(), None);
    assert_eq!(TaskList::Done.left(), None);
}

#[test]
fn task_list_left_navigates_to_correct_preceding_column() {
    assert_eq!(TaskList::InProgress.left(), Some(TaskList::Inbox));
    assert_eq!(TaskList::Backlog.left(), Some(TaskList::InProgress));
}

#[test]
fn task_list_right_returns_none_for_backlog_and_done() {
    assert_eq!(TaskList::Backlog.right(), None);
    assert_eq!(TaskList::Done.right(), None);
}

#[test]
fn task_list_right_navigates_to_correct_following_column() {
    assert_eq!(TaskList::Inbox.right(), Some(TaskList::InProgress));
    assert_eq!(TaskList::InProgress.right(), Some(TaskList::Backlog));
}
