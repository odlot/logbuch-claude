# TODO — Logbuch

## Decisions

| Topic | Decision |
|-------|----------|
| Language/Framework | Rust with ratatui + crossterm |
| Storage | SQLite via rusqlite (bundled) |
| Timer expiry | Desktop notification (notify-rust) + visual TUI indicator |
| Summaries | Export to Markdown files |
| CI/CD | GitHub Actions |
| Branching | Feature branches from `develop`, releases via PR to `main` |
| Starting version | 0.1.0 |

## Data Model

- **Task**: description, list (inbox/in-progress/backlog), position, created_at, updated_at
- **Session**: task_id, begin_at, end_at (nullable = active), duration_min (default 25), notes
- **Todo**: task_id, description, done, position, completed_at

## TUI Views

1. **Board** — three-column kanban (inbox, in-progress, backlog), vim-style hjkl navigation
2. **Task Detail** — description, todos list, session history; Tab/Shift+Tab to cycle sections
3. **Active Session** — centered MM:SS countdown, progress bar, notes input area

## Phase 1: Foundation

- [x] Create `Cargo.toml` with all dependencies (ratatui, crossterm, rusqlite, notify-rust, chrono, serde, toml, clap, dirs, anyhow)
- [x] Set up project directory structure (`src/`, `src/ui/`, `src/db/`, `src/model/`)
- [x] Implement `src/config.rs` — Config struct with defaults, TOML loading, XDG paths (~/.config/logbuch/config.toml, ~/.local/share/logbuch/logbuch.db)
- [x] Implement `src/event.rs` — Event loop thread with crossterm polling and 250ms tick timer via mpsc channel
- [x] Implement `src/db/mod.rs` — Connection init, WAL mode, foreign keys, migration runner
- [x] Implement `src/db/migrations.rs` — Initial SQL schema (task, session, todo, schema_version tables)
- [x] Implement `src/model/` — Task (with TaskList enum), Session, Todo structs
- [x] Implement `src/app.rs` — App struct with minimal state, quit handling, main loop
- [x] Implement `src/ui/mod.rs` and `src/ui/board.rs` — Three empty columns with titles
- [x] Implement `src/main.rs` — CLI parsing (clap), config loading, DB init, terminal setup/restore, run loop
- [x] **Milestone**: `cargo run` shows a three-column board, `q` quits cleanly

## Phase 2: Task CRUD + Board Navigation

- [x] Implement `db/queries.rs` — insert_task, list_tasks, delete_task, move_task, update_task_description
- [x] Add board navigation: `h`/`l` switch columns, `j`/`k` select tasks
- [x] Render tasks in columns with selection highlighting (reversed colors, `>>` marker)
- [x] Active column highlighted with distinct border color
- [x] Implement `src/ui/input.rs` — Text input widget with cursor movement
- [x] `n` — New task in active column (enters editing mode)
- [x] `d` — Delete selected task
- [x] `H`/`L` (Shift) — Move selected task to adjacent list
- [x] `Enter` — Open task detail (wired in Phase 3)
- [x] Status message bar at bottom (brief feedback, fades after 3s)
- [x] **Milestone**: data persists across restarts
## Phase 3: Task Detail View + Todos

- [x] Implement `db/queries.rs` — list_todos, insert_todo, toggle_todo, delete_todo
- [x] Implement `src/ui/task_detail.rs` — Render description, todos list, session history placeholder
- [x] `Enter` from board opens task detail; `Esc` returns to board
- [x] `Tab`/`Shift+Tab` — Cycle through Description, Todos, Sessions sections
- [x] `j`/`k` — Navigate within sections
- [x] `e` — Edit task description (editing mode)
- [x] `a` — Add new todo (editing mode)
- [x] `x` — Toggle selected todo done/not-done
- [x] `D` — Delete selected todo
- [x] **Milestone**: Full task detail view with working todos
## Phase 4: Pomodoro Sessions

