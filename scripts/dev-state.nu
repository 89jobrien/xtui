#!/usr/bin/env nu

# Generate or verify .ctx/dev-state.json — local dev session metadata for LLM context.
# Usage: nu scripts/dev-state.nu [--verify]
# Wired into: SessionStart hook, pre-push hook, or run manually via cargo xtask dev-state.

def main [--verify] {
    let out = ".ctx/dev-state.json"

    if $verify {
        if not ($out | path exists) {
            print $"FAIL: ($out) does not exist"
            exit 1
        }
        let state = (open $out)
        let head = (git log -1 --format="%H" | str trim | str substring 0..7)
        let branch = (git branch --show-current | str trim)
        let version = (
            open --raw Cargo.toml
            | lines
            | where { |l| ($l | str starts-with "version = \"") }
            | first
            | str replace --regex '^version = "(.+)"$' '$1'
        )
        mut errors = []
        if $state.commit != $head { $errors = ($errors | append $"commit mismatch: state=($state.commit) head=($head)") }
        if $state.branch != $branch { $errors = ($errors | append $"branch mismatch: state=($state.branch) current=($branch)") }
        if $state.version != $version { $errors = ($errors | append $"version mismatch: state=($state.version) cargo=($version)") }
        if ($errors | is-not-empty) {
            for e in $errors { print $"FAIL: ($e)" }
            exit 1
        }
        print $"OK: dev-state is current \(commit ($head), branch ($branch), v($version)\)"
        return
    }

    let commit = (git log -1 --format="%H" | str trim)
    let commit_short = ($commit | str substring 0..7)
    let commit_msg = (git log -1 --format="%s" | str trim)
    let branch = (git branch --show-current | str trim)

    let version = (
        open --raw Cargo.toml
        | lines
        | where { |l| ($l | str starts-with "version = \"") }
        | first
        | str replace --regex '^version = "(.+)"$' '$1'
    )

    let dirty_files = (
        git status --porcelain
        | lines
        | where { |l| ($l | str trim | is-not-empty) }
        | each { |l| $l | str trim | split row " " | last }
    )

    let state = {
        timestamp: (date now | format date "%Y-%m-%dT%H:%M:%S%z"),
        claude_session_pid: $nu.pid,
        branch: $branch,
        commit: $commit_short,
        commit_msg: $commit_msg,
        version: $version,
        dirty: ($dirty_files | is-not-empty),
        dirty_files: $dirty_files,
    }

    mkdir .ctx
    $state | to json --indent 2 | save --force $out
    print $"dev-state written → ($out)"
}
