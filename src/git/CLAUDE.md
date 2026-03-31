# git/

Git integration via CLI commands.

## Files

| File | Purpose |
|---|---|
| `status.rs` | `GitInfo` struct — branch, file statuses, ahead/behind counts. `GutterMark` enum for per-line diff marks. Both gathered by shelling out to `git`. |

## Functions

- `GitInfo::gather(root)` — runs `git rev-parse`, `git status --porcelain`, `git rev-list` to collect all info
- `GitInfo::diff_for_file(root, path)` — runs `git diff --unified=0` and parses hunk headers to determine added/modified/deleted lines
- `GitInfo::status_line()` — formats `main ~3 +1 ↑2 ↓0` for the statusbar

## Refresh

Called every 5 seconds from `app.check_git_refresh()` in the event loop. Updates statusbar info, file tree indicators, and gutter marks.

## Gotchas

- Uses CLI `git` not libgit2 — requires git to be installed
- `diff_for_file` parses `@@ -old,count +new,count @@` hunk headers. The counts default to 1 when omitted (single-line changes)
- Gutter marks use 0-indexed line numbers (matching ropey), but git diff uses 1-indexed
