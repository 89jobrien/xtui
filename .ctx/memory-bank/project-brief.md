# Project Brief: xtui

## What

A TUI dashboard for discovering and running project commands. Supports 8 sources
(xtask, cargo, just, nu-script, npm, make, mise, cargo-bin), organized into tabs.
Runs selected commands with live streaming output and ANSI color support.
Also shows a dep view (D key) with direct dependency metadata from crates.io.

## Who

Joseph O'Brien (89jobrien). Solo project.

## Current version: 0.4.1

## Done Criteria (v0.4.1 — complete)

- Discovers commands from 8 sources via `CommandSource` trait
- Runs selected commands with live stdout/stderr streaming
- Renders ANSI-colored output in TUI
- Tabbed source view with per-source command lists
- Args-input mode for passing arguments before running
- Pipeline mode: run all commands in a tab sequentially
- Output search with incremental match cycling
- History: last 50 runs per project, output logs persisted to `~/.config/xtui/`
- Git status display in status bar
- Cancel (Esc), refresh (r), clipboard copy (c, OSC52), quit (q)
- Dep view (D key): direct deps with crates.io latest version, TTL cache (redb), local dep detection
- Installable via `cargo xtask install` or `cargo install --path .`
- CI: fmt + clippy + nextest on every push/PR to main
- Release: patch auto-bump on push, minor/major via `cargo rail release run xtui`

## Repo

github.com/89jobrien/xtui
