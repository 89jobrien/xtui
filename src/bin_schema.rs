use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Subcommand entry as stored in the cache.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BinSubcommand {
    pub name: String,
    pub description: Option<String>,
}

/// On-disk schema for one binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinSchema {
    pub binary: String,
    /// Unix mtime (secs) of the binary at cache-write time.
    pub bin_mtime: u64,
    pub schema_version: u32,
    /// Empty means no subcommands found — binary is listed bare.
    pub subcommands: Vec<BinSubcommand>,
}

const SCHEMA_VERSION: u32 = 1;
/// Maximum wall-clock time allowed for a `--help` probe.
const HELP_TIMEOUT: Duration = Duration::from_millis(500);

/// Binaries known to be unsafe or useless to probe with `--help`.
const SKIP_HELP_PROBE: &[&str] = &[
    "alacritty",         // GUI app — launches a window
    "sccache",           // compilation cache daemon
    "searchboxd",        // background search daemon
    "trunk",             // Wasm dev server
    "nu_plugin_query",   // Nushell plugin — speaks msgpack, not CLI
    "rustup",            // slow toolchain proxy (~300ms)
    "samply",            // profiler — may require elevated permissions
    "hotpath",           // profiler
    "hotpath-samply",    // profiler
    "hotpath-crashtest", // profiler
    "wgslfmt",           // reads stdin by default
    "kani",              // formal verification tool — slow init
    "cargo-kani",        // formal verification tool — slow init
];

/// Returns `~/.config/xtui/bin-schema/`.
pub fn schema_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("xtui")
        .join("bin-schema")
}

/// Returns the cached schema for `binary_name` if it exists and is still valid
/// (i.e. the binary's mtime matches the cached mtime). Returns `None` on any
/// miss, stale entry, or I/O error.
pub fn load_cached(dir: &std::path::Path, binary_name: &str) -> Option<BinSchema> {
    let file = dir.join(format!("{binary_name}.json"));
    let contents = fs::read_to_string(&file).ok()?;
    let schema: BinSchema = serde_json::from_str(&contents).ok()?;
    if schema.schema_version != SCHEMA_VERSION {
        return None;
    }
    let current_mtime = mtime_of_binary(binary_name)?;
    if schema.bin_mtime != current_mtime {
        return None;
    }
    Some(schema)
}

/// Writes `schema` to `<dir>/<binary>.json`, creating the directory if needed.
pub fn save_schema(dir: &std::path::Path, schema: &BinSchema) -> Result<()> {
    fs::create_dir_all(dir)?;
    let file = dir.join(format!("{}.json", schema.binary));
    let json = serde_json::to_string_pretty(schema)?;
    fs::write(file, json)?;
    Ok(())
}

/// Probes `binary_name` (looked up in `~/.cargo/bin/`) by running it with
/// `--help`, parses the output for subcommand lines, and caches the result.
///
/// Returns the schema (which may have an empty `subcommands` list if the binary
/// does not expose subcommands or is in the skip list).
pub fn probe_and_cache(dir: &std::path::Path, binary_name: &str) -> BinSchema {
    let bin_mtime = mtime_of_binary(binary_name).unwrap_or(0);
    let subcommands = if SKIP_HELP_PROBE.contains(&binary_name) {
        vec![]
    } else {
        probe_subcommands(binary_name).unwrap_or_default()
    };
    let schema = BinSchema {
        binary: binary_name.to_string(),
        bin_mtime,
        schema_version: SCHEMA_VERSION,
        subcommands,
    };
    // Best-effort cache write — failure is non-fatal.
    let _ = save_schema(dir, &schema);
    schema
}

/// Returns the cached schema if fresh, otherwise probes and caches.
pub fn get_schema(dir: &std::path::Path, binary_name: &str) -> BinSchema {
    if let Some(cached) = load_cached(dir, binary_name) {
        return cached;
    }
    probe_and_cache(dir, binary_name)
}

// ── Internals ────────────────────────────────────────────────────────────────

fn mtime_of_binary(name: &str) -> Option<u64> {
    let path = dirs::home_dir()?.join(".cargo").join("bin").join(name);
    let meta = fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    Some(mtime)
}

