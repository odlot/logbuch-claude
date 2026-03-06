# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Logbuch — a plain CLI developer productivity tool for capturing tasks, todos, and time-boxing work with pomodoro sessions. Licensed under GPL-3.0. Written in Rust.

## Build & Run

```bash
cargo build              # debug build
cargo build --release    # release build
cargo run                # print help and exit 0
cargo run -- add "fix the bug"   # add a task
cargo run -- list                # list all tasks
cargo run -- --db /tmp/test.db list   # override DB path
```

## Test & Lint

```bash
cargo test               # run all tests
cargo test -- test_name  # run a single test
cargo fmt --all -- --check  # check formatting
cargo clippy --all-targets --all-features  # lint
```

## Architecture

Plain CLI with clap subcommands. No TUI.

- **`src/main.rs`** — clap CLI definition (`Commands` enum) and dispatch.
- **`src/cmd/`** — Command implementations split by concern:
  - `tasks.rs` — add, list, show, done, rm, defer, edit
  - `todos.rs` — todo, check
  - `sessions.rs` — start, stop, note, status, `_notify` (hidden background process)
  - `log.rs` — log (daily/weekly)
- **`src/output.rs`** — TTY detection (`Out` struct with ANSI colour helpers; strips colour when stdout is not a terminal).
- **`src/db/`** — SQLite persistence. `migrations.rs` (schema versioning), `queries.rs` (all CRUD operations). Uses rusqlite with bundled SQLite.
- **`src/model/`** — Data structs: `Task` (with `TaskList` enum for inbox/in_progress/backlog), `Session`, `Todo`.
- **`src/config.rs`** — TOML config loading from XDG paths (`~/.config/logbuch/config.toml`). Defaults: 25min sessions.
- **`src/summary.rs`** — Markdown report generation (daily/weekly), exported to files.

### Pomodoro notification

`logbuch start <id>` records the session in the DB, then spawns a detached child process (`logbuch _notify --session-id <n> --seconds <n> --db <path>`) that sleeps, marks the session complete, fires a desktop notification via `notify-rust`, and exits. The child PID is written to `notify.pid` in the DB directory. `logbuch stop` kills the child and ends the session.

## Data Storage

- Config: `~/.config/logbuch/config.toml`
- Database: `~/.local/share/logbuch/logbuch.db` (SQLite with WAL mode)
- Reports: `~/logbuch-reports/` (configurable)

## Branching

- `develop` — integration branch, feature branches merge here
- `main` — stable releases only, PRs from develop
- Feature branches: `feature/<name>` from `develop`
- Releases: tag `v*` on main triggers cross-platform build + GitHub Release

## Release Process

**Important:** Version bumping happens in `develop` *before* the release PR — never as an auto-commit on `main`. This prevents recurring merge conflicts caused by squash-merge ancestry breaks.

1. In `develop`, run `./scripts/bump-version.sh patch` (or `minor`/`major`)
2. Commit: `git add Cargo.toml Cargo.lock && git commit -m "chore: bump version to X.Y.Z"`
3. Open PR from `develop` → `main` — use **"Create a merge commit"** (not squash)
4. After merge, the `tag-release` workflow reads the version from Cargo.toml and creates `vX.Y.Z`, triggering the release build

**Why not squash-merge?** Squash merges sever the git ancestry chain, making every subsequent release PR appear to conflict with the entire history of develop.
