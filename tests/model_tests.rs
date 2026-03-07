use logbuch::model::TaskList;

#[test]
fn task_list_as_str_returns_correct_string_for_each_variant() {
    assert_eq!(TaskList::Inbox.as_str(), "inbox");
    assert_eq!(TaskList::InProgress.as_str(), "in_progress");
    assert_eq!(TaskList::Backlog.as_str(), "backlog");
}

#[test]
fn task_list_from_str_parses_all_valid_strings() {
    assert_eq!(TaskList::from_str("inbox"), Some(TaskList::Inbox));
    assert_eq!(
        TaskList::from_str("in_progress"),
        Some(TaskList::InProgress)
    );
    assert_eq!(TaskList::from_str("backlog"), Some(TaskList::Backlog));
}

#[test]
fn task_list_from_str_returns_none_for_unknown_string() {
    assert_eq!(TaskList::from_str("INBOX"), None);
    assert_eq!(TaskList::from_str(""), None);
    assert_eq!(TaskList::from_str("unknown"), None);
    // "done" is no longer a valid list — tasks are deleted on completion
    assert_eq!(TaskList::from_str("done"), None);
}

#[test]
fn task_list_display_name_returns_human_readable_label_for_each_variant() {
    assert_eq!(TaskList::Inbox.display_name(), "Inbox");
    assert_eq!(TaskList::InProgress.display_name(), "In Progress");
    assert_eq!(TaskList::Backlog.display_name(), "Backlog");
}