- [x] Implement `db/queries.rs` — start_session, end_session, append_session_notes, get_active_session, list_sessions, close_orphaned_sessions
- [x] Implement `src/ui/session_view.rs` — Centered MM:SS timer, Gauge progress bar, notes area
- [x] `s` from task detail starts a session (configurable duration, default 25 min)
- [x] Timer uses Instant-based elapsed calculation (not tick counting)
- [x] On timer expiry: desktop notification via notify-rust (Urgency::Critical, Timeout::Never) + visual indicator (bar turns green, "DONE")
- [x] `Enter` — Focus note input / submit note line
- [x] `Esc` — End session early (records actual elapsed time)
- [x] Notes appended to session in DB with newline separators
- [x] Status bar shows active session indicator from all views ("Session active: MM:SS remaining")
- [x] Session history renders in task detail view
- [x] Close orphaned sessions on startup (crash recovery)
- [x] `q` blocked during active session
- [x] **Milestone**: Full pomodoro flow
## Phase 5: Summary Reports

- [x] Add `completed_at` column to todo table (included in migration 001)
- [x] Implement `db/queries.rs` — sessions_in_range, todos_completed_in_range
- [x] Implement `src/summary.rs` — Daily and weekly Markdown generation
- [x] Daily: sessions and completed todos grouped by task for a given date
- [x] Weekly: same but Monday–Sunday range, with ISO week number
- [x] File naming: `logbuch-daily-YYYY-MM-DD.md`, `logbuch-weekly-YYYY-WNN.md`
- [x] Export to configurable directory (default ~/logbuch-reports/)
- [x] `r d` / `r w` keybindings from board view
- [x] `--summary daily|weekly` CLI flag for headless generation
- [x] Status message showing exported file path
- [x] **Milestone**: Reports generate correctly
## Phase 6: Polish & CI/CD

- [x] `.github/workflows/ci.yml` — fmt, clippy, build, test on push/PR to main and develop
- [x] `.github/workflows/release.yml` — Cross-platform builds (Linux, macOS, Windows) on version tags
- [ ] Create `develop` branch from `main`, set as default branch
- [ ] Branch protection: main requires PR + CI pass, develop requires CI pass
- [x] `?` key — Help overlay showing keybindings for current view
- [x] Handle edge cases: empty lists, long descriptions (truncation), terminal resize
- [ ] Replace all `unwrap()` with proper error handling
- [x] Unit tests for `db/queries.rs` (in-memory SQLite) — `tests/db_tests.rs`
- [ ] Unit tests for `summary.rs`
- [ ] Integration test: create app, simulate key sequence, verify state
- [ ] `README.md` — Installation, usage, keybinding reference
- [x] Update `CLAUDE.md` with build/test/lint commands
- [ ] **Milestone**: v0.1.0 release

## Keybindings Reference

### Global
| Key | Action |
|-----|--------|
| `q` | Quit (blocked during active session) |
| `?` | Toggle help overlay |
| `Esc` | Back / Cancel input |

### Board View
| Key | Action |
|-----|--------|
| `h` / Left | Focus left column |
| `l` / Right | Focus right column |
| `j` / Down | Select next task |
| `k` / Up | Select previous task |
| `Enter` | Open task detail |
| `n` | New task in current column |
| `d` | Delete selected task |
| `H` (Shift) | Move task to left column |
| `L` (Shift) | Move task to right column |
| `r d` | Generate daily report |
| `r w` | Generate weekly report |

### Task Detail View
| Key | Action |
|-----|--------|
| `Esc` | Back to board |
| `Tab` / `Shift+Tab` | Cycle sections |
| `j` / `k` | Navigate within section |
| `e` | Edit description |
| `a` | Add todo |
| `x` | Toggle todo |
| `D` | Delete todo |
| `s` | Start session |

### Active Session View
| Key | Action |
|-----|--------|
| `Esc` | End session early |
| `Enter` | Submit note / focus note input |
| Text | Type note content |

### Editing Mode
| Key | Action |
|-----|--------|
| `Enter` | Confirm input |
| `Esc` | Cancel input |
| `Backspace` | Delete character |
| Left / Right | Move cursor |
