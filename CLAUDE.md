# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Logbuch — a keyboard-driven TUI task-management and productivity app with pomodoro sessions. Licensed under GPL-3.0. Written in Rust.

## Build & Run

```bash
cargo build              # debug build
cargo build --release    # release build
cargo run                # run the TUI
cargo run -- --summary daily   # generate daily report without TUI
cargo run -- --summary weekly  # generate weekly report without TUI
cargo run -- --config /path/to/config.toml  # custom config path
```

## Test & Lint

```bash
cargo test               # run all tests
cargo test -- test_name  # run a single test
cargo fmt --all -- --check  # check formatting
cargo clippy --all-targets --all-features  # lint
```

## Architecture

The app follows a modified Elm Architecture (Model-Update-View):

- **`src/app.rs`** — Central state machine (`App` struct). Holds all state, processes key events, dispatches DB mutations. The largest and most important file.
- **`src/event.rs`** — Crossterm event loop in a background thread. Sends `Key`, `Tick` (250ms), and `Resize` events via mpsc channel.
- **`src/ui/`** — View layer. `board.rs` (3-column kanban), `task_detail.rs` (description/todos/sessions), `session_view.rs` (timer/notes). The `mod.rs` handles view dispatch and help overlay.
- **`src/db/`** — SQLite persistence. `migrations.rs` (schema versioning), `queries.rs` (all CRUD operations). Uses rusqlite with bundled SQLite.
- **`src/model/`** — Data structs: `Task` (with `TaskList` enum for inbox/in_progress/backlog), `Session`, `Todo`.
- **`src/config.rs`** — TOML config loading from XDG paths (`~/.config/logbuch/config.toml`). Defaults: 25min sessions.
- **`src/summary.rs`** — Markdown report generation (daily/weekly), exported to files.

## Data Storage

- Config: `~/.config/logbuch/config.toml`
- Database: `~/.local/share/logbuch/logbuch.db` (SQLite with WAL mode)
- Reports: `~/logbuch-reports/` (configurable)

## Branching

- `develop` — integration branch, feature branches merge here
- `main` — stable releases only, PRs from develop
- Feature branches: `feature/<name>` from `develop`
- Releases: tag `v*` on main triggers cross-platform build + GitHub Release
