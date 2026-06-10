#!/usr/bin/env nu

# Generate or verify .ctx/xstate.json — local dev session metadata for LLM context.
# Usage: nu scripts/xstate.nu [--verify]
# Wired into: SessionStart hook, pre-push hook, or run manually via cargo xtask xstate.

def main [--verify] {
    let out = ".ctx/xstate.json"

    if $verify {
        if not ($out | path exists) {
            print $"FAIL: ($out) does not exist"
            exit 1
        }
        let state = (open $out)
        let head = (git log -1 --format="%H" | str trim | str substring 0..7)
        let branch = (git branch --show-current | str trim)
        let commit_msg = (git log -1 --format="%s" | str trim)
        let version = (
            open --raw Cargo.toml
            | lines
            | where { |l| ($l | str starts-with "version = \"") }
            | first
            | str replace --regex '^version = "(.+)"$' '$1'
        )
        let actual_mtime = (ls $out | get 0.modified | into int)
        let mtime_delta = (($actual_mtime - $state.mtime) | math abs)

        let expected_hash = (
            $"($state.session_id)($state.workspace_root)($state.hostname)($state.username)"
            | hash sha256
        )

        mut errors = []
        if $state.commit != $head {
            $errors = ($errors | append $"commit mismatch: state=($state.commit) head=($head)")
        }
        if $state.branch != $branch {
            $errors = ($errors | append $"branch mismatch: state=($state.branch) current=($branch)")
        }
        if $state.version != $version {
            $errors = ($errors | append $"version mismatch: state=($state.version) cargo=($version)")
        }
        if $state.commit_msg != $commit_msg {
            $errors = ($errors | append $"commit_msg mismatch: state=($state.commit_msg) head=($commit_msg)")
        }
        if $mtime_delta > 2000000000 {
            $errors = ($errors | append $"mtime mismatch: file may have been manually edited \(delta: ($mtime_delta)ns\)")
        }
        if $state.session_hash != $expected_hash {
            $errors = ($errors | append $"session_hash mismatch: stored hash does not match recomputed hash — session may have changed")
        }
        if ($errors | is-not-empty) {
            for e in $errors { print $"FAIL: ($e)" }
            exit 1
        }
        print $"OK: xstate is current \(commit ($head), branch ($branch), v($version)\)"
        return
    }

    let project = (
        open --raw Cargo.toml
        | lines
        | where { |l| ($l | str starts-with "name = \"") }
        | first
        | str replace --regex '^name = "(.+)"$' '$1'
    )
    let workspace_root = ($env.PWD)
    let hostname = (sys host | get hostname)
    let username = ($env | get -o USER | default "")

    let ts = (date now | format date "%y%m%d.%H%M%S")
    let session_id = $"($project)-($nu.pid)-($ts)"

    let session_hash = (
        $"($session_id)($workspace_root)($hostname)($username)"
        | hash sha256
    )

    let crate_versions = (
        cargo metadata --no-deps --format-version 1
        | from json
        | get packages
        | each { |p| {name: $p.name, version: $p.version} }
        | reduce --fold {} { |p, acc| $acc | insert $p.name $p.version }
    )

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
        session_id: $session_id,
        session_hash: $session_hash,
        claude_session_pid: $nu.pid,
        project: $project,
        workspace_root: $workspace_root,
        hostname: $hostname,
        username: $username,
        branch: $branch,
        commit: $commit_short,
        commit_msg: $commit_msg,
        version: $version,
        dirty: ($dirty_files | is-not-empty),
        dirty_files: $dirty_files,
        crate_versions: $crate_versions,
        last_pushed_version: (
            if (".ctx/xstate.json" | path exists) {
                open .ctx/xstate.json | get -o last_pushed_version | default ""
            } else { "" }
        ),
        mtime: 0,
    }

    mkdir .ctx
    $state | to json --indent 2 | save --force $out
    let mtime = (ls $out | get 0.modified | into int)
    $state | merge {mtime: $mtime} | to json --indent 2 | save --force $out
    print $"xstate written → ($out)"
}
