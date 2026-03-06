# Logbuch CLI — Plan

## Goal

A developer productivity tool for capturing tasks, breaking them into todos,
and time-boxing work with pomodoro sessions. Plain CLI — no TUI, no wizard,
no interactive mode. Fast to type, scriptable, stays out of the way.

---

## Command Interface

```
logbuch add <description>                       add a task to inbox
logbuch list                                    show all tasks (alias: ls)
logbuch show <id>                               full task detail (todos + sessions)
logbuch done <id>                               mark task complete and remove it
logbuch rm <id>                                 delete a task (prompts unless --yes)
logbuch defer <id>                              move task to backlog
logbuch edit <id> <description>                 rename a task
logbuch todo <task-id> <description>            add a todo to a task
logbuch check <task-id> <todo-id>               toggle a todo done/undone
logbuch edit <task-id> <todo-id> <description>  rename a todo
logbuch start <id> [--min <n>]                  start a pomodoro (default 45min)
logbuch stop                                    cancel the running session
logbuch note <text>                             attach timestamped note to active session (alias: n)
logbuch status                                  show running session and time remaining
logbuch log [--week]                            daily summary (--week for this week)
```

`logbuch` with no arguments prints the command list and exits 0.
`logbuch help <command>` and `logbuch <command> --help` both work.

---

## Output Behaviour

- **Quiet on success** — one confirmation line per action, nothing more
- **Loud on errors** — clear human message to stderr, non-zero exit code
- **TTY detection** — strip colour and decorations when stdout is not a TTY
  so piping to grep/awk/scripts works without parsing ANSI codes
- **Never truncate** — show full descriptions; let the terminal wrap

---

## Exit Codes

- `0` success
- `1` general error (task not found, session already running, etc.)
- `logbuch status` specifically exits `0` if a session is running, `1` if not,
  enabling shell prompt widgets and scripting like `logbuch status || logbuch start 4`

---

## Destructive Actions

- `logbuch rm <id>` prompts "Delete 'fix login redirect'? [y/N]" by default
- `--yes` skips the prompt for scripting: `logbuch rm 4 --yes`
- `logbuch done <id>` on an already-complete task: no-op with message, not an error

---

## Pomodoro Notification

- `logbuch start <id>` records the session in the DB, then spawns a detached
  background process (`logbuch _notify --session-id <n> --seconds <n>`) that:
  1. Sleeps for the session duration
  2. Fires a desktop notification via `notify-rust`
  3. Marks the session complete in the DB
  4. Exits
- The foreground process returns to the shell prompt immediately
- The notifier PID is stored (DB or lockfile) so `logbuch stop` can kill it
- `logbuch stop` kills the notifier, marks the session cancelled in the DB
- Known limitation v1: wall-clock sleep does not account for system suspend

---

## Notes

- `logbuch note <text>` errors with a clear message if no session is running
- Notes are stored on the session with a timestamp
- Multiple notes per session, shown chronologically under their session in `show`

---

## Todos

- `logbuch todo <task-id> <description>` appends a todo; referenced as `<task-id> <todo-id>`
- `logbuch check <task-id> <todo-id>` toggles done/undone
- `list` shows todos inline for In Progress tasks only (keeps output short for active work)
- `show` always displays all todos regardless of task state

---

## `list` Output Format

```
  Inbox
  #1  write unit tests
  #3  update README

  In Progress
  #4  fix login redirect              ▶ 18:42 remaining
      [ ] 1  investigate redirect chain
      [x] 2  reproduce with test case
      [ ] 3  patch and verify

  Backlog
  #2  migrate to postgres
```

Empty sections are omitted entirely.

---

## `show` Output Format

```
  #4  fix login redirect    In Progress
  ──────────────────────────────────────

  Todos
  [ ] 1  investigate redirect chain
  [x] 2  reproduce with test case
  [ ] 3  patch and verify

  Sessions
  2026-03-05 09:00  25m
    14:03  checked the middleware stack
    14:17  the issue is in the OAuth callback
```

---

## `log` Output Format

```
  Thursday 5 Mar 2026

  Sessions
  ─────────────────────────────────────────
  write unit tests     09:00–09:25   25m
  fix login redirect   10:00–10:45   45m
  ─────────────────────────────────────────
  Total                              70m

  Completed todos
  [x] reproduce with test case       fix login redirect
```

---

## Configuration

- Config file at `~/.config/logbuch/config.toml` — no wizard, documented defaults
- DB at `~/.local/share/logbuch/logbuch.db`
- Env var overrides: `LOGBUCH_DB_PATH`, `LOGBUCH_SESSION_DURATION`
- `--config <path>` and `--db <path>` flags available on every command

---

## What to Keep from the Existing Codebase

- `src/db/migrations.rs` — schema is sound; remove the `done` list value since
  tasks are deleted on `done`, not archived
- `src/db/queries.rs` — keep all query functions; remove archive/triage/purge
- `src/config.rs` — keep as-is
- `src/summary.rs` — keep daily/weekly report generation
- `notify-rust` dependency — already present
- `tests/` — keep all tests, extend for new CLI behaviour

## What to Throw Away

- `src/app.rs` — entire TUI state machine
- `src/ui/` — all rendering code
- `src/event.rs` — crossterm event loop
- `src/wizard.rs` — first-run wizard
- `ratatui` and `crossterm` dependencies

---

## Implementation Order

1. [ ] Strip the TUI — delete `src/app.rs`, `src/ui/`, `src/event.rs`,
       `src/wizard.rs`; remove `ratatui`/`crossterm` from `Cargo.toml`;
       update `src/lib.rs` and `src/main.rs`
2. [ ] Scaffold `clap` subcommands; `logbuch` with no args prints usage and exits 0
3. [ ] `add`, `list`, `show`, `done`, `rm` (with confirm prompt + `--yes`), `defer`, `edit`
4. [ ] `todo`, `check`, `edit <task> <todo>`
5. [ ] `start` + detached notifier process + `stop` with PID tracking
6. [ ] `note` (errors if no session running) + `status` with correct exit codes
7. [ ] `log` (daily), `log --week`
8. [ ] TTY detection — strip colour when stdout is not a TTY
9. [ ] `--config` and `--db` flags wired through to every subcommand
10. [ ] Shell completions (bash, zsh, fish) — clap can generate these
11. [ ] Review and extend tests

---

## Out of Scope for v1

- `--dry-run` flag
- Editing session notes after the fact
- Recurring tasks
- Tags or labels
- Export formats beyond Markdown
- Man page
