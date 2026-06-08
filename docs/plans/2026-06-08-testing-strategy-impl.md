# Implementation Plan: Testing Strategy

**Date:** 2026-06-08
**Design:** `docs/plans/2026-06-08-testing-strategy.md`
**Status:** Done (v0.1.0)

## Context Map

### New Files

| File                   | Purpose                                                |
| ---------------------- | ------------------------------------------------------ |
| `tests/common/mod.rs`  | `ProjectFixture` builder with temp dir lifecycle       |
| `tests/integration.rs` | Cross-module integration + real-repo `#[ignore]` tests |
| `tests/sources.rs`     | Per-source integration tests with fixtures             |

### Files to Modify (inline test additions)

| File              | Changes                                      |
| ----------------- | -------------------------------------------- |
| `src/discover.rs` | Edge-case unit tests                         |
| `src/pipeline.rs` | State machine boundary tests + proptest      |
| `src/search.rs`   | Boundary tests + proptest                    |
| `src/history.rs`  | Pruning boundary + date math + special chars |
| `src/registry.rs` | Nonexistent path test                        |
| `src/app.rs`      | `base64_encode` RFC test vectors             |

### Dependencies

No new crate dependencies. `proptest` is already in `[dev-dependencies]`.

## Tasks

### Task 1: Create fixture builder

**File(s):** `tests/common/mod.rs`
**Run:** `cargo test --test integration` (will fail — no test file yet)

1. Create `tests/common/mod.rs` with:

   ```rust
   use std::path::{Path, PathBuf};
   use std::fs;

   pub struct ProjectFixture {
       root: PathBuf,
   }

   impl ProjectFixture {
       pub fn new() -> Self { /* tempdir */ }
       pub fn path(&self) -> &Path { &self.root }
       pub fn with_cargo_toml(self, content: &str) -> Self { /* write */ }
       pub fn with_justfile(self, content: &str) -> Self { /* write */ }
       pub fn with_package_json(self, content: &str) -> Self { /* write */ }
       pub fn with_makefile(self, content: &str) -> Self { /* write */ }
       pub fn with_mise_toml(self, content: &str) -> Self { /* write */ }
       pub fn with_nu_script(self, name: &str, content: &str) -> Self
       pub fn with_xtask_main(self, content: &str) -> Self { /* write */ }
   }

   impl Drop for ProjectFixture {
       fn drop(&mut self) { let _ = fs::remove_dir_all(&self.root); }
   }
   ```

2. Each builder method writes to `self.root/<path>` and returns `self`.

3. `new()` creates a unique temp dir via
   `std::env::temp_dir().join(format!("xtui-fixture-{}", pid-counter))`.

4. Commit: `test(xtui): add ProjectFixture builder for integration tests`

---

### Task 2: Create integration test file

**File(s):** `tests/integration.rs`
**Run:** `cargo test --test integration`
**Depends on:** Task 1

1. Write `all_sources_discover_fixture`:

   ```rust
   #[test]
   fn all_sources_discover_fixture() {
       let fix = ProjectFixture::new()
           .with_cargo_toml("[package]\nname=\"t\"\nversion=\"0.1.0\"")
           .with_justfile("build:\n  echo ok\ntest:\n  echo ok")
           .with_package_json(r#"{"scripts":{"dev":"echo","lint":"echo"}}"#)
           .with_makefile("build:\n\techo\nclean:\n\trm")
           .with_mise_toml("[tasks.ci]\nrun = \"echo\"")
           .with_nu_script("check", "# check");
       let sources = xtui::source::all_sources();
       let tabs: Vec<(&str, Vec<_>)> = sources.iter()
           .filter_map(|s| {
               let cmds = s.discover(fix.path()).unwrap();
               if cmds.is_empty() { None }
               else { Some((s.name(), cmds)) }
           })
           .collect();
       // cargo, just, npm, make, mise, nu — 6 sources
       assert_eq!(tabs.len(), 6);
   }
   ```

2. Write `mixed_project_some_sources_missing`:

   ```rust
   #[test]
   fn mixed_project_some_sources_missing() {
       let fix = ProjectFixture::new()
           .with_cargo_toml("[package]\nname=\"t\"\nversion=\"0.1.0\"")
           .with_justfile("build:\n  echo ok");
       let sources = xtui::source::all_sources();
       let count = sources.iter()
           .filter(|s| !s.discover(fix.path()).unwrap().is_empty())
           .count();
       assert_eq!(count, 2); // cargo + just
   }
   ```

3. Write `empty_project_no_tabs`:

   ```rust
   #[test]
   fn empty_project_no_tabs() {
       let fix = ProjectFixture::new(); // no markers
       let sources = xtui::source::all_sources();
       let count = sources.iter()
           .filter(|s| !s.discover(fix.path()).unwrap().is_empty())
           .count();
       assert_eq!(count, 0);
   }
   ```

