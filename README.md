# Logbuch

A keyboard-driven TUI for task management and focused work sessions (Pomodoro-style). Tasks live in a three-column kanban board; each task has a description, a checklist of todos, and a history of timed work sessions.

## Installation

**Prerequisites:** Rust stable toolchain (`rustup`).

```bash
git clone https://github.com/odlot/logbuch
cd logbuch
cargo build --release
# Binary at target/release/logbuch — copy it anywhere on your $PATH
```

## Usage

```bash
logbuch                              # open the TUI
logbuch --summary daily              # generate daily report and exit
logbuch --summary weekly             # generate weekly report and exit
logbuch --show-config                # print effective configuration and exit
logbuch --config /path/to/cfg.toml   # use a custom config file
logbuch --db-path /tmp/test.db       # override database location
logbuch --summary-dir ~/reports      # override report output directory
logbuch --session-duration 50        # override default session length (minutes)
```

## TUI Overview

Logbuch uses a modal interface similar to Vim. There are three main views:

```
┌─────────────┬──────────────┬──────────────┐
│    Inbox    │  In Progress │   Backlog    │
│─────────────│──────────────│──────────────│
│ >> Task A   │  Task C      │  Task E      │
│    Task B   │  Task D      │              │
└─────────────┴──────────────┴──────────────┘
  [ n:new  d:delete  H/L:move  Enter:open  ?:help ]
```

### Board View

The starting view shows all tasks across three columns.

| Key | Action |
|-----|--------|
| `h` / `←` | Focus left column |
| `l` / `→` | Focus right column |
| `j` / `↓` | Select next task |
| `k` / `↑` | Select previous task |
| `Enter` | Open task detail |
| `n` | New task (type description, Enter to confirm) |
| `d` | Delete selected task (press `d` again to confirm) |
| `H` | Move task left (Shift+h) |
| `L` | Move task right (Shift+l) |
| `r d` | Generate daily summary report |
| `r w` | Generate weekly summary report |
| `/` | Open fuzzy search overlay |
| `q` | Quit |
| `?` | Toggle help |

### Task Detail View

Opens when you press `Enter` on a task. Three sections — Description, Todos, Sessions — cycle with `Tab`.

```
┌─ Task: Fix login bug ────────────────────────────────┐
│ List: In Progress | Created: 2026-03-01 09:00        │
├─ Description ────────────────────────────────────────┤
│ Investigate why JWT refresh fails after 1h.          │
├─ Todos (3) ──────────────────────────────────────────┤
│ >> [ ] Reproduce with curl                           │
│    [x] Read refresh token docs                       │
│    [ ] Write regression test                         │
├─ Sessions (2, 0h 50m) ───────────────────────────────┤
│ >> 2026-03-01 09:00 - 09:25 (25m) "tried curl"      │
│    2026-03-01 10:00 - 10:25 (25m)                    │
├─ Session Notes ──────────────────────────────────────┤
│ tried curl -X POST /auth/refresh — returns 401 after │
│ token expiry.                                        │
└──────────────────────────────────────────────────────┘
```

#### Navigation

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous section |
| `j` / `↓` | Select next item in active section |
| `k` / `↑` | Select previous item in active section |
| `Esc` | Back to board |
| `?` | Toggle help |

#### Description section

| Key | Action |
|-----|--------|
| `e` | Edit description (confirm with Enter, cancel with Esc) |

#### Todos section

| Key | Action |
|-----|--------|
| `a` | Add todo |
| `x` | Toggle todo done/undone |
| `e` | Edit selected todo description |
| `D` | Delete selected todo (press `D` again to confirm) |
| `J` | Move todo down (Shift+j) |
| `K` | Move todo up (Shift+k) |

#### Sessions section

| Key | Action |
|-----|--------|
| `s` | Start a new session (prompts for duration in minutes) |
| `D` | Delete selected session (press `D` again to confirm) |

The **Session Notes** panel below the list always shows the full notes for the selected session.

### Session View

When a session is active, Logbuch switches to a full-screen timer view.

```
┌─ Session: Fix login bug ────────────────────────────┐
│                                                     │
│         ████████████████░░░░░░░░  18:43 left        │
│                                                     │
│  Notes                                              │
│  > curl reproduces it — token_exp claim missing     │
│                                                     │
│  [ Enter:submit note   Esc:end session ]            │
└─────────────────────────────────────────────────────┘
```

| Key | Action |
|-----|--------|
| Any character | Start typing a note line |
| `Enter` | Submit current note line |
| `Esc` | End the session early and return to task detail |

Notes are appended to the session one line at a time. When the timer reaches zero, the session ends automatically and a desktop notification fires.

### Fuzzy Search Overlay

Press `/` from any view to open the search overlay.

```
┌─ Search tasks ───────────────────────────────┐
│ / log_                                       │
│─────────────────────────────────────────────│
│ >> [Inbox]       Login page flicker          │
│    [In Progress] Fix login bug               │
│    [Backlog]     Log rotation setup          │
└──────────────────────────────────────────────┘
```

| Key | Action |
|-----|--------|
| Type | Filter tasks (subsequence / fuzzy match) |
| `↑` / `↓` | Navigate results |
| `Enter` | Open highlighted task detail |
| `Esc` | Dismiss |

## Configuration

Settings are resolved in priority order (highest wins):

1. **CLI flags** — `--db-path`, `--summary-dir`, `--session-duration`
2. **Environment variables** — `LOGBUCH_DB_PATH`, `LOGBUCH_SUMMARY_DIR`, `LOGBUCH_SESSION_DURATION`
3. **Config file** — `~/.config/logbuch/config.toml`
4. **Built-in defaults**

### Config file

Created automatically with commented-out defaults on first run. Edit any value:

```toml
# ~/.config/logbuch/config.toml

# Duration of a new session in minutes (default: 25)
# session_duration_min = 25

# Where summary reports are written (default: ~/logbuch-reports)
# summary_export_dir = "~/logbuch-reports"

# SQLite database location (default: ~/.local/share/logbuch/logbuch.db)
# db_path = "~/.local/share/logbuch/logbuch.db"
```

Paths support `~` expansion. Use a custom config file with `--config /path/to/cfg.toml`.

### Environment variables

```bash
export LOGBUCH_DB_PATH=~/.local/share/logbuch/logbuch.db
export LOGBUCH_SUMMARY_DIR=~/logbuch-reports
export LOGBUCH_SESSION_DURATION=25   # minutes
```

Useful for switching between databases without editing the config file (e.g. work vs personal).

### Inspect effective configuration

```bash
logbuch --show-config
```

Prints the config file path and every resolved value so you can see exactly which source won.

## Data

| Path | Contents |
|------|----------|
| `~/.local/share/logbuch/logbuch.db` | SQLite database (tasks, todos, sessions) |
| `~/.config/logbuch/config.toml` | User config |
| `~/logbuch-reports/` | Generated Markdown reports |

## Summary Reports

`r d` (daily) and `r w` (weekly) generate Markdown files in `summary_export_dir`. Each report lists completed todos and finished work sessions grouped by task, with total time tracked.

## Development

```bash
cargo build           # debug build
cargo test            # run tests
cargo fmt --all       # format
cargo clippy --all-targets --all-features  # lint
```

### Pre-commit hook

The repository ships a pre-commit hook that runs `cargo fmt --check` and `cargo clippy` before every commit, matching the CI checks. Activate it once after cloning:

```bash
git config core.hooksPath .githooks
```

## License

GPL-3.0 — see [LICENSE](LICENSE).
