# Patterns — xtui

*Last reviewed: 2026-06-08. Next review: after 5+ commits or 3+ sessions.*

---

### Convention: CommandSource Port Trait

**Frequency**: 8/8 source impls
**Confidence**: high (all sources follow this shape)
**Description**: All command sources implement `CommandSource: Send + Sync` with
`name() -> &str` and `discover(project: &Path) -> Result<Vec<SourceCommand>>`.
Empty vec = source not applicable. `all_sources()` returns them in tab order.
**Evidence**:
1. `src/source.rs:16-19` — trait definition
2. `src/source.rs:22-33` — `all_sources()` factory
**Recommended actions**: Any new source must implement this trait. Do not add
source-specific logic outside the source's `discover()` impl.

---

### Convention: Module-per-concern Layout

**Frequency**: 10/10 source modules
**Confidence**: high
**Description**: Each module owns exactly one concern. No cross-module state mutation.
**Evidence**:
- `app.rs` — state + event loop
- `source.rs` — source trait + impls
- `discover.rs` — xtask source parser
- `runner.rs` — child process management
- `ui.rs` — ratatui rendering (pure, no mutation)
- `pipeline.rs` — state machine only
- `search.rs` — search/match tracking only
- `history.rs` — persistence only
- `status.rs` — git status only
- `registry.rs` — project discovery only
**Recommended actions**: Maintain this separation. When a module starts importing
from 3+ others, consider whether it has grown a second concern.

---

### Convention: Proptest for Parser Robustness

**Frequency**: 3/3 parser modules with proptests
**Confidence**: high (consistent across discover, pipeline, search)
**Description**: Parser and state-machine functions get proptest fuzz coverage
alongside unit tests.
**Evidence**:
1. `src/discover.rs` — proptest for `parse_help_output` and `parse_source`
2. `src/pipeline.rs` — proptest for state machine transitions
3. `src/search.rs` — proptest for match cycling
**Recommended actions**: Apply same pattern to any new parser or state machine.

---

### Convention: Parallel Subagent Testing Wave

**Frequency**: 1/1 observed testing sessions (2026-06-08)
**Confidence**: low (single instance)
**Description**: Testing tasks (t22–t29) were dispatched in parallel as a subagent
batch. All completed with identical ~205s duration, confirming true parallelism.
**Evidence**:
- `.ctx/godmode/sessions/2026-06-08.jsonl` lines 27–42 — simultaneous start times,
  matching durations
**Recommended actions**: Use parallel subagent dispatch for independent per-module
test tasks. Verify all subagent commits landed before marking wave done.

---

### API Shape: anyhow::Result Throughout

**Frequency**: all fallible functions
**Confidence**: high
**Description**: All fallible functions use `anyhow::Result<T>` — no custom error types.
**Evidence**: `discover()`, `spawn_command()`, `App::run()`, all source impls
**Recommended actions**: Consider `thiserror` only if error variants need matching
at call sites (thiserror is already a transitive dep).

---

### Convention: No External Clipboard Dependency

**Frequency**: 1/1 clipboard operations
**Confidence**: low (single usage)
**Description**: Uses OSC52 escape sequence with a custom base64 encoder instead
of a platform clipboard crate.
**Evidence**:
1. `src/app.rs` — `base64_encode()` hand-rolled, `copy_output()` writes OSC52
**Recommended actions**: Works for terminal-native usage. Revisit if non-terminal
contexts are needed.