4. Write `#[ignore]` real-repo tests:

   ```rust
   #[test]
   #[ignore]
   fn real_repo_xtui() {
       let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
       let sources = xtui::source::all_sources();
       let names: Vec<&str> = sources.iter()
           .filter(|s| !s.discover(&root).unwrap().is_empty())
           .map(|s| s.name())
           .collect();
       assert!(names.contains(&"cargo"));
       assert!(names.contains(&"xtask"));
   }

   #[test]
   #[ignore]
   fn real_repo_minibox() {
       let minibox = PathBuf::from(env!("HOME")).join("dev/minibox");
       if !minibox.exists() { return; }
       let sources = xtui::source::all_sources();
       let names: Vec<&str> = sources.iter()
           .filter(|s| !s.discover(&minibox).unwrap().is_empty())
           .map(|s| s.name())
           .collect();
       assert!(names.contains(&"xtask"));
   }
   ```

5. Commit: `test(xtui): add integration tests with fixtures and real repos`

---

### Task 3: Create source-specific integration tests

**File(s):** `tests/sources.rs`
**Run:** `cargo test --test sources`
**Depends on:** Task 1

1. Write `just_source_with_descriptions`:
   - Fixture with `Justfile` containing `recipe: # description` lines
   - Assert descriptions are parsed

2. Write `cargo_source_with_bin_targets`:
   - Fixture with `[[bin]]` entries in Cargo.toml
   - Assert `run --bin <name>` commands discovered

3. Write `xtask_source_nested_subcommands`:
   - Fixture with xtask/src/main.rs containing nested dispatch
   - Assert subcommands like `"test unit"` discovered

4. Commit: `test(xtui): add source-specific integration tests`

---

### Task 4: Add inline edge-case tests to `discover.rs`

**File(s):** `src/discover.rs`
**Run:** `cargo test -- discover`

1. `test_parse_source_empty_input` — empty string returns empty vec
2. `test_parse_source_no_match_block` — fn main with no match
3. `test_descriptions_with_escaped_quotes` — description containing `\"`
4. `test_descriptions_multiword_command` — `"test unit"` key in help

5. Commit: `test(xtui): add discover edge-case tests`

---

### Task 5: Add inline edge-case tests to `pipeline.rs`

**File(s):** `src/pipeline.rs`
**Run:** `cargo test -- pipeline`

1. `test_advance_on_pending_is_noop` — advance on Pending does nothing
2. `test_advance_on_done_is_noop` — advance on Done does nothing
3. `test_advance_on_failed_is_noop` — advance on Failed does nothing
4. `test_single_step_success` — one step, advance(0) -> Done
5. `test_single_step_failure` — one step, advance(1) -> Failed(0, 1)
6. Proptest: `advance` with arbitrary exit codes never panics

7. Commit: `test(xtui): add pipeline state machine edge-case tests`

---

### Task 6: Add inline edge-case tests to `search.rs`

**File(s):** `src/search.rs`
**Run:** `cargo test -- search`

1. `test_empty_query_matches_all` — empty query matches every line
2. `test_empty_lines` — search over empty vec returns no matches
3. `test_single_line_match` — one line matching, next wraps to itself
4. Proptest: `find_matches` never panics on arbitrary (query, lines)

5. Commit: `test(xtui): add search boundary tests and proptest`

---

### Task 7: Add inline edge-case tests to `history.rs`

**File(s):** `src/history.rs`
**Run:** `cargo test -- history`

1. `test_prune_at_exactly_100` — 100 files, no pruning
2. `test_prune_at_101` — 101 files, prunes 1
3. `test_save_entry_special_chars` — project name with spaces/unicode
4. `test_days_to_ymd_epoch` — day 0 = 1970-01-01
5. `test_days_to_ymd_leap` — 2024-02-29
6. `test_days_to_ymd_today` — 2026-06-08

7. Commit: `test(xtui): add history boundary and date math tests`

---

### Task 8: Add inline edge-case tests to `app.rs` and `registry.rs`

**File(s):** `src/app.rs`, `src/registry.rs`
**Run:** `cargo test`

1. `app.rs` — `base64_encode` RFC 4648 test vectors:
   - `""` -> `""`
   - `"f"` -> `"Zg=="`
   - `"fo"` -> `"Zm8="`
   - `"foo"` -> `"Zm9v"`
   - `"foob"` -> `"Zm9vYg=="`
   - `"fooba"` -> `"Zm9vYmE="`
   - `"foobar"` -> `"Zm9vYmFy"`

2. `registry.rs` — `test_scan_nonexistent_path`:
   - `scan_directory(Path::new("/nonexistent"))` returns empty vec

3. Commit: `test(xtui): add base64 vectors and registry edge case`

---

## Dependency Order

```
Task 1 (fixture builder)
  |
  +---> Task 2 (integration tests)
  |
  +---> Task 3 (source integration tests)
  |
Task 4-8 (inline tests) — all independent, parallelizable

Suggested serial order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8
```

Tasks 4-8 have no dependencies on each other or on Tasks 1-3.

## Verification

After all tasks:

```
cargo test                    # all fixture + inline tests pass
cargo test -- --ignored       # real-repo tests pass locally
cargo clippy                  # zero warnings
```
