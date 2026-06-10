# Active Context: xtui

## Current Focus

Branch: `develop`. v0.4.10. Dev-state session metadata system complete. No active
feature development.

## Recently Completed (since last memory-bank update)

- `.github/.DS_Store` removed from git index (`634c72a`)
- `xbook/**` added to `rail.toml` infrastructure paths — no longer triggers build+test
- `xtask` version set to `0.0.0` (internal tool, not versioned)
- pre-push hook: rc suffix stripping, proper patch bump pipeline
- `scripts/dev-state.nu` — generates/verifies `.ctx/dev-state.json`
- `.claude/settings.json` — SessionStart hook runs `cargo xtask dev-state`
- `cargo xtask dev-state` and `cargo xtask dev-state --verify` commands
- `session_id` derived as `{project}-{pid}-{YYMMDD.HHMMSS}` (no external input)
- `session_hash` = sha256 of `session_id + workspace_root + hostname + username`
- pre-push gate: refresh + verify dev-state before allowing push
- `crate_versions` map in dev-state (from `cargo metadata --no-deps`)
- `mtime` field for manual-edit detection (nanosecond epoch, 2s tolerance)
- `scripts/session-init.nu` disabled (`.disabled` suffix) — superseded by derived session_id
- README updated with Dev Tasks section and new project structure entries

## Recent Decisions

- `ratatui 0.30` (maintained fork of tui-rs)
- `CommandSource` trait as port — each source is a separate impl
- OSC52 for clipboard instead of platform-specific crate
- Custom base64 to avoid adding a dependency for one function
- depview uses hexagonal ports (MetadataCache, MetadataFetcher) for testability
- Normal deps only in dep view (not dev/build deps) — krates filter
- CI uses `cargo rail run --profile ci --merge-base` — no explicit profile block needed
- dev-state: session_id is fully derived (no Claude API dependency)
- dev-state: pre-push hook refreshes BEFORE gate check, then again AFTER amend

## Open Questions / Gaps

- `registry.rs` still dead code — project picker not wired to UI
- No config file support
- No multiple workspace support

## Dirty files (session 2026-06-10)

- `xbook/ci-workflow.md` (untracked, uncommitted)
