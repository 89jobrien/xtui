# Product Context: xtui

## Why It Exists

Every project accumulates runnable commands across multiple tools — xtask, just,
make, npm scripts, mise tasks, cargo bins. xtui provides a single visual menu to
browse, select, and run any of them with live output, without remembering command
names or switching tools.

## UX Principles

- Vim-style navigation (j/k, Enter, Esc, q)
- Tabbed sources — each tool gets its own tab, empty tabs are hidden
- Auto-scroll output to bottom; output pane focusable for manual scroll
- ANSI color preservation from child process output
- OSC52 clipboard copy for terminal output sharing
- Flash messages for transient feedback (2s timeout)
- Args-input mode: modal prompt before run, no shell quoting needed
- Pipeline mode: chain all tab commands sequentially, stop on failure
- Dep view (D key): full-screen panel showing direct deps + crates.io latest,
  color-coded by fetch state (Loading/Ready/Error), j/k scroll, local dep detection
