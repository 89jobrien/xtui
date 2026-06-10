---
version: 1
last_updated: 2026-06-10
next_review: 2026-06-17
---

# Recurring Mistakes Ledger

## Clippy Lints

### collapsible_if

- **Occurrences**: 1
- **First seen**: `7809d06` (2026-06-08)
- **Affected**: multiple files (resolved in same commit)
- **Pattern**: Nested `if` blocks that can be collapsed with `&&`.
- **Fix**: Merge conditions — `if a { if b { ... } }` → `if a && b { ... }`.
- **Prevention**: `cargo clippy` before commit catches this. Already in CI gate.

### const_is_empty

- **Occurrences**: 1
- **First seen**: `0ff3fb3` (2026-06-08)
- **Affected**: MakeSource / source.rs area
- **Pattern**: `clippy::const_is_empty` triggered on a const slice `.is_empty()` check.
- **Fix**: `#[allow(clippy::const_is_empty)]` at the call site.
- **Prevention**: Acknowledge this is a false positive lint; allowlist it locally if it recurs.

## Commit Type Violations

### Non-conventional type: `quality`

- **Occurrences**: 1
- **First seen**: `d41f655` (2026-06-08)
- **Pattern**: Used `quality(xtui):` prefix for a rustqual improvement pass.
- **Impact**: `git-cliff` with `filter_unconventional = true` silently drops this commit
  from changelogs. No parser rule for `quality` in `cliff.toml`.
- **Fix**: Use `chore` or `refactor` instead. Optionally add preprocessor to `cliff.toml`:
  `{ pattern = '^quality', replace = "chore" }`.

## CI / Workflow Errors

### Release tag format churn (xtui-v* vs v*)

- **Occurrences**: 3 fix commits
- **Dates**: `018a404`, `5347f40`, `85da28d` (all 2026-06-09)
- **Pattern**: Release workflow trigger tag format changed multiple times:
  initial `xtui-v*` (cargo-rail default) → `v*` (standard). Also required removing
  `tag.yml` once `rail` owned tagging. Three commits to stabilize.
- **Prevention**: Before adding a release workflow, verify the tag format cargo-rail
  emits with `cargo rail release run --dry-run` and set trigger to match from the start.

### Missing CI dependency: `just`

- **Occurrences**: 1
- **Date**: `57a44c8` (2026-06-09)
- **Pattern**: Integration tests call `just`, but the CI runner image did not have `just`
  on PATH. Tests passed locally, failed on CI.
- **Fix**: Added `just` install step to `ci.yml`.
- **Prevention**: Audit all command sources and their runtime deps when adding integration
  tests. For each `Source`, ensure its binary (`just`, `nu`, `mise`, etc.) is installed
  in the CI matrix.

### Nu syntax: missing `--raw` flag on `open` in hook

- **Occurrences**: 1
- **Date**: `26aa26f` (2026-06-09)
- **Pattern**: Pre-push hook read `Cargo.toml` with `open` instead of `open --raw`,
  causing the hook to interpret it as structured data rather than raw text.
- **Fix**: Changed to `open --raw`.
- **Prevention**: When reading arbitrary files in Nu scripts, default to `open --raw`.
  Only omit `--raw` when the file format is known and structured parsing is intended.

## Test Failures

_No patterns detected yet._

## Process Errors

_No patterns detected yet._

## Hook False Positives

_No patterns detected yet._

## Hook Ordering

### Gate check placed before refresh — always blocked on first push

- **Occurrences**: 1
- **First seen**: 2026-06-10 (`fix: refresh dev-state before gate check`)
- **Pattern**: pre-push hook ran `dev-state --verify` before `dev-state`. Any commit
  made after the last refresh caused the gate to block, requiring a manual
  `cargo xtask dev-state` before every push.
- **Fix**: Moved refresh before verify in the hook. Order is now:
  refresh → verify → amend → refresh.
- **Prevention**: When a hook both generates and validates state, always generate first.
  A gate that can only pass after manual intervention defeats the automation.

## CI / Rail Classification

### xbook/** not in infrastructure paths — triggers build+test on doc edits

- **Occurrences**: 1
- **First seen**: 2026-06-09 (session 2, doublecheck analysis)
- **Pattern**: `xbook/**` files (SUMMARY.md, book.toml, xREADME.md) are tracked by
  xtui via rail's `UNCERTAIN_FALLBACK`. Any edit to the book triggers a full
  build+test run because the paths are not in `[change-detection] infrastructure`.
- **Fix**: Added `"xbook/**"` to `infrastructure` list in `.config/rail.toml`.
- **Prevention**: When adding doc directories to a rail workspace, explicitly add
  them to `infrastructure` paths so they don't inherit build+test surfaces.

## Reverts

_No patterns detected yet._

## Gitignore / Index Drift

### .gitignore entry present but file still tracked in index

- **Occurrences**: 1
- **First seen**: 2026-06-09 (`.github/.DS_Store`)
- **Pattern**: `.gitignore` contained `.github/.DS_Store` (line 31) but the file
  was already committed to the index. Git ignores `.gitignore` for tracked files —
  the file showed as modified on every session start until manually removed.
- **Fix**: `git rm --cached .github/.DS_Store`
- **Prevention**: After adding a `.gitignore` entry for a file that was previously
  committed, immediately follow with `git rm --cached <path>` to remove it from
  the index. The pattern is: add gitignore entry → verify with `git ls-files <path>`
  → if tracked, `git rm --cached` it.

## Feature Implementation

### Incomplete initial implementation requiring immediate fix

- **Occurrences**: 1
- **Date**: `80a0d34` follows `a2850e9` (2026-06-09)
- **Pattern**: Dep view feature shipped (`a2850e9 feat: render dep view panel in ui`)
  then immediately fixed in the next commit (`80a0d34 fix: dep view shows normal deps
  only; renders full-screen`). Dev/build deps leaked into the view and full-screen
  rendering was wrong.
- **Prevention**: Before marking a feature task done, run the binary and visually
  verify the rendered output. Dep graph filtering and layout should be validated with
  a real project that has mixed dep types.
