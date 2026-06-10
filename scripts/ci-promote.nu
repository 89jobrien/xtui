#!/usr/bin/env nu

# CI promote: FF-merge develop → main, rail patch bump, push commit + tags.
#
# Usage: nu scripts/ci-promote.nu [--remote <name>] [--dry-run]
#
# Expects to run with the repo already checked out.
# On success: main is ahead by one chore(release) commit, tag vX.Y.Z pushed.
# On failure: exits non-zero, main is unchanged.

def main [
    --remote: string = "origin",   # git remote name
    --repo-root: string = "",      # repo root (default: cwd); used by tests
    --dry-run (-n),                # print plan, make no changes
] {
    if ($repo_root | is-not-empty) {
        cd $repo_root
    }

    let branch = (git branch --show-current | str trim)

    # Accept running from develop or a detached HEAD (CI checkout)
    git fetch $remote develop main

    # Verify develop is fast-forwardable into main
    let develop_sha = (git rev-parse $"($remote)/develop" | str trim)
    let main_sha    = (git rev-parse $"($remote)/main"    | str trim)
    let base_sha    = (git merge-base $develop_sha $main_sha | str trim)

    if $base_sha != $main_sha {
        print $"Promote blocked: ($remote)/main is not an ancestor of ($remote)/develop — branches have diverged."
        print $"  main:    ($main_sha | str substring 0..7)"
        print $"  develop: ($develop_sha | str substring 0..7)"
        print $"  base:    ($base_sha | str substring 0..7)"
        exit 1
    }

    if $develop_sha == $main_sha {
        print "Nothing to promote: develop and main are already in sync."
        exit 0
    }

    if $dry_run {
        print $"[dry-run] git checkout main"
        print $"[dry-run] git merge --ff-only ($remote)/develop"
        print $"[dry-run] cargo rail release run xtui --bump=patch --skip-publish --yes"
        print $"[dry-run] git push ($remote) main"
        print $"[dry-run] git push ($remote) --tags"
        exit 0
    }

    git checkout main
    git merge --ff-only $"($remote)/develop"

    cargo rail release run xtui --bump=patch --skip-publish --yes

    git push $remote main
    git push $remote --tags

    let new_version = (
        open --raw Cargo.toml
        | lines
        | where { |l| $l | str starts-with "version = \"" }
        | first
        | str replace --regex '^version = "(.+)"$' '$1'
    )
    print $"Promoted develop → main: xtui v($new_version)"
}
