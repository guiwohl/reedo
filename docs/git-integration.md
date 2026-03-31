# Git Integration

## Status Bar

The bottom status bar shows git info on the left:

```
[NORMAL]  42/520  main ~3 +1 ↑2 ↓0  app.rs [+]
                  ^^^^ ^^ ^^ ^^ ^^
                  |    |  |  |  └ commits behind upstream
                  |    |  |  └ commits ahead of upstream
                  |    |  └ staged files count
                  |    └ changed (unstaged + untracked) files count
                  └ current branch name
```

## File Tree Indicators

In the file explorer (Ctrl+E), files show their git status:

| Indicator | Meaning |
|---|---|
| `M` | Modified |
| `A` | Added / staged |
| `D` | Deleted |
| `U` | Unmerged (conflict) |
| `?` | Untracked |

## Gutter Marks

The editor gutter (left of line numbers) shows per-line diff status compared to the git index:

| Mark | Color | Meaning |
|---|---|---|
| `│` | Green | Added line |
| `│` | Yellow | Modified line |
| `▸` | Red | Deleted line (marks the line after deletion) |

## Refresh

Git info refreshes automatically every 5 seconds. This updates:
- Status bar (branch, counts)
- File tree indicators
- Gutter marks

## Implementation

Uses `git` CLI commands (not libgit2):
- `git rev-parse --abbrev-ref HEAD` — branch name
- `git status --porcelain` — file statuses
- `git rev-list --left-right --count HEAD...@{upstream}` — ahead/behind
- `git diff --unified=0 --no-color -- <file>` — per-line diff for gutter marks
