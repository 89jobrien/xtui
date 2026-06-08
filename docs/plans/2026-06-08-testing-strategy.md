# Testing Strategy: Pure Logic + Integration

**Date:** 2026-06-08
**Status:** Done (v0.1.0)
**Scope:** xtui crate only

## Goal

Deepen test coverage across two axes:

1. **Pure logic** — edge cases, error paths, and property tests for
   existing modules (discover, pipeline, search, history, registry, app)
2. **Integration** — end-to-end discovery tests using synthetic fixture
   projects, plus `#[ignore]` tests against real repos

## Architecture

### New files

| Path                   | Purpose                            |
| ---------------------- | ---------------------------------- |
| `tests/common/mod.rs`  | `ProjectFixture` builder + cleanup |
| `tests/integration.rs` | Cross-module integration tests     |
| `tests/sources.rs`     | Per-source integration tests       |

### Fixture builder (`tests/common/mod.rs`)

`ProjectFixture` creates a temp dir and exposes chainable builder
methods for each source type:

```rust
let fix = ProjectFixture::new()
    .with_cargo_toml("[package]\nname=\"test\"")
    .with_justfile("build:\n  echo ok")
    .with_package_json(r#"{"scripts":{"dev":"echo"}}"#)
    .with_makefile("build:\n\techo")
    .with_mise_toml("[tasks.lint]\nrun = \"echo\"")
    .with_nu_script("lint", "# lint")
    .with_xtask_main("fn main() { ... }");
```

`Drop` impl removes the temp directory. `fix.path()` returns `&Path`.

### Inline unit test additions (pure logic)

**discover.rs**

- Malformed fn blocks / empty match arms
- Descriptions with special chars (`"`, `\n`)

**pipeline.rs**

- `advance` called on Pending/Done/Failed states (no-op)
- Single-step pipeline success and failure

**search.rs**

- Empty query, empty lines, single-line input
- Proptest: `find_matches` never panics on arbitrary input

**history.rs**

- `prune_logs` at boundary (100 files, 101 files)
- `save_entry` with special chars in project name
- `days_to_ymd` known dates: epoch (0), 2024-02-29 (leap), 2026-06-08

**registry.rs**

- `scan_directory` on nonexistent path

**app.rs**

- `base64_encode` known vectors: empty, 1-byte, 2-byte, 3-byte,
  RFC 4648 test vectors ("f", "fo", "foo", "foob", "fooba", "foobar")

### Integration tests (`tests/integration.rs`)

- `all_sources_discover_fixture` — fixture with all 6 source types,
  assert each tab discovered
- `mixed_project_some_sources_missing` — Cargo.toml + Justfile only,
  assert exactly 2 tabs
- `empty_project_no_tabs` — no markers, assert 0 tabs

### Source-specific integration tests (`tests/sources.rs`)

- `just_source_with_descriptions` — Justfile with `# comment`
  descriptions
- `cargo_source_with_bin_targets` — Cargo.toml with `[[bin]]` entries
- `xtask_source_nested_subcommands` — multi-level xtask main.rs

### Real-repo `#[ignore]` tests (`tests/integration.rs`)

- `real_repo_xtui` — scan `CARGO_MANIFEST_DIR`, assert cargo + xtask
  tabs
- `real_repo_minibox` — scan `~/dev/minibox` if it exists, assert
  xtask tab

## Tech Decisions

- **Fixture-based over mocks**: `CommandSource` is a trait but real
  implementations are simple filesystem reads. Synthetic temp dirs
  are deterministic and test the real code path.
- **Proptest additions**: extend to `search::find_matches` and
  `pipeline::advance` — both are pure functions over bounded state.
- **`#[ignore]` for real repos**: keeps `cargo test` fast by default;
  run with `cargo test -- --ignored` for local validation.
- **No new dependencies**: `proptest` is already in dev-dependencies.

## Out of Scope

- TUI rendering / snapshot tests for `ui::draw`
- Async runner tests beyond existing `spawn_command` coverage
- `status.rs` git integration (requires repo setup)
- CI config changes
- Mocking `CommandSource` trait
