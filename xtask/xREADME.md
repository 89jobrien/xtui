# xtask

Dev task runner for the xtui workspace. All commands run via `cargo xtask <command>`.

## Commands

### Build & Quality

| Command | Description |
| ------- | ----------- |
| `check` | `cargo check --workspace` |
| `test` | `cargo test --workspace` |
| `clippy` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `install` | Install `xtui` binary to `~/.cargo/bin` |

### Docs

| Command | Description |
| ------- | ----------- |
| `docs` | Copy sources per `xbook/copies.toml`, build mdbook → `xbook/dist/` |
| `book` | Same as `docs` but runs `mdbook serve --open` (live preview) |

### Release Pipeline

| Command | Options | Description |
| ------- | ------- | ----------- |
| `promote-staging` | `--dry-run` | FF-merge `develop` → `staging` and push to `github` remote |
| `promote-main` | `--dry-run` | FF-merge `staging` → `main` and push to `github` remote |
| `nightly` | `--dry-run` | Build release binary, upsert floating `nightly` tag, run `cargo rail release` |

`--dry-run` / `-n` on promote commands prints the git operations without executing them.
`--dry-run` on `nightly` runs `cargo rail release run xtui --check` (preview only).

## Branch Flow

```
develop  →[CI]→  staging  →[Nightly]→  main  →[v* tag]→  release.yml
```

Each promotion is automated via GitHub Actions but can be triggered manually:

```sh
cargo xtask promote-staging --dry-run   # preview
cargo xtask promote-staging             # execute

cargo xtask promote-main --dry-run
cargo xtask promote-main
```

## Book Config

Source copies are declared in [`xbook/copies.toml`](../xbook/copies.toml).
Each `[[copy]]` section maps source files (supporting globs) to a destination
subdirectory under `xbook/`. Only `xbook/SUMMARY.md`, `xbook/intro.md`,
`xbook/book.toml`, and `xbook/copies.toml` are tracked in git — everything
else is generated.
