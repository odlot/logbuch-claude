# Logbuch

A plain CLI for developer task management and focused work sessions.

Tasks have a description, a checklist of todos, and a history of timed work sessions. The pomodoro timer runs in the background — you get a desktop notification when it ends and your shell prompt is never blocked.

## Installation

```bash
git clone https://github.com/odlot/logbuch
cd logbuch
cargo build --release
# Binary at target/release/logbuch — copy it anywhere on your $PATH
```

## Commands

```
logbuch add <description>                       add a task to inbox
logbuch list / ls                               show all tasks
logbuch show <id>                               full task detail (todos + sessions)
logbuch done <id>                               mark task complete and remove it
logbuch rm <id> [--yes]                         delete a task (prompts unless --yes)
logbuch defer <id>                              move task to backlog
logbuch edit <id> <description>                 rename a task
logbuch edit <task-id> <todo-id> <description>  rename a todo

logbuch todo <task-id> <description>            add a todo to a task
logbuch check <task-id> <todo-id>               toggle a todo done/undone

logbuch start <id> [--min <n>]                  start a session (default 45 min)
logbuch stop                                    cancel the running session
logbuch resume [--min <n>]                      new session on the last worked task
logbuch note <text> / n <text>                  attach a timestamped note to active session
logbuch status                                  show running session + time remaining

logbuch log                                     today's activity
logbuch log --week                              this week (Mon–Sun)
logbuch log <yyyy-mm-dd>                        specific date
logbuch log <yyyy-mm-dd> <yyyy-mm-dd>           date range
```

`logbuch` with no arguments prints the command list and exits 0.
`logbuch <command> --help` for per-command help.

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | success |
| `1` | error (task not found, bad arguments, etc.) |

`logbuch status` exits `0` if a session is running, `1` if not — making it
scriptable:

```bash
logbuch status || logbuch start 4
```

## Pomodoro sessions

`logbuch start <id>` records the session in the database and spawns a detached
background process that sleeps for the session duration, marks it complete in
the database, and fires a desktop notification. Your shell prompt returns
immediately.

`logbuch stop` kills the background process and marks the session ended.

`logbuch resume` starts a new session on whichever task you worked on most
recently — useful for picking up where you left off after a break.

## `list` output

```
  Inbox
  #1    write unit tests
  #3    update README

  In Progress
  #4    fix login redirect              ▶ 18:42 remaining
        [ ] 1  investigate redirect chain
        [x] 2  reproduce with test case
        [ ] 3  patch and verify

  Backlog
  #2    migrate to postgres
```

In-progress tasks show their active session countdown and todos inline.
Empty sections are omitted.

## `show` output

```
  #4    fix login redirect    In Progress
  ────────────────────────────────────────

  Todos
  [ ] 1  investigate redirect chain
  [x] 2  reproduce with test case
  [ ] 3  patch and verify

  Sessions
  2026-03-05 09:00  45m
    14:03  checked the middleware stack
    14:17  the issue is in the OAuth callback
```

## `log` output

```
  Friday 6 Mar 2026
  ─────────────────────────────────────────────
  fix login redirect           09:00–09:45   45m
  write unit tests             10:00–10:45   45m
  ─────────────────────────────────────────────
  Total                                      90m

  Completed todos
  [x] reproduce with test case       fix login redirect
```

Weekend days are omitted from range reports unless they have activity.

## Configuration

Settings are resolved in priority order (highest wins):

1. CLI flags — `--db`, `--config`
2. Environment variables — `LOGBUCH_DB_PATH`, `LOGBUCH_SESSION_DURATION`
3. Config file — `~/.config/logbuch/config.toml`
4. Built-in defaults

### Config file

```toml
# ~/.config/logbuch/config.toml

# Duration of a new session in minutes (default: 45)
# session_duration_min = 45

# SQLite database path (default: ~/.local/share/logbuch/logbuch.db)
# db_path = "~/.local/share/logbuch/logbuch.db"
```

Paths support `~` expansion.

### Environment variables

```bash
export LOGBUCH_DB_PATH=~/.local/share/logbuch/work.db
export LOGBUCH_SESSION_DURATION=60
```

## Data

| Path | Contents |
|------|----------|
| `~/.local/share/logbuch/logbuch.db` | SQLite database (tasks, todos, sessions) |
| `~/.local/share/logbuch/notify.pid` | PID of the running notifier (if a session is active) |
| `~/.config/logbuch/config.toml` | User config |

## Development

```bash
cargo build
cargo test
cargo fmt --all
cargo clippy --all-targets --all-features
```

### Pre-commit hook

```bash
git config core.hooksPath .githooks
```

Runs `cargo fmt --check` and `cargo clippy` before every commit.

## License

GPL-3.0 — see [LICENSE](LICENSE).
