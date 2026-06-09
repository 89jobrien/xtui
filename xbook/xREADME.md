# xbook

mdBook documentation site for xtui. Sources are copied from their canonical
locations in the repo — nothing in `xbook/` is edited directly except the
four tracked files listed below.

## Tracked Files

| File | Purpose |
| ---- | ------- |
| `xbook/book.toml` | mdBook config (`src = "."`, `build-dir = "dist"`) |
| `xbook/copies.toml` | Declares source files to copy into `xbook/` (supports globs) |
| `xbook/SUMMARY.md` | mdBook table of contents |
| `xbook/intro.md` | Book introduction page |

Everything else under `xbook/` is generated and gitignored.

## Build Commands

| Command | Description |
| ------- | ----------- |
| `cargo xtask docs` | Copy sources, build → `xbook/dist/` |
| `cargo xtask book` | Copy sources, build, serve with live reload (`mdbook serve --open`) |

## copies.toml Format

Each `[[copy]]` section copies one or more files into a destination subdirectory
under `xbook/`. Entries support glob patterns (no `:dest` suffix) or explicit
renames (`src:dest`).

```toml
[[copy]]
dest = "guide"
files = [
    "README.md:readme.md",       # explicit rename
    "docs/plans/*.md",           # glob — uses filename as dest
]
```

Source paths are relative to the workspace root. Missing sources emit a warning
and are skipped; they do not fail the build.

## Book Structure

| Section | Source |
| ------- | ------ |
| `guide/` | Root docs: README, CLAUDE.md, CONTRIBUTING.md, DEPLOYMENT.md, CHANGELOG.md, AGENTS.md, xtask, xbook |
| `architecture/` | `ARCHITECTURE.md` |
| `plans/` | `docs/plans/*.md` |
| `context/memory-bank/` | `.ctx/memory-bank/*.md` |
| `knowledge/summary/` | `.kgx/wiki/summary/*.md` |
| `knowledge/entity/` | `.kgx/wiki/entity/*.md` |

## Prerequisites

```sh
cargo install mdbook
```
