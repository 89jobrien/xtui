#!/usr/bin/env nu

# SessionStart hook entry point.
# Reads Claude hook JSON from stdin, extracts session_id, runs dev-state.
# Usage: called by .claude/settings.json SessionStart hook.

let ctx = (open --raw /dev/stdin | from json)
let session_id = ($ctx | get -o session_id | default "")

with-env {CLAUDE_SESSION_ID: $session_id} {
    nu scripts/dev-state.nu
}
