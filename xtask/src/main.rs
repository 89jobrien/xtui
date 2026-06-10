use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;
use std::collections::{HashMap, HashSet};

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("xtask must be inside workspace")
        .to_path_buf()
}

fn cargo(args: &[&str]) -> ! {
    let root = workspace_root();
    let manifest = root.join("Cargo.toml");
    // Split at "--" so --manifest-path goes before the separator
    let split = args.iter().position(|a| *a == "--");
    let (before, after) = match split {
        Some(i) => (&args[..i], &args[i..]),
        None => (args, [].as_slice()),
    };
    let status = Command::new("cargo")
        .args(before)
        .arg("--manifest-path")
        .arg(&manifest)
        .args(after)
        .status()
        .expect("failed to run cargo");
    std::process::exit(status.code().unwrap_or(1));
}

#[derive(Deserialize)]
struct CopiesConfig {
    #[serde(rename = "copy")]
    sections: Vec<CopySection>,
}

#[derive(Deserialize)]
struct CopySection {
    dest: String,
    files: Vec<String>,
}

fn copy_sources(root: &PathBuf, book_dir: &PathBuf) {
    let config_path = book_dir.join("copies.toml");
    let config_str = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", config_path.display()));
    let config: CopiesConfig =
        toml::from_str(&config_str).unwrap_or_else(|e| panic!("failed to parse copies.toml: {e}"));

    for section in &config.sections {
        let dest_dir = book_dir.join(&section.dest);
        std::fs::create_dir_all(&dest_dir)
            .unwrap_or_else(|e| panic!("failed to create {}: {e}", dest_dir.display()));

        for entry in &section.files {
            let (pattern, explicit_dest) = match entry.split_once(':') {
                Some((p, d)) => (p.to_string(), Some(d.to_string())),
                None => (entry.clone(), None),
            };

            let abs_pattern = root.join(&pattern);
            let abs_pattern_str = abs_pattern.to_string_lossy();
            let matches: Vec<_> = glob::glob(&abs_pattern_str)
                .unwrap_or_else(|e| panic!("invalid glob pattern {pattern}: {e}"))
                .filter_map(|r| r.ok())
                .collect();

            if matches.is_empty() {
                eprintln!("WARN: no matches for: {pattern}");
                continue;
            }

            for src in matches {
                let dest_name = match &explicit_dest {
                    Some(d) => d.clone(),
                    None => src
                        .file_name()
                        .expect("glob match has no filename")
                        .to_string_lossy()
                        .into_owned(),
                };
                let dest = dest_dir.join(&dest_name);
                let mut content = std::fs::read_to_string(&src)
                    .unwrap_or_else(|e| panic!("failed to read {}: {e}", src.display()));
                content = fix_bare_angle_brackets(content);
                std::fs::write(&dest, &content)
                    .unwrap_or_else(|e| panic!("failed to write {}: {e}", dest.display()));
            }
        }
    }
}

fn docs() -> ! {
    let root = workspace_root();
    let book_dir = root.join("xbook");
    copy_sources(&root, &book_dir);
    let status = Command::new("mdbook")
        .arg("build")
        .arg(&book_dir)
        .status()
        .expect("failed to run mdbook — is it installed? `cargo install mdbook`");
    std::process::exit(status.code().unwrap_or(1));
}

fn book() -> ! {
    let root = workspace_root();
    let book_dir = root.join("xbook");
    copy_sources(&root, &book_dir);
    let status = Command::new("mdbook")
        .arg("serve")
        .arg("--open")
        .arg(&book_dir)
        .status()
        .expect("failed to run mdbook — is it installed? `cargo install mdbook`");
    std::process::exit(status.code().unwrap_or(1));
}

