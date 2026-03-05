/// A single keybinding entry shown in the help overlay and status bar.
pub struct Keybinding {
    /// Short key label, e.g. `"h / ←"` or `"r d"`.
    pub keys: &'static str,
    /// One-line description of what the key does.
    pub description: &'static str,
}

pub const BOARD: &[Keybinding] = &[
    Keybinding {
        keys: "h / ←",
        description: "Focus left column",
    },
    Keybinding {
        keys: "l / →",
        description: "Focus right column",
    },
    Keybinding {
        keys: "j / ↓",
        description: "Select next task",
    },
    Keybinding {
        keys: "k / ↑",
        description: "Select previous task",
    },
    Keybinding {
        keys: "Enter",
        description: "Open task detail",
    },
    Keybinding {
        keys: "n",
        description: "New task",
    },
    Keybinding {
        keys: "d",
        description: "Delete task (press d again to confirm)",
    },
    Keybinding {
        keys: "A",
        description: "Archive selected task",
    },
    Keybinding {
        keys: "H",
        description: "Move task left",
    },
    Keybinding {
        keys: "L",
        description: "Move task right",
    },
    Keybinding {
        keys: "a",
        description: "Open archive view",
    },
    Keybinding {
        keys: "r d",
        description: "Generate daily summary report",
    },
    Keybinding {
        keys: "r w",
        description: "Generate weekly summary report",
    },
    Keybinding {
        keys: "/",
        description: "Search tasks",
    },
    Keybinding {
        keys: "q",
        description: "Quit",
    },
    Keybinding {
        keys: "?",
        description: "Toggle help",
    },
];

pub const TASK_DETAIL: &[Keybinding] = &[
    Keybinding {
        keys: "Esc",
        description: "Back to board",
    },
    Keybinding {
        keys: "Tab",
        description: "Next section",
    },
    Keybinding {
        keys: "Shift+Tab",
        description: "Previous section",
    },
    Keybinding {
        keys: "j / ↓",
        description: "Select next item in section",
    },
    Keybinding {
        keys: "k / ↑",
        description: "Select previous item in section",
    },
    Keybinding {
        keys: "e",
        description: "Edit description or selected todo",
    },
    Keybinding {
        keys: "a",
        description: "Add todo",
    },
    Keybinding {
        keys: "x",
        description: "Toggle todo done/undone",
    },
    Keybinding {
        keys: "D",
        description: "Delete todo/session (press D again to confirm)",
    },
    Keybinding {
        keys: "J",
        description: "Move todo down",
    },
    Keybinding {
        keys: "K",
        description: "Move todo up",
    },
    Keybinding {
        keys: "s",
        description: "Start session",
    },
    Keybinding {
        keys: "/",
        description: "Search tasks",
    },
    Keybinding {
        keys: "?",
        description: "Toggle help",
    },
];

pub const ACTIVE_SESSION: &[Keybinding] = &[
    Keybinding {
        keys: "Esc",
        description: "End session early",
    },
    Keybinding {
        keys: "Enter",
        description: "Submit current note line",
    },
    Keybinding {
        keys: "<type>",
        description: "Add to current note line",
    },
    Keybinding {
        keys: "?",
        description: "Toggle help",
    },
];

pub const ARCHIVE: &[Keybinding] = &[
    Keybinding {
        keys: "j / ↓",
        description: "Select next task",
    },
    Keybinding {
        keys: "k / ↑",
        description: "Select previous task",
    },
    Keybinding {
        keys: "Enter",
        description: "View task detail",
    },
    Keybinding {
        keys: "r",
        description: "Restore task to Inbox",
    },
    Keybinding {
        keys: "d",
        description: "Delete permanently (press d again to confirm)",
    },
    Keybinding {
        keys: "Esc",
        description: "Back to board",
    },
    Keybinding {
        keys: "?",
        description: "Toggle help",
    },
];
