# CI Workflow

## Branch Pipeline

```mermaid
flowchart LR
    dev([develop])
    staging([staging])
    main([main])
    nightly_rel([nightly release])
    stable_rel([stable release])

    dev -->|"CI\nfmt · clippy · rail run"| staging
    staging -->|"Nightly\nbuild · prerelease tag"| nightly_rel
    staging -->|"promote-main\nff-merge"| main
    main -->|"release.yml\nv* tag"| stable_rel
```

## Surfaces (cargo rail)

| File kind | build | test | docs | infra |
|---|:---:|:---:|:---:|:---:|
| `*.rs` | ✓ | ✓ | | |
| `Cargo.toml` | ✓ | ✓ | | ✓ |
| `.github/**` | | | | ✓ |
| `.gitignore` | | | ✓ | |
| `CHANGELOG.md` | | | ✓ | |
| unclassified | ✓ | ✓ | | ✓ |

`bench` is detected but disabled. `cargo rail run --profile ci` runs build + test + infra surfaces.

## Stable Release

```mermaid
sequenceDiagram
    participant dev as Developer
    participant local as Local
    participant gh as GitHub Actions
    participant crates as crates.io

    dev->>local: cargo rail release run xtui --bump minor
    local->>local: bump Cargo.toml · update CHANGELOG · commit
    dev->>local: git push github main
    dev->>local: git push github v0.5.0
    gh->>gh: release.yml triggers on v* tag
    gh->>gh: build binary · generate release notes
    gh->>gh: create GitHub release
    gh->>crates: cargo publish
```