/// Wrap bare `<word>` tokens outside fenced/inline code with backticks so mdbook
/// does not treat them as unclosed HTML tags.
fn fix_bare_angle_brackets(content: String) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_fence = false;
    // Track unclosed inline backtick spans across line boundaries.
    let mut in_backtick = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if !in_backtick && trimmed.starts_with("```") {
            in_fence = !in_fence;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        // Outside fenced blocks: protect bare <word> sequences not inside backtick spans.
        // Pass and update cross-line backtick state.
        let (protected, new_backtick_state) = protect_angle_brackets_in_line(line, in_backtick);
        in_backtick = new_backtick_state;
        out.push_str(&protected);
        out.push('\n');
    }
    // Preserve trailing newline behaviour of the original
    if !content.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn protect_angle_brackets_in_line(line: &str, initial_backtick: bool) -> (String, bool) {
    let mut result = String::with_capacity(line.len());
    let mut in_backtick = initial_backtick;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            in_backtick = !in_backtick;
            result.push('`');
            i += 1;
            continue;
        }
        if !in_backtick && chars[i] == '<' {
            // Collect up to '>'
            let start = i;
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '>' && chars[j] != '<' && chars[j] != '\n' {
                j += 1;
            }
            if j < chars.len() && chars[j] == '>' {
                let inner: String = chars[i + 1..j].iter().collect();
                // Only wrap if inner looks like a placeholder (word chars, slashes, dots, spaces)
                // and not like an HTML/XML tag we want to keep (e.g. already in a code block).
                let looks_like_placeholder = !inner.is_empty()
                    && inner.chars().all(|c| {
                        c.is_alphanumeric()
                            || c == '_'
                            || c == '-'
                            || c == '/'
                            || c == '.'
                            || c == ' '
                    });
                if looks_like_placeholder {
                    result.push('`');
                    result.push('<');
                    result.push_str(&inner);
                    result.push('>');
                    result.push('`');
                    i = j + 1;
                    continue;
                }
            }
            // Not a placeholder — emit as-is
            result.push(chars[start]);
            i = start + 1;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    (result, in_backtick)
}