/// Runs `<binary> --help` with a 500ms timeout and parses subcommand lines.
///
/// Returns `None` on timeout, exec failure, or non-UTF-8 output.
fn probe_subcommands(binary_name: &str) -> Option<Vec<BinSubcommand>> {
    let bin_path = dirs::home_dir()?
        .join(".cargo")
        .join("bin")
        .join(binary_name);

    // Spawn with a deadline. We use a thread + channel rather than tokio so
    // this stays synchronous (called from CommandSource::discover).
    let bin_path_clone = bin_path.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = Command::new(&bin_path_clone).arg("--help").output();
        let _ = tx.send(result);
    });

    let output = rx.recv_timeout(HELP_TIMEOUT).ok()?.ok()?;
    // Accept both exit 0 and exit 1 — many CLIs exit 1 for `--help`.
    let text = String::from_utf8_lossy(&output.stdout);
    let stderr_text = String::from_utf8_lossy(&output.stderr);
    // Some tools write help to stderr
    let help_text = if text.trim().is_empty() {
        &stderr_text
    } else {
        &text
    };

    let subs = parse_help_subcommands(help_text);
    Some(subs)
}

/// Parses a `--help` output string for a subcommands block.
///
/// Recognises two common patterns:
///
/// 1. Section header (`Commands:`, `Subcommands:`, `SUBCOMMANDS:`, etc.)
///    followed by indented lines of the form `  <name>    [description]`.
///
/// 2. Clap-style output where subcommands appear under any section header and
///    are indented by 2+ spaces with at least 2 spaces before the description.
///
/// Returns an empty vec if no recognisable subcommand section is found.
// qual:allow(error_handling) reason: "Regex::new on compile-time constant patterns — panics only on programmer error, not runtime input"
pub fn parse_help_subcommands(help: &str) -> Vec<BinSubcommand> {
    // Match section headers that signal subcommands.
    let section_re =
        Regex::new(r"(?i)^\s*(commands|subcommands|available commands|subcommand)[:\s]*$").unwrap();
    // Match an indented command line: 2+ leading spaces, word-char name, optional description.
    let cmd_re = Regex::new(r"^  {1,8}(\S+)\s{2,}(.+)$").unwrap();
    let bare_re = Regex::new(r"^  {1,8}(\S+)\s*$").unwrap();

    let mut in_section = false;
    let mut results: Vec<BinSubcommand> = Vec::new();

    for line in help.lines() {
        if section_re.is_match(line) {
            in_section = true;
            continue;
        }
        // An unindented non-empty line that isn't a section header ends the block.
        if in_section && !line.starts_with(' ') && !line.trim().is_empty() {
            in_section = false;
        }
        if in_section {
            if let Some(caps) = cmd_re.captures(line) {
                let name = caps[1].to_string();
                let desc = caps[2].trim().to_string();
                // Skip meta-entries like "help", "version", or lines that look
                // like flags (start with -).
                if !name.starts_with('-') && name != "help" {
                    results.push(BinSubcommand {
                        name,
                        description: Some(desc),
                    });
                }
            } else if let Some(caps) = bare_re.captures(line) {
                let name = caps[1].to_string();
                if !name.starts_with('-') && name != "help" {
                    results.push(BinSubcommand {
                        name,
                        description: None,
                    });
                }
            }
        }
    }

    results
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn subs(help: &str) -> Vec<String> {
        parse_help_subcommands(help)
            .into_iter()
            .map(|s| s.name)
            .collect()
    }

    #[test]
    fn parses_commands_section() {
        let help = "
Usage: mytool <command>

Commands:
  build    Compile the project
  test     Run the test suite
  clean    Remove build artifacts

Options:
  --help   Print this help
";
        assert_eq!(subs(help), vec!["build", "test", "clean"]);
    }

    #[test]
    fn parses_subcommands_section() {
        let help = "
SUBCOMMANDS:
  run      Run a task
  list     List tasks
  help     Print help
";
        // "help" is filtered out
        assert_eq!(subs(help), vec!["run", "list"]);
    }

    #[test]
    fn ignores_flags_in_section() {
        let help = "
Commands:
  build       Build
  --verbose   Enable verbose output
  test        Test
";
        assert_eq!(subs(help), vec!["build", "test"]);
    }

    #[test]
    fn returns_empty_when_no_section() {
        let help = "Usage: tool [OPTIONS]\n  --help\n  --version\n";
        assert!(subs(help).is_empty());
    }

    #[test]
    fn parses_bare_subcommands_no_description() {
        let help = "
Commands:
  init
  run
";
        assert_eq!(subs(help), vec!["init", "run"]);
    }

    #[test]
    fn skip_help_probe_contains_known_binaries() {
        #[allow(clippy::const_is_empty)]
        let not_empty = !SKIP_HELP_PROBE.is_empty();
        assert!(not_empty);
        assert!(SKIP_HELP_PROBE.contains(&"alacritty"));
        assert!(SKIP_HELP_PROBE.contains(&"sccache"));
    }
}
