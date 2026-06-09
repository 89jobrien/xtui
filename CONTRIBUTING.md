# Contributing

## Build & Test

```sh
cargo check                   # type check
cargo clippy                  # lint (fix all warnings before committing)
cargo test                    # unit + fixture integration tests
cargo test -- --ignored       # real-repo tests (requires local repos, local only)
cargo xtask check             # workspace check
cargo xtask test              # workspace test
cargo xtask clippy            # workspace clippy -D warnings
cargo xtask install           # install to ~/.cargo/bin
```

Run `cargo clippy` and `cargo test` before every commit.

## Adding a Command Source

1. Implement `CommandSource` in `src/source.rs`:

   ```rust
   pub trait CommandSource: Send + Sync {
       fn name(&self) -> &str;
       fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>>;
   }
   ```

2. Return an empty `Vec` when the source is not applicable — never return an error for
   "file not found".

3. Add the new impl to `all_sources()` in `source.rs` at the desired tab position.

4. Add a dispatch arm in `runner.rs` matching `cmd.source == "your-source-name"`.

5. Add fixture-based integration tests in `tests/sources.rs` using `ProjectFixture`.

## Testing Conventions

- Every source file has an inline `#[cfg(test)]` module with unit tests.
- `tests/common/mod.rs` provides `ProjectFixture` for building synthetic project trees.
- `tests/sources.rs` covers per-source discovery against fixtures.
- `tests/integration.rs` has cross-module tests; real-repo tests are `#[ignore]`.
- Parsers and state machines get `proptest` fuzz coverage — follow the pattern in
  `discover.rs`, `pipeline.rs`, and `search.rs`.

## Code Conventions

- `anyhow::Result<T>` for all fallible functions — no custom error types unless variants
  need matching at call sites.
- Each module owns exactly one concern. No cross-module state mutation.
- `ui.rs` is pure rendering — it must not mutate `App` state.
- Sources return `Vec<SourceCommand>`, never panic on malformed input.

## Commit Style

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(xtui): add <source> command source
fix(xtui): handle edge case in <module>
chore(xtui): update dependencies
refactor(xtui): extract <thing> into its own module
test(xtui): add coverage for <module>
docs(xtui): update ARCHITECTURE
```

Do not use `quality` as a commit type — use `chore` or `refactor` instead. The changelog
generator (`git-cliff`) drops unknown types silently.

## Known Pitfalls

- **`clippy::collapsible_if`** — nested `if` blocks that can be merged with `&&` are
  flagged. `cargo clippy` catches this before commit.
- **`clippy::const_is_empty`** — asserting `!CONST.is_empty()` directly is flagged; bind
  to a `let` first and add `#[allow(clippy::const_is_empty)]`.
- **MakeSource regex** — the target regex excludes `=` after `:` to avoid matching
  variable assignments (`VAR := value`). Keep this constraint when modifying the pattern.