fn graph() -> ! {
    let output = Command::new("cargo")
        .args(["rail", "graph"])
        .output()
        .expect("failed to run cargo rail graph — is cargo-rail installed?");
    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(output.status.code().unwrap_or(1));
    }

    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo rail graph output was not valid JSON");

    let nodes = v["nodes"].as_array().expect("missing nodes");
    let edges = v["edges"].as_array().expect("missing edges");

    // Build lookup: id → label
    let labels: HashMap<&str, &str> = nodes
        .iter()
        .map(|n| (n["id"].as_str().unwrap(), n["label"].as_str().unwrap()))
        .collect();

    // Surfaces: id → label
    let mut surfaces: Vec<(&str, &str)> = nodes
        .iter()
        .filter(|n| n["kind"] == "surface")
        .map(|n| (n["id"].as_str().unwrap(), n["label"].as_str().unwrap()))
        .collect();
    surfaces.sort_by_key(|(_, l)| *l);

    // Files owned by each crate
    let mut crate_files: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        if edge["relation"] == "owned_by" {
            let file = edge["from"].as_str().unwrap();
            let owner = edge["to"].as_str().unwrap();
            if let Some(label) = labels.get(file) {
                crate_files.entry(owner).or_default().push(label);
            }
        }
    }

    // Reasons enabling each surface, and which files map to each reason
    // reason id → set of file labels (via owned_by chain)
    let _reason_files: HashMap<&str, Vec<&str>> = {
        let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in edges {
            if edge["relation"] == "owned_by" {
                let file_id = edge["from"].as_str().unwrap();
                if let Some(label) = labels.get(file_id) {
                    map.entry(file_id).or_default().push(label);
                }
            }
        }
        map
    };

    // surface id → set of reason labels enabling it
    let mut surface_reasons: HashMap<&str, HashSet<&str>> = HashMap::new();
    for edge in edges {
        if edge["relation"] == "enables" {
            let reason_id = edge["from"].as_str().unwrap();
            let surface_id = edge["to"].as_str().unwrap();
            if let Some(label) = labels.get(reason_id) {
                surface_reasons.entry(surface_id).or_default().insert(label);
            }
        }
    }

    // Crates
    let crates: Vec<&str> = nodes
        .iter()
        .filter(|n| n["kind"] == "crate")
        .map(|n| n["id"].as_str().unwrap())
        .collect();

    for crate_id in &crates {
        let crate_label = labels[crate_id];
        println!("{crate_label}");

        // Surfaces
        println!("├── surfaces");
        for (i, (surf_id, surf_label)) in surfaces.iter().enumerate() {
            let is_last_surf = i == surfaces.len() - 1;
            let surf_prefix = if is_last_surf {
                "└──"
            } else {
                "├──"
            };
            let mut reasons: Vec<&&str> = surface_reasons
                .get(surf_id)
                .map(|s| s.iter().collect())
                .unwrap_or_default();
            reasons.sort();
            let reasons_str = if reasons.is_empty() {
                String::new()
            } else {
                format!(
                    "  [{}]",
                    reasons
                        .iter()
                        .map(|r| r
                            .trim_start_matches("FILE_KIND_")
                            .trim_start_matches("OWNER_"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            println!("│   {surf_prefix} {surf_label}{reasons_str}");
        }

        // Files
        let mut files = crate_files.get(crate_id).cloned().unwrap_or_default();
        files.sort();
        println!("└── files ({})", files.len());
        for (i, f) in files.iter().enumerate() {
            let is_last = i == files.len() - 1;
            let prefix = if is_last { "└──" } else { "├──" };
            println!("    {prefix} {f}");
        }
    }

    std::process::exit(0);
}

fn git(args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("failed to run git: {e}"))
        .success()
}

fn is_dry_run() -> bool {
    env::args().any(|a| a == "--dry-run" || a == "-n")
}

fn promote_staging() -> ! {
    let dry = is_dry_run();
    let root = workspace_root();
    std::env::set_current_dir(&root).expect("failed to cd to workspace root");
    if dry {
        eprintln!("[dry-run] git fetch github");
        eprintln!("[dry-run] git checkout staging");
        eprintln!("[dry-run] git merge --ff-only github/develop");
        eprintln!("[dry-run] git push github staging");
        std::process::exit(0);
    }
    git(&["fetch", "github"]);
    git(&["checkout", "staging"]);
    let ok = git(&["merge", "--ff-only", "github/develop"]);
    if !ok {
        eprintln!("error: fast-forward failed — staging and develop have diverged");
        std::process::exit(1);
    }
    let ok = git(&["push", "github", "staging"]);
    std::process::exit(if ok { 0 } else { 1 });
}

fn promote_main() -> ! {
    let dry = is_dry_run();
    let root = workspace_root();
    std::env::set_current_dir(&root).expect("failed to cd to workspace root");
    if dry {
        eprintln!("[dry-run] git fetch github");
        eprintln!("[dry-run] git checkout main");
        eprintln!("[dry-run] git merge --ff-only github/staging");
        eprintln!("[dry-run] git push github main");
        std::process::exit(0);
    }
    git(&["fetch", "github"]);
    git(&["checkout", "main"]);
    let ok = git(&["merge", "--ff-only", "github/staging"]);
    if !ok {
        eprintln!("error: fast-forward failed — main and staging have diverged");
        std::process::exit(1);
    }
    let ok = git(&["push", "github", "main"]);
    std::process::exit(if ok { 0 } else { 1 });
}

fn nightly() -> ! {
    let dry = is_dry_run();
    let root = workspace_root();
    if dry {
        let status = Command::new("cargo")
            .args(["rail", "release", "run", "xtui", "--check"])
            .current_dir(&root)
            .status()
            .expect("failed to run cargo rail — is it installed? `cargo install cargo-rail`");
        std::process::exit(status.code().unwrap_or(1));
    }
    let status = Command::new("cargo")
        .args(["build", "--release", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .status()
        .expect("failed to run cargo build");
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    git(&["tag", "-f", "nightly"]);
    let status = Command::new("cargo")
        .args([
            "rail",
            "release",
            "run",
            "xtui",
            "--bump",
            "prerelease",
            "--skip-publish",
            "--skip-tag",
            "-y",
        ])
        .current_dir(&root)
        .status()
        .expect("failed to run cargo rail — is it installed? `cargo install cargo-rail`");
    std::process::exit(status.code().unwrap_or(1));
}

fn main() {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("check") => cargo(&["check", "--workspace"]),
        Some("test") => cargo(&["test", "--workspace"]),
        Some("clippy") => cargo(&["clippy", "--workspace", "--", "-D", "warnings"]),
        Some("install") => {
            let root = workspace_root();
            let status = Command::new("cargo")
                .args(["install", "--path"])
                .arg(&root)
                .status()
                .expect("failed to run cargo install");
            std::process::exit(status.code().unwrap_or(1));
        }
        Some("docs") => docs(),
        Some("book") => book(),
        Some("graph") => graph(),
        Some("promote-staging") => promote_staging(),
        Some("promote-main") => promote_main(),
        Some("nightly") => nightly(),
        _ => {
            eprintln!("Available commands:");
            eprintln!("    check            Run cargo check");
            eprintln!("    test             Run cargo test");
            eprintln!("    clippy           Run cargo clippy");
            eprintln!("    install          Install xtui to ~/.cargo/bin");
            eprintln!("    docs             Copy sources and build the mdbook → xbook/dist/");
            eprintln!("    book             Copy sources and serve the mdbook (opens browser)");
            eprintln!("    graph            Render cargo rail graph as a text tree");
            eprintln!("    promote-staging  FF-merge develop → staging and push  [--dry-run]");
            eprintln!("    promote-main     FF-merge staging → main and push     [--dry-run]");
            eprintln!(
                "    nightly          Build release binary and upsert nightly tag  [--dry-run: rail --check]"
            );
        }
    }
}
