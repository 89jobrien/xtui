use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

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
    let config: CopiesConfig = toml::from_str(&config_str)
        .unwrap_or_else(|e| panic!("failed to parse copies.toml: {e}"));

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
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
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
        // Outside fenced blocks: protect bare <word> / <word arg> sequences that are
        // not already inside backtick spans.
        out.push_str(&protect_angle_brackets_in_line(line));
        out.push('\n');
    }
    // Preserve trailing newline behaviour of the original
    if !content.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn protect_angle_brackets_in_line(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut in_backtick = false;
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
                    && inner
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '/' || c == '.' || c == ' ');
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
    result
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
        _ => {
            eprintln!("Available commands:");
            eprintln!("    check        Run cargo check");
            eprintln!("    test         Run cargo test");
            eprintln!("    clippy       Run cargo clippy");
            eprintln!("    install      Install xtui to ~/.cargo/bin");
            eprintln!("    docs         Copy sources and build the mdbook → xbook/dist/");
            eprintln!("    book         Copy sources and serve the mdbook (opens browser)");
        }
    }
}
